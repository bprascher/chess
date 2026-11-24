//! DD-ST-002 `StoreThread`: the single OS thread that owns the write connection and runs every
//! write transaction (D-13, architecture 4.8). It knows no SQL: callers send closures over the
//! connection (SDD 3.8 layering rule). Part D3 adds the apply backlog and its retry timer on the
//! same thread (`backlog.rs` is the pure scheduling; the attempt itself is the `Applier` that
//! the store hands in).
//!
//! Failure behaviour (SDD section 4): a panic in a command is caught and returned as `Internal`
//! (the closure's reply sender is dropped during the unwind); the thread then reopens the
//! connection once, and after a second panic every call returns `Unavailable`. A caller that
//! stops waiting never leaves a half transaction: the closure runs to completion or its
//! transaction rolls back when it is dropped.
//!
//! The loop (part D3): run a due backlog pass, answer the `sync_backlog` waiters, then wait for
//! the next command. While the backlog or a waiter exists the wait is at most `POLL_MS`, so a
//! monotonic clock that a test moves by hand is noticed within that time; with nothing pending
//! the thread blocks on the channel.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use chess_domain::GameId;
use rusqlite::Connection;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::api::{ApplyOutcome, BacklogStatus, ParkCause, ParkedGame};
use crate::backlog::{Attempt, Backlog, Flush, MAX_BATCHES_PER_GAME, MAX_PENDING_BATCHES};
use crate::error::{StoreError, UnavailableCause};
use crate::journal::JournalRecord;

type Job = Box<dyn FnOnce(Result<&mut Connection, StoreError>) + Send>;
type Reopen = Box<dyn Fn() -> Result<Connection, StoreError> + Send>;

/// One attempt to apply a batch (`repo::apply::apply_records`, handed in by `Store`).
pub(crate) type Applier =
    Arc<dyn Fn(&mut Connection, GameId, &[JournalRecord]) -> Result<(), StoreError> + Send + Sync>;

/// Monotonic milliseconds. The default counts from the start of the thread; a test injects its
/// own (`StoreOptions::with_monotonic_clock`, feature `test-support`).
pub type MonotonicClock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// The longest the thread sleeps while the backlog or a `sync_backlog` call is pending.
const POLL_MS: u64 = 100;

pub(crate) fn instant_clock() -> MonotonicClock {
    let origin = Instant::now();
    Arc::new(move || u64::try_from(origin.elapsed().as_millis()).unwrap_or(u64::MAX))
}

enum Command {
    Run(Job),
    /// A job that runs after the queued batches of these games were applied (D3-09).
    RunFlushed {
        games: Vec<GameId>,
        job: Job,
    },
    Unpark {
        game: GameId,
        reply: oneshot::Sender<Option<usize>>,
    },
    Parked {
        reply: oneshot::Sender<Vec<ParkedGame>>,
    },
    #[cfg(test)]
    Waiters {
        reply: oneshot::Sender<usize>,
    },
    Apply {
        game: GameId,
        records: Vec<JournalRecord>,
        reply: oneshot::Sender<Result<ApplyOutcome, StoreError>>,
    },
    Sync {
        timeout: Duration,
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
    Status {
        reply: oneshot::Sender<BacklogStatus>,
    },
}

pub(crate) struct StoreThread {
    sender: Mutex<Option<mpsc::Sender<Command>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

fn unavailable() -> StoreError {
    StoreError::Unavailable {
        cause: UnavailableCause::Io(std::io::ErrorKind::Other),
    }
}

/// `Busy` and `Unavailable` are the errors that a later attempt can cure (SDD 3.12, architecture
/// 4.8: busy, locked or unavailable). Everything else is a fault of the batch or the file, and
/// parks the game. `Internal` is in that group (unknown stays permanent), but `error::map_sqlite`
/// no longer sends the SQLite codes that a retry cures there (out of memory, interrupt, lock
/// protocol, schema change: they are `Busy`; FIND-D3R-09).
fn classify(result: &Result<(), StoreError>) -> Attempt {
    match result {
        Ok(()) => Attempt::Done,
        Err(StoreError::Busy | StoreError::Unavailable { .. }) => Attempt::Transient,
        Err(error) => Attempt::Permanent(park_cause(error)),
    }
}

fn park_cause(error: &StoreError) -> ParkCause {
    match error {
        StoreError::InvariantViolated => ParkCause::InvariantViolated,
        StoreError::NotFound { .. } => ParkCause::NotFound,
        StoreError::Corrupt => ParkCause::Corrupt,
        StoreError::Internal => ParkCause::Internal,
        _ => ParkCause::Other,
    }
}

impl StoreThread {
    /// Starts the thread with its connection. `reopen` builds a replacement after a panic;
    /// `applier` runs one batch; `clock` times the backlog.
    pub(crate) fn spawn(
        conn: Connection,
        reopen: Reopen,
        applier: Applier,
        clock: MonotonicClock,
    ) -> Result<StoreThread, StoreError> {
        let (sender, receiver) = mpsc::channel::<Command>();
        let handle = std::thread::Builder::new()
            .name("chess-store-writer".to_owned())
            .spawn(move || {
                Worker {
                    slot: Slot {
                        conn: Some(conn),
                        restarts_left: 1,
                        reopen,
                    },
                    backlog: Backlog::new(),
                    waiters: Vec::new(),
                    applier,
                    clock,
                }
                .run(&receiver);
            })
            .map_err(|e| crate::error::map_io(&e))?;
        Ok(StoreThread {
            sender: Mutex::new(Some(sender)),
            handle: Mutex::new(Some(handle)),
        })
    }

    fn send(&self, command: Command) -> Result<(), StoreError> {
        let guard = self.sender.lock().unwrap_or_else(|p| p.into_inner());
        match guard.as_ref() {
            Some(sender) if sender.send(command).is_ok() => Ok(()),
            _ => Err(unavailable()),
        }
    }

    /// Runs `f` on the store thread and returns its result.
    pub(crate) async fn run<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let (reply, answer) = oneshot::channel();
        let job: Job = Box::new(move |conn| {
            let outcome = match conn {
                Ok(conn) => f(conn),
                Err(error) => Err(error),
            };
            // The caller may have stopped waiting; the work is done either way.
            let _ = reply.send(outcome);
        });
        self.send(Command::Run(job))?;
        answer.await.unwrap_or(Err(StoreError::Internal))
    }

    /// Runs `f` on the store thread after the queued batches of `games` were applied, in order
    /// (D3-09: a live finalize, abort, void or reset must not run ahead of earlier batches of its
    /// game). `Busy` when a batch cannot be applied yet, `GameParked` for a parked game; `f` does
    /// not run then.
    pub(crate) async fn run_flushed<T, F>(&self, games: Vec<GameId>, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let (reply, answer) = oneshot::channel();
        let job: Job = Box::new(move |conn| {
            let outcome = match conn {
                Ok(conn) => f(conn),
                Err(error) => Err(error),
            };
            let _ = reply.send(outcome);
        });
        self.send(Command::RunFlushed { games, job })?;
        answer.await.unwrap_or(Err(StoreError::Internal))
    }

    /// Releases a parked game and drops its held queue; the number of batches dropped, `None`
    /// when it was not parked.
    pub(crate) async fn unpark(&self, game: GameId) -> Result<Option<usize>, StoreError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Unpark { game, reply })?;
        answer.await.map_err(|_| StoreError::Internal)
    }

    pub(crate) async fn parked(&self) -> Result<Vec<ParkedGame>, StoreError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Parked { reply })?;
        answer.await.map_err(|_| StoreError::Internal)
    }

    /// The number of `sync_backlog` calls the thread still holds (test hook, FIND-D3R-08).
    #[cfg(test)]
    pub(crate) async fn waiters(&self) -> Result<usize, StoreError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Waiters { reply })?;
        answer.await.map_err(|_| StoreError::Internal)
    }

    /// Applies a batch now, or queues it (`ApplyOutcome::Queued`) when the database is busy or
    /// unavailable or when an earlier batch of the game still waits. An error that a retry
    /// cannot cure is returned, parks the game and keeps the batch at the head of its queue
    /// (D3 ruling 2026-11-24); later batches of a parked game are refused (`GameParked`), and
    /// a game holds at most `MAX_BATCHES_PER_GAME` batches and the backlog `MAX_PENDING_BATCHES`
    /// (`BacklogFull`; a game that has a queue is parked with `ParkCause::QueueFull`).
    pub(crate) async fn apply(
        &self,
        game: GameId,
        records: Vec<JournalRecord>,
    ) -> Result<ApplyOutcome, StoreError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Apply {
            game,
            records,
            reply,
        })?;
        answer.await.unwrap_or(Err(StoreError::Internal))
    }

    /// Waits until the backlog is empty (`Ok`), `GameParked` as soon as a game is parked (the
    /// barrier cannot be met), or `Busy` after `timeout`.
    pub(crate) async fn sync(&self, timeout: Duration) -> Result<(), StoreError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Sync { timeout, reply })?;
        answer.await.unwrap_or(Err(StoreError::Internal))
    }

    pub(crate) async fn status(&self) -> Result<BacklogStatus, StoreError> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Status { reply })?;
        answer.await.map_err(|_| StoreError::Internal)
    }
}

impl Drop for StoreThread {
    fn drop(&mut self) {
        // Closing the channel ends the loop; joining makes the connection (and its WAL
        // checkpoint) close before the owner continues.
        self.sender.lock().unwrap_or_else(|p| p.into_inner()).take();
        if let Some(handle) = self.handle.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = handle.join();
        }
    }
}

/// The write connection and the rule for a panic (reopen once, then unavailable).
struct Slot {
    conn: Option<Connection>,
    restarts_left: u8,
    reopen: Reopen,
}

impl Slot {
    /// Runs `f` with the connection, catching a panic. `None` means `f` panicked.
    fn run<R>(&mut self, f: impl FnOnce(Result<&mut Connection, StoreError>) -> R) -> Option<R> {
        let outcome = match self.conn.as_mut() {
            Some(conn) => catch_unwind(AssertUnwindSafe(|| f(Ok(conn)))),
            None => catch_unwind(AssertUnwindSafe(|| f(Err(unavailable())))),
        };
        match outcome {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::error!("a store command panicked");
                self.conn = None;
                if self.restarts_left > 0 {
                    self.restarts_left -= 1;
                    self.conn = (self.reopen)().ok();
                }
                None
            }
        }
    }
}

struct Waiter {
    deadline_ms: u64,
    /// The next time a waiter forces a pass of its own (FIND-D3R-03): the timer of the backlog
    /// can be seconds away after a long busy spell, the caller's window is not.
    next_try_ms: u64,
    every_ms: u64,
    reply: oneshot::Sender<Result<(), StoreError>>,
}

struct Worker {
    slot: Slot,
    backlog: Backlog<Vec<JournalRecord>>,
    waiters: Vec<Waiter>,
    applier: Applier,
    clock: MonotonicClock,
}

impl Worker {
    fn now(&self) -> u64 {
        (self.clock)()
    }

    /// One attempt of one batch, on the connection, with the panic rule.
    fn attempt(
        slot: &mut Slot,
        applier: &Applier,
        game: &Uuid,
        records: &[JournalRecord],
    ) -> Result<(), StoreError> {
        let game = GameId::from_uuid(*game);
        slot.run(|conn| match conn {
            Ok(conn) => applier(conn, game, records),
            Err(error) => Err(error),
        })
        .unwrap_or(Err(StoreError::Internal))
    }

    fn pass(&mut self, force: bool) {
        let Worker {
            slot,
            backlog,
            applier,
            clock,
            ..
        } = self;
        backlog.pass(force, &|| clock(), &mut |game, records| {
            let result = Worker::attempt(slot, applier, game, records);
            if let Err(error) = &result {
                tracing::debug!(?error, "apply attempt from the backlog did not commit");
            }
            classify(&result)
        });
    }

    fn settle_waiters(&mut self) {
        // FIND-D3R-08: a caller that stopped waiting (its future was dropped) is forgotten now,
        // not at its own deadline.
        self.waiters.retain(|waiter| !waiter.reply.is_closed());
        if self.waiters.is_empty() {
            return;
        }
        // FIND-D3R-03: a waiter forces a pass every `every_ms` (a quarter of its timeout, at
        // least `POLL_MS`), so a lock that is released inside its window is noticed inside it.
        let now = self.now();
        if !self.backlog.is_empty() && self.waiters.iter().any(|w| now >= w.next_try_ms) {
            self.pass(true);
            for waiter in &mut self.waiters {
                if now >= waiter.next_try_ms {
                    waiter.next_try_ms = now.saturating_add(waiter.every_ms);
                }
            }
        }
        let now = self.now();
        let empty = self.backlog.is_empty();
        let parked = self.backlog.first_parked();
        for waiter in std::mem::take(&mut self.waiters) {
            if empty {
                let _ = waiter.reply.send(Ok(()));
            } else if let Some(game) = parked {
                let _ = waiter.reply.send(Err(StoreError::GameParked {
                    game: GameId::from_uuid(game),
                }));
            } else if now >= waiter.deadline_ms {
                let _ = waiter.reply.send(Err(StoreError::Busy));
            } else {
                self.waiters.push(waiter);
            }
        }
    }

    /// Applies the waiting batches of `games` in order (D3-09). `Busy` when one cannot be
    /// applied yet, `GameParked` for a parked game.
    fn flush(&mut self, games: &[GameId]) -> Result<(), StoreError> {
        for game in games {
            let now = self.now();
            let Worker {
                slot,
                backlog,
                applier,
                ..
            } = self;
            let outcome = backlog.flush_game(&game.as_uuid(), now, &mut |key, records| {
                classify(&Worker::attempt(slot, applier, key, records))
            });
            match outcome {
                Flush::Drained => {}
                Flush::Transient => return Err(StoreError::Busy),
                Flush::Parked => return Err(StoreError::GameParked { game: *game }),
            }
        }
        Ok(())
    }

    /// How long to wait for a command: `None` blocks.
    fn wait_ms(&self) -> Option<u64> {
        let now = self.now();
        let from_backlog = self.backlog.wait_ms(now);
        let from_waiters = (!self.waiters.is_empty()).then_some(POLL_MS);
        match (from_backlog, from_waiters) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(POLL_MS).min(b.unwrap_or(POLL_MS)).min(POLL_MS)),
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Run(job) => {
                let _ = self.slot.run(job);
            }
            Command::Apply {
                game,
                records,
                reply,
            } => {
                let outcome = self.apply(game, records);
                let _ = reply.send(outcome);
            }
            Command::RunFlushed { games, job } => match self.flush(&games) {
                Ok(()) => {
                    let _ = self.slot.run(job);
                }
                Err(error) => job(Err(error)),
            },
            Command::Unpark { game, reply } => {
                let _ = reply.send(self.backlog.unpark(&game.as_uuid()));
            }
            Command::Parked { reply } => {
                let now = self.now();
                let _ = reply.send(
                    self.backlog
                        .parked_games(now)
                        .into_iter()
                        .map(|(game, cause, pending_batches, age_ms)| ParkedGame {
                            game: GameId::from_uuid(game),
                            cause,
                            pending_batches,
                            parked_for: Duration::from_millis(age_ms),
                        })
                        .collect(),
                );
            }
            #[cfg(test)]
            Command::Waiters { reply } => {
                let _ = reply.send(self.waiters.len());
            }
            Command::Sync { timeout, reply } => {
                self.pass(true);
                let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
                let now = self.now();
                let every_ms = (timeout_ms / 4).max(POLL_MS);
                self.waiters.push(Waiter {
                    deadline_ms: now.saturating_add(timeout_ms),
                    next_try_ms: now.saturating_add(every_ms),
                    every_ms,
                    reply,
                });
                self.settle_waiters();
            }
            Command::Status { reply } => {
                let _ = reply.send(self.backlog.status(self.now()));
            }
        }
    }

    fn apply(
        &mut self,
        game: GameId,
        records: Vec<JournalRecord>,
    ) -> Result<ApplyOutcome, StoreError> {
        let key = game.as_uuid();
        // An empty batch changes nothing, so it neither waits nor is refused.
        if records.is_empty() {
            return Ok(ApplyOutcome::Applied);
        }
        // A parked game takes nothing more: a batch behind the failed one would leave a hole
        // (`journal_seq` would claim records the database never got) and the queue would grow
        // without bound (D3 ruling 2026-11-24).
        if self.backlog.is_parked(&key) {
            return Err(StoreError::GameParked { game });
        }
        // Order: behind an earlier batch of the game. Load: while some game waits for a retry
        // the database is believed busy, so this batch waits too instead of costing an attempt.
        if self.backlog.has_game(&key) || self.backlog.has_retrying() {
            let now = self.now();
            if self.backlog.len_of(&key) >= MAX_BATCHES_PER_GAME
                || self.backlog.len() >= MAX_PENDING_BATCHES
            {
                // Park rather than drop a queued batch (D-145); a game with no queue has
                // nothing to hold and only gets the error.
                self.backlog.park(key, ParkCause::QueueFull, now);
                return Err(StoreError::BacklogFull);
            }
            self.backlog.push(key, records, now);
            return Ok(ApplyOutcome::Queued);
        }
        let result = Worker::attempt(&mut self.slot, &self.applier, &key, &records);
        match classify(&result) {
            Attempt::Done => Ok(ApplyOutcome::Applied),
            Attempt::Transient => {
                let now = self.now();
                self.backlog.push(key, records, now);
                Ok(ApplyOutcome::Queued)
            }
            Attempt::Permanent(cause) => {
                // The same rule as a refused retry: the batch stays at the head, the game is
                // parked, the error goes to the caller.
                let now = self.now();
                self.backlog.push(key, records, now);
                self.backlog.park(key, cause, now);
                result.map(|()| ApplyOutcome::Applied)
            }
        }
    }

    fn run(mut self, receiver: &mpsc::Receiver<Command>) {
        loop {
            self.pass(false);
            self.settle_waiters();
            let command = match self.wait_ms() {
                None => match receiver.recv() {
                    Ok(command) => command,
                    Err(_) => break,
                },
                Some(ms) => match receiver.recv_timeout(Duration::from_millis(ms)) {
                    Ok(command) => command,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                },
            };
            self.handle(command);
        }
        if !self.backlog.is_empty() {
            // The journal holds every one of these (SEQ-ST-02); the next start reconciles.
            tracing::warn!(
                batches = self.backlog.status(self.now()).pending_batches,
                "the store thread ended with batches in the apply backlog"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::task::{Context, Waker};

    use super::*;

    /// FIND-D3R-11: every test call has an outer real-time limit, so a regression in the timer
    /// or the sync code fails the test instead of hanging CI. The injected clock is frozen in
    /// most of these tests; only real time can end a hang.
    const OUTER_LIMIT: Duration = Duration::from_secs(30);

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                tokio::time::timeout(OUTER_LIMIT, future)
                    .await
                    .expect("the call did not finish within the outer limit")
            })
    }

    fn spawn_plain(conn: Connection, reopen: Reopen) -> Result<StoreThread, StoreError> {
        StoreThread::spawn(conn, reopen, Arc::new(|_, _, _| Ok(())), instant_clock())
    }

    fn memory() -> Result<Connection, StoreError> {
        Ok(Connection::open_in_memory()?)
    }

    fn thread_with_reopen_counter() -> (StoreThread, Arc<AtomicU32>) {
        let reopened = Arc::new(AtomicU32::new(0));
        let counter = Arc::clone(&reopened);
        let thread = spawn_plain(
            memory().unwrap(),
            Box::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                memory()
            }),
        )
        .unwrap();
        (thread, reopened)
    }

    #[test]
    fn runs_a_closure_on_a_separate_thread_and_returns_its_result() {
        let (thread, _) = thread_with_reopen_counter();
        let caller = std::thread::current().id();
        let answer = block_on(thread.run(move |conn| {
            conn.execute_batch("CREATE TABLE t (x)")?;
            Ok(std::thread::current().id() != caller)
        }))
        .unwrap();
        assert!(answer, "the closure ran on the store thread");
        // State lives in the one connection the thread owns.
        let rows: i64 = block_on(thread.run(|conn| {
            Ok(conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))?)
        }))
        .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_panic_is_internal_and_the_connection_is_reopened_once_then_calls_are_unavailable() {
        let (thread, reopened) = thread_with_reopen_counter();
        let first = block_on(thread.run(|_| -> Result<(), StoreError> { panic!("boom") }));
        assert!(matches!(first, Err(StoreError::Internal)), "{first:?}");
        // The caller is answered before the thread reopens; the next command runs after it.
        // The replacement connection is a fresh in-memory database and works.
        block_on(thread.run(|conn| Ok(conn.execute_batch("CREATE TABLE t (x)")?))).unwrap();
        assert_eq!(reopened.load(Ordering::SeqCst), 1);
        let second = block_on(thread.run(|_| -> Result<(), StoreError> { panic!("again") }));
        assert!(matches!(second, Err(StoreError::Internal)), "{second:?}");
        assert_eq!(reopened.load(Ordering::SeqCst), 1, "only one restart");
        let after = block_on(thread.run(|_| Ok(())));
        assert!(
            matches!(after, Err(StoreError::Unavailable { .. })),
            "{after:?}"
        );
    }

    #[test]
    fn a_caller_that_stops_waiting_does_not_cancel_the_work() {
        let (thread, _) = thread_with_reopen_counter();
        block_on(thread.run(|conn| Ok(conn.execute_batch("CREATE TABLE t (x)")?))).unwrap();
        let mut future = Box::pin(thread.run(|conn| {
            conn.execute_batch("INSERT INTO t VALUES (1)")?;
            Ok(())
        }));
        // One poll sends the job; then the caller walks away.
        let mut context = Context::from_waker(Waker::noop());
        let _ = future.as_mut().poll(&mut context);
        drop(future);
        let count: i64 = block_on(
            thread.run(|conn| Ok(conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0))?)),
        )
        .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_failed_reopen_leaves_the_thread_unavailable() {
        let thread = spawn_plain(memory().unwrap(), Box::new(|| Err(StoreError::Corrupt))).unwrap();
        let _ = block_on(thread.run(|_| -> Result<(), StoreError> { panic!("boom") }));
        let after = block_on(thread.run(|_| Ok(())));
        assert!(
            matches!(after, Err(StoreError::Unavailable { .. })),
            "{after:?}"
        );
    }

    #[test]
    fn dropping_the_handle_joins_the_thread_and_closes_the_connection() {
        let (thread, _) = thread_with_reopen_counter();
        drop(thread); // must return: the loop ends when the channel closes
    }

    // ---- Part D3: the backlog on the thread, with an injected clock ----

    use std::sync::atomic::AtomicU64;

    use crate::journal::format::Payload;

    /// A clock the test moves by hand, and an applier that records `(clock, game, seq)` of each
    /// attempt and answers from a script.
    struct Rig {
        thread: StoreThread,
        now: Arc<AtomicU64>,
        calls: Arc<Mutex<Vec<(u64, u128, u64)>>>,
        script: Arc<Mutex<Vec<Result<(), StoreError>>>>,
    }

    fn record(seq: u64) -> JournalRecord {
        JournalRecord {
            seq,
            wall_ms: 0,
            payload: Payload::Pause,
        }
    }

    fn game(n: u128) -> GameId {
        GameId::from_uuid(Uuid::from_u128(n))
    }

    impl Rig {
        /// `script` is consumed front to back, one answer per attempt; an empty script answers
        /// `Ok`.
        fn new(script: Vec<Result<(), StoreError>>) -> Rig {
            let now = Arc::new(AtomicU64::new(0));
            let calls: Arc<Mutex<Vec<(u64, u128, u64)>>> = Arc::default();
            let script = Arc::new(Mutex::new(script));
            let (clock, log, answers) = (Arc::clone(&now), Arc::clone(&calls), Arc::clone(&script));
            let applier: Applier = Arc::new(move |_, game, records| {
                let at = clock.load(Ordering::SeqCst);
                let seq = records.first().map_or(0, |r| r.seq);
                log.lock()
                    .unwrap()
                    .push((at, game.as_uuid().as_u128(), seq));
                let mut answers = answers.lock().unwrap();
                if answers.is_empty() {
                    Ok(())
                } else {
                    answers.remove(0)
                }
            });
            let clock = Arc::clone(&now);
            let thread = StoreThread::spawn(
                memory().unwrap(),
                Box::new(memory),
                applier,
                Arc::new(move || clock.load(Ordering::SeqCst)),
            )
            .unwrap();
            Rig {
                thread,
                now,
                calls,
                script,
            }
        }

        fn set(&self, ms: u64) {
            self.now.store(ms, Ordering::SeqCst);
        }

        fn attempts(&self) -> Vec<(u64, u128, u64)> {
            self.calls.lock().unwrap().clone()
        }

        /// Waits (real time, at most 5 s) until `n` attempts were made: the thread notices the
        /// moved clock within its 100 ms poll.
        fn wait_for(&self, n: usize) {
            self.wait_for_within(n, Duration::from_secs(5));
        }

        fn wait_for_within(&self, n: usize, limit: Duration) {
            let begun = std::time::Instant::now();
            while self.attempts().len() < n {
                assert!(begun.elapsed() < limit, "{:?}", self.attempts());
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    fn busy() -> Result<(), StoreError> {
        Err(StoreError::Busy)
    }

    /// The schedule on the real thread: attempts at 0 (direct), 100, 300, 700, 1500 on the
    /// injected clock (waits 0.1, 0.2, 0.4, 0.8 s), then success; the queue is then empty.
    #[test]
    fn the_thread_retries_on_the_doubling_schedule_of_the_injected_clock() {
        let rig = Rig::new(vec![busy(), busy(), busy(), busy(), busy()]);
        let first = block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        assert_eq!(first, ApplyOutcome::Queued);
        for due in [100, 300, 700, 1_500] {
            let before = rig.attempts().len();
            rig.set(due - 1);
            std::thread::sleep(Duration::from_millis(250));
            assert_eq!(rig.attempts().len(), before, "nothing before {due}");
            rig.set(due);
            rig.wait_for(before + 1);
        }
        rig.set(1_500 + 1_600);
        rig.wait_for(6);
        let times: Vec<u64> = rig.attempts().iter().map(|c| c.0).collect();
        assert_eq!(times, vec![0, 100, 300, 700, 1_500, 3_100]);
        block_on(rig.thread.sync(Duration::from_secs(5))).unwrap();
        assert_eq!(block_on(rig.thread.status()).unwrap().pending_batches, 0);
    }

    /// The thread notices a clock that jumps ahead within its poll (100 ms), not after the wait it
    /// computed earlier: with the wait grown to 5 s, a jump of 5 s is acted on within 1.5 s.
    #[test]
    fn a_jump_of_the_injected_clock_is_noticed_within_the_poll_not_after_the_old_wait() {
        let rig = Rig::new((0..12).map(|_| busy()).collect());
        block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        let mut at = 0;
        for n in 2..=9 {
            at += 6_000;
            rig.set(at);
            rig.wait_for_within(n, Duration::from_secs(10));
        }
        // The next wait is 5 s (the cap). Jump 5 s: the old wait would be 5 s of real time.
        let before = rig.attempts().len();
        rig.set(at + 5_000);
        rig.wait_for_within(before + 1, Duration::from_millis(1_500));
    }

    /// A batch of a game with a waiting batch queues behind it and applies after it, in order,
    /// whatever else happens; a batch of another game does not cost a database attempt while the
    /// backlog is retrying.
    #[test]
    fn batches_of_one_game_apply_in_order_and_a_retrying_backlog_costs_no_extra_attempt() {
        let rig = Rig::new(vec![busy()]);
        assert_eq!(
            block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap(),
            ApplyOutcome::Queued
        );
        assert_eq!(
            block_on(rig.thread.apply(game(1), vec![record(2)])).unwrap(),
            ApplyOutcome::Queued
        );
        assert_eq!(
            block_on(rig.thread.apply(game(2), vec![record(9)])).unwrap(),
            ApplyOutcome::Queued
        );
        assert_eq!(
            rig.attempts().len(),
            1,
            "only the first call touched the database"
        );
        let status = block_on(rig.thread.status()).unwrap();
        assert_eq!((status.pending_batches, status.pending_games), (3, 2));
        rig.set(100);
        rig.wait_for(4);
        let order: Vec<(u128, u64)> = rig.attempts().iter().skip(1).map(|c| (c.1, c.2)).collect();
        assert_eq!(order, vec![(1, 1), (1, 2), (2, 9)]);
    }

    /// D3 ruling 2026-11-24 (FIND-D3R-01): an error that no retry cures parks the game on the
    /// first attempt as on a retry. The error goes to the caller, the batch stays at the head,
    /// later batches of that game are refused (not queued), other games go on, `sync_backlog`
    /// says `GameParked` instead of `Busy`, `parked_games` says what failed, and `unpark` drops the
    /// held queue so that the caller can pass the whole journal.
    #[test]
    fn a_refused_first_attempt_parks_the_game_and_later_batches_are_refused() {
        let rig = Rig::new(vec![Err(StoreError::InvariantViolated)]);
        let first = block_on(rig.thread.apply(game(1), vec![record(1)]));
        assert!(
            matches!(first, Err(StoreError::InvariantViolated)),
            "{first:?}"
        );
        let status = block_on(rig.thread.status()).unwrap();
        assert_eq!((status.pending_batches, status.parked_games), (1, 1));
        // Later batches of the game are refused and not queued (nothing applied behind a hole).
        for seq in [2, 3] {
            let later = block_on(rig.thread.apply(game(1), vec![record(seq)]));
            assert!(
                matches!(later, Err(StoreError::GameParked { game: g }) if g == game(1)),
                "{later:?}"
            );
        }
        assert_eq!(rig.attempts().len(), 1, "no attempt behind the parked head");
        assert_eq!(block_on(rig.thread.status()).unwrap().pending_batches, 1);
        // Another game is not affected.
        assert_eq!(
            block_on(rig.thread.apply(game(2), vec![record(9)])).unwrap(),
            ApplyOutcome::Applied
        );
        // The barrier names the park instead of waiting for ever.
        let sync = block_on(rig.thread.sync(Duration::from_secs(60)));
        assert!(
            matches!(sync, Err(StoreError::GameParked { game: g }) if g == game(1)),
            "{sync:?}"
        );
        let parked = block_on(rig.thread.parked()).unwrap();
        assert_eq!(parked.len(), 1);
        assert_eq!(
            (parked[0].game, parked[0].cause, parked[0].pending_batches),
            (game(1), ParkCause::InvariantViolated, 1)
        );
        // `unpark` drops the held queue (D-145): nothing is retried, the backlog is empty.
        let attempts = rig.attempts().len();
        assert_eq!(block_on(rig.thread.unpark(game(1))).unwrap(), Some(1));
        assert_eq!(block_on(rig.thread.unpark(game(1))).unwrap(), None);
        block_on(rig.thread.sync(Duration::from_secs(5))).unwrap();
        assert_eq!(
            rig.attempts().len(),
            attempts,
            "the dropped batch is not applied"
        );
        assert_eq!(block_on(rig.thread.status()).unwrap().pending_batches, 0);
        assert!(block_on(rig.thread.parked()).unwrap().is_empty());
        assert_eq!(
            block_on(rig.thread.apply(game(1), vec![record(4)])).unwrap(),
            ApplyOutcome::Applied
        );
    }

    /// The retry path of the same rule: a batch queued behind a busy one that then fails for good
    /// parks the game; the batches behind it stay queued in order, new ones are refused.
    #[test]
    fn a_refused_retry_parks_the_game_and_keeps_the_queue_in_order() {
        let rig = Rig::new(vec![busy(), Err(StoreError::Corrupt)]);
        assert_eq!(
            block_on(rig.thread.apply(game(1), vec![record(2)])).unwrap(),
            ApplyOutcome::Queued
        );
        block_on(rig.thread.apply(game(1), vec![record(3)])).unwrap();
        rig.set(100);
        rig.wait_for(2);
        let status = block_on(rig.thread.status()).unwrap();
        assert_eq!((status.pending_batches, status.parked_games), (2, 1));
        let later = block_on(rig.thread.apply(game(1), vec![record(4)]));
        assert!(
            matches!(later, Err(StoreError::GameParked { .. })),
            "{later:?}"
        );
        let parked = block_on(rig.thread.parked()).unwrap();
        assert_eq!(
            (parked[0].cause, parked[0].pending_batches),
            (ParkCause::Corrupt, 2)
        );
        assert_eq!(block_on(rig.thread.unpark(game(1))).unwrap(), Some(2));
        block_on(rig.thread.sync(Duration::from_secs(5))).unwrap();
        let order: Vec<u64> = rig.attempts().iter().map(|c| c.2).collect();
        assert_eq!(
            order,
            vec![2, 2],
            "the direct busy one and the refused retry, nothing after"
        );
        // The caller passes the whole journal again: applied.
        assert_eq!(
            block_on(rig.thread.apply(game(1), vec![record(2), record(3)])).unwrap(),
            ApplyOutcome::Applied
        );
    }

    /// FIND-D3R-09: `Busy` and `Unavailable` are transient; everything else parks, with its
    /// category.
    #[test]
    fn classify_separates_what_a_retry_cures_from_what_parks() {
        let transient = [
            StoreError::Busy,
            StoreError::Unavailable {
                cause: UnavailableCause::DiskFull,
            },
        ];
        for error in transient {
            assert_eq!(classify(&Err(error)), Attempt::Transient);
        }
        let cause = |e: StoreError| match classify(&Err(e)) {
            Attempt::Permanent(cause) => cause,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            cause(StoreError::InvariantViolated),
            ParkCause::InvariantViolated
        );
        assert_eq!(
            cause(StoreError::NotFound {
                entity: crate::error::Entity::Player,
                id: Uuid::nil()
            }),
            ParkCause::NotFound
        );
        assert_eq!(cause(StoreError::Corrupt), ParkCause::Corrupt);
        assert_eq!(cause(StoreError::Internal), ParkCause::Internal);
        assert_eq!(cause(StoreError::NameTaken), ParkCause::Other);
        assert_eq!(classify(&Ok(())), Attempt::Done);
    }

    /// D-145 bounds (ESTIMATE constants): a game that reaches `MAX_BATCHES_PER_GAME` gets
    /// `BacklogFull` for that batch and is parked with `QueueFull`; nothing queued is dropped, later
    /// batches get `GameParked`, `unpark` drops the 256 held batches and the game takes the whole
    /// journal again.
    #[test]
    fn a_game_at_its_queue_bound_is_parked_with_queue_full_and_nothing_is_dropped() {
        let rig = Rig::new(vec![busy()]);
        for n in 0..MAX_BATCHES_PER_GAME {
            let n = u64::try_from(n).unwrap();
            assert_eq!(
                block_on(rig.thread.apply(game(1), vec![record(n)])).unwrap(),
                ApplyOutcome::Queued
            );
        }
        // Another game is not affected by the first game's bound.
        assert_eq!(
            block_on(rig.thread.apply(game(2), vec![record(1)])).unwrap(),
            ApplyOutcome::Queued
        );
        let over = block_on(rig.thread.apply(game(1), vec![record(9_999)]));
        assert!(matches!(over, Err(StoreError::BacklogFull)), "{over:?}");
        let again = block_on(rig.thread.apply(game(1), vec![record(10_000)]));
        assert!(
            matches!(again, Err(StoreError::GameParked { .. })),
            "{again:?}"
        );
        let parked = block_on(rig.thread.parked()).unwrap();
        assert_eq!(
            (parked[0].game, parked[0].cause, parked[0].pending_batches),
            (game(1), ParkCause::QueueFull, MAX_BATCHES_PER_GAME)
        );
        let sync = block_on(rig.thread.sync(Duration::from_secs(1)));
        assert!(
            matches!(sync, Err(StoreError::GameParked { .. })),
            "{sync:?}"
        );
        assert_eq!(
            block_on(rig.thread.unpark(game(1))).unwrap(),
            Some(MAX_BATCHES_PER_GAME)
        );
        block_on(rig.thread.sync(Duration::from_secs(20))).unwrap();
        assert_eq!(
            block_on(rig.thread.apply(game(1), vec![record(1), record(2)])).unwrap(),
            ApplyOutcome::Applied
        );
    }

    /// The whole backlog holds `MAX_PENDING_BATCHES`: with every game at 256 or fewer, the 4 097th
    /// batch is refused with `BacklogFull`. A game that has a queue is parked; a game that has none
    /// only gets the error (nothing of it is held), and room returns when the queues drain.
    #[test]
    fn the_backlog_as_a_whole_is_bounded_and_recovers() {
        let rig = Rig::new(vec![busy()]);
        let games = MAX_PENDING_BATCHES / MAX_BATCHES_PER_GAME;
        for g in 0..games {
            for n in 0..MAX_BATCHES_PER_GAME {
                let seq = u64::try_from(n).unwrap();
                assert_eq!(
                    block_on(rig.thread.apply(game(1 + g as u128), vec![record(seq)])).unwrap(),
                    ApplyOutcome::Queued
                );
            }
        }
        assert_eq!(
            block_on(rig.thread.status()).unwrap().pending_batches,
            MAX_PENDING_BATCHES
        );
        let fresh = block_on(rig.thread.apply(game(1_000), vec![record(1)]));
        assert!(matches!(fresh, Err(StoreError::BacklogFull)), "{fresh:?}");
        assert!(
            block_on(rig.thread.parked()).unwrap().is_empty(),
            "no queue, no park"
        );
        assert_eq!(
            block_on(rig.thread.status()).unwrap().pending_batches,
            MAX_PENDING_BATCHES
        );
        block_on(rig.thread.sync(Duration::from_secs(30))).unwrap();
        assert_eq!(block_on(rig.thread.status()).unwrap().pending_batches, 0);
        assert_eq!(
            block_on(rig.thread.apply(game(1_000), vec![record(1)])).unwrap(),
            ApplyOutcome::Applied
        );
    }

    /// FIND-D3R-08: a `sync_backlog` future that is dropped leaves no waiter on the thread.
    #[test]
    fn a_cancelled_sync_leaves_no_waiter_behind() {
        let rig = Rig::new((0..64).map(|_| busy()).collect());
        block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        let mut context = Context::from_waker(Waker::noop());
        for _ in 0..50 {
            let mut future = Box::pin(rig.thread.sync(Duration::from_secs(3_600)));
            let _ = future.as_mut().poll(&mut context); // sends the command
            drop(future); // the caller walks away
        }
        let begun = std::time::Instant::now();
        loop {
            let left = block_on(rig.thread.waiters()).unwrap();
            if left == 0 {
                break;
            }
            assert!(
                begun.elapsed() < Duration::from_secs(3),
                "{left} waiters of cancelled syncs are still held"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// FIND-D3R-03: the lock is released inside the sync window; the sync attempts again by
    /// itself (every quarter of its timeout) and succeeds, although the retry timer, grown to
    /// 5 s, would not look before the deadline. The injected clock moves by hand.
    #[test]
    fn sync_backlog_succeeds_when_the_lock_is_released_inside_its_window() {
        let rig = Rig::new((0..64).map(|_| busy()).collect());
        block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        // Grow the timer to its 5 s cap: attempts at 100, 300, 700, 1500, 3100, 6300.
        for (n, at) in [100u64, 300, 700, 1_500, 3_100, 6_300]
            .into_iter()
            .enumerate()
        {
            rig.set(at);
            rig.wait_for(n + 2);
        }
        let before = rig.attempts().len();
        let outcome = std::thread::scope(|scope| {
            let handle = scope.spawn(|| block_on(rig.thread.sync(Duration::from_secs(2))));
            // The sync's own forced pass at 6 300 is one more (busy) attempt.
            rig.wait_for(before + 1);
            // The lock is released; the clock moves a quarter of the window on (500 ms).
            rig.script.lock().unwrap().clear();
            rig.set(6_300 + 500);
            handle.join().unwrap()
        });
        assert!(outcome.is_ok(), "{outcome:?}");
        assert_eq!(block_on(rig.thread.status()).unwrap().pending_batches, 0);
    }

    /// D3-09 on the thread: a job that runs "flushed" sees the queued batches of its game applied
    /// first, in order; when a batch cannot be applied the job does not run and the answer is
    /// `Busy`; a parked game answers `GameParked`.
    #[test]
    fn a_flushed_job_runs_after_the_queued_batches_of_its_game() {
        let rig = Rig::new(vec![busy()]);
        block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        block_on(rig.thread.apply(game(1), vec![record(2)])).unwrap();
        block_on(rig.thread.apply(game(2), vec![record(7)])).unwrap();
        let ran = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&ran);
        // Still busy: the flush cannot drain, the job does not run.
        *rig.script.lock().unwrap() = vec![busy()];
        let refused = block_on(rig.thread.run_flushed(vec![game(1)], move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        assert!(matches!(refused, Err(StoreError::Busy)), "{refused:?}");
        assert_eq!(ran.load(Ordering::SeqCst), 0);
        // Free: batches 1 and 2 of game 1 apply, game 2's waits, then the job runs.
        let seen = Arc::clone(&ran);
        let log = Arc::clone(&rig.calls);
        let at_job = block_on(rig.thread.run_flushed(vec![game(1)], move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(log.lock().unwrap().len())
        }))
        .unwrap();
        assert_eq!(ran.load(Ordering::SeqCst), 1);
        let order: Vec<(u128, u64)> = rig.attempts().iter().map(|c| (c.1, c.2)).collect();
        assert_eq!(
            order,
            vec![(1, 1), (1, 1), (1, 1), (1, 2)],
            "retry order, then 2"
        );
        assert_eq!(at_job, 4, "the job saw both batches applied before it ran");
        let status = block_on(rig.thread.status()).unwrap();
        assert_eq!(status.pending_batches, 1, "game 2 was not flushed");
        block_on(rig.thread.sync(Duration::from_secs(5))).unwrap();
        // A parked game: no job.
        *rig.script.lock().unwrap() = vec![Err(StoreError::InvariantViolated)];
        let parked = block_on(rig.thread.apply(game(3), vec![record(1)]));
        assert!(parked.is_err());
        let seen = Arc::clone(&ran);
        let out = block_on(rig.thread.run_flushed(vec![game(3)], move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        assert!(matches!(out, Err(StoreError::GameParked { .. })), "{out:?}");
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    /// `sync_backlog` tries at once (it does not wait for the timer), returns `Ok` when the
    /// backlog empties, and `Busy` once the injected clock passes its deadline.
    #[test]
    fn sync_backlog_forces_a_pass_and_times_out_on_the_injected_clock() {
        let rig = Rig::new(vec![busy()]);
        block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        // Timer not due (clock 0, due at 100): the forced pass applies it.
        block_on(rig.thread.sync(Duration::from_secs(1))).unwrap();
        assert_eq!(rig.attempts().len(), 2);
        // Now a backlog that stays busy: the waiter ends with `Busy` at its deadline.
        *rig.script.lock().unwrap() = vec![busy(), busy(), busy(), busy(), busy(), busy()];
        block_on(rig.thread.apply(game(1), vec![record(2)])).unwrap();
        let waiting = std::thread::scope(|scope| {
            let handle = scope.spawn(|| block_on(rig.thread.sync(Duration::from_secs(2))));
            std::thread::sleep(Duration::from_millis(300));
            assert!(
                !handle.is_finished(),
                "the clock has not reached the deadline"
            );
            rig.set(2_000);
            handle.join().unwrap()
        });
        assert!(matches!(waiting, Err(StoreError::Busy)), "{waiting:?}");
    }

    /// `StoreBacklog` is the oldest batch's age on the injected clock: not at 29.9 s, yes at 30 s.
    #[test]
    fn store_backlog_follows_the_age_of_the_oldest_batch_on_the_injected_clock() {
        let rig = Rig::new((0..41).map(|_| busy()).collect());
        block_on(rig.thread.apply(game(1), vec![record(1)])).unwrap();
        rig.set(29_999);
        let early = block_on(rig.thread.status()).unwrap();
        assert!(!early.store_backlog);
        rig.set(30_000);
        let late = block_on(rig.thread.status()).unwrap();
        assert!(late.store_backlog);
        assert_eq!(late.oldest_age, Duration::from_secs(30));
    }

    /// A panic inside an attempt is `Internal` for the caller, uses the restart rule, and does
    /// not take the thread down.
    #[test]
    fn a_panic_in_an_attempt_is_internal_and_the_thread_survives() {
        let applier: Applier = Arc::new(|_, _, _| panic!("boom"));
        let thread = StoreThread::spawn(
            memory().unwrap(),
            Box::new(memory),
            applier,
            instant_clock(),
        )
        .unwrap();
        let first = block_on(thread.apply(game(1), vec![record(1)]));
        assert!(matches!(first, Err(StoreError::Internal)), "{first:?}");
        block_on(thread.run(|c| Ok(c.execute_batch("CREATE TABLE t (x)")?))).unwrap();
    }
}
