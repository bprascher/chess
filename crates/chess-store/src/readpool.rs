//! DD-ST-003 `ReadPool`: a small pool of read-only connections used from `spawn_blocking`
//! (architecture 4.8, R-11). `Mutex<Vec<Connection>>` plus a condvar: a caller waits when every
//! connection is in use. The pool holds no lock while a query runs.

use std::sync::{Condvar, Mutex, PoisonError};

use rusqlite::Connection;

use crate::error::StoreError;

pub(crate) struct ReadPool {
    idle: Mutex<Vec<Connection>>,
    available: Condvar,
}

/// Returns the connection to the pool when dropped, also during a panic.
struct Lease<'a> {
    pool: &'a ReadPool,
    conn: Option<Connection>,
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool
                .idle
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(conn);
            self.pool.available.notify_one();
        }
    }
}

impl ReadPool {
    pub(crate) fn new(connections: Vec<Connection>) -> ReadPool {
        ReadPool {
            idle: Mutex::new(connections),
            available: Condvar::new(),
        }
    }

    /// Blocking. Never call it from an async worker thread: `Store` uses `spawn_blocking`.
    pub(crate) fn with<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let mut idle = self.idle.lock().unwrap_or_else(PoisonError::into_inner);
        let conn = loop {
            if let Some(conn) = idle.pop() {
                break conn;
            }
            idle = self
                .available
                .wait(idle)
                .unwrap_or_else(PoisonError::into_inner);
        };
        drop(idle);
        let lease = Lease {
            pool: self,
            conn: Some(conn),
        };
        match lease.conn.as_ref() {
            Some(conn) => f(conn),
            None => Err(StoreError::Internal),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;

    use super::*;

    fn pool(n: usize) -> Arc<ReadPool> {
        let conns = (0..n)
            .map(|_| Connection::open_in_memory().unwrap())
            .collect();
        Arc::new(ReadPool::new(conns))
    }

    #[test]
    fn a_second_caller_waits_for_the_only_connection_and_gets_it_back() {
        let pool = pool(1);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (started_tx, started_rx) = mpsc::channel();
        let holder = {
            let pool = Arc::clone(&pool);
            std::thread::spawn(move || {
                pool.with(|_| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
            })
        };
        entered_rx.recv().unwrap();
        let waiter = {
            let pool = Arc::clone(&pool);
            std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                pool.with(|conn| Ok(conn.query_row("SELECT 7", [], |r| r.get::<_, i64>(0))?))
            })
        };
        started_rx.recv().unwrap();
        release_tx.send(()).unwrap();
        holder.join().unwrap().unwrap();
        assert_eq!(waiter.join().unwrap().unwrap(), 7);
    }

    #[test]
    fn a_panicking_reader_returns_its_connection_to_the_pool() {
        let pool = pool(1);
        let p = Arc::clone(&pool);
        let crashed = std::thread::spawn(move || {
            let _ = p.with(|_| -> Result<(), StoreError> { panic!("reader panicked") });
        })
        .join();
        assert!(crashed.is_err());
        // The only connection is back: this would block forever otherwise.
        assert_eq!(pool.with(|_| Ok(1)).unwrap(), 1);
    }
}
