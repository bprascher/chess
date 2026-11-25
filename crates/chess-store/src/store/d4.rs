//! Part D4 methods of `Store`, crate-internal (SDD 3.7.3 `open_for_resume`, 3.9.1, 3.11.3):
//! what the resume and the startup recovery need from the database. A child module of `store`,
//! like `d3` and `d5`, so that it uses the private write helpers. Every call runs on the store
//! thread **after the queued batches of the game were applied** (D3-09), so a resume reads the
//! rows that the journal already produced; a parked game answers `GameParked` and a batch that
//! cannot be applied yet answers `Busy`.

use std::time::Duration;

use chess_domain::GameId;

use super::Store;
use crate::error::StoreError;
use crate::journal::Plan;
use crate::repo::apply;
use crate::repo::resume::{self, ResumeRow};

impl Store {
    /// The `store_uuid` that every journal header of this store carries; `Corrupt` when the
    /// store has no `store.id` (FIND-007: no journal is read or written under an identity the
    /// store cannot name).
    pub(crate) fn store_uuid(&self) -> Result<uuid::Uuid, StoreError> {
        self.inner.store_id.ok_or(StoreError::Corrupt)
    }

    /// The `GAME` row, its `MOVE` rows and the seat kinds, or `None` when the game has no row.
    pub(crate) async fn resume_row(&self, game: GameId) -> Result<Option<ResumeRow>, StoreError> {
        self.write_after_flush(vec![game], move |conn| resume::load(conn, game))
            .await
    }

    /// Step 2 of the rebase (SDD 3.11.3) and the reconcile of a resume: one transaction that
    /// applies the plan and, when `journal_seq` is given, sets `GAME.journal_seq` **exactly** to
    /// it (the rebase sets the new file's last `seq`, which can be lower than the old value).
    /// The plan applier keeps the connection's normal lock wait and not the 100 ms of
    /// `apply_records`: a resume must finish, and the store thread serves one game's resume
    /// while the backlog of the others waits at most that long (FIND-D3-14, D4 decision).
    ///
    /// `wait` shortens the lock wait of this one call (the startup scan's budget, FIND-D4R-07);
    /// `None` keeps the connection's normal wait.
    pub(crate) async fn apply_reconcile(
        &self,
        game: GameId,
        plan: Plan,
        journal_seq: Option<u64>,
        wait: Option<Duration>,
    ) -> Result<(), StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write_after_flush(vec![game], move |conn| match wait {
            Some(wait) => apply::with_busy(conn, wait, |conn| {
                apply::apply_reconcile_plan(conn, game, &plan, journal_seq, &ctx)
            }),
            None => apply::apply_reconcile_plan(conn, game, &plan, journal_seq, &ctx),
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::datadir::DataDir;
    use crate::store::StoreOptions;

    /// The wait given to the plan applier is the wait it makes (FIND-D4R-07; mutant M27): with
    /// the database's write lock held elsewhere the call answers `Busy` after about the wait,
    /// not after the connection's 5 s.
    #[test]
    fn apply_reconcile_waits_for_the_lock_no_longer_than_it_is_told() {
        let dir = tempfile::TempDir::new().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let store = runtime
            .block_on(Store::open(StoreOptions::new(DataDir::new(dir.path()))))
            .unwrap()
            .store;
        let other = rusqlite::Connection::open(dir.path().join("chess.db")).unwrap();
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        let game = GameId::from_uuid(uuid::Uuid::from_u128(1));
        let started = Instant::now();
        let result = runtime.block_on(store.apply_reconcile(
            game,
            Plan {
                replace_from: None,
                rows_to_delete_from: None,
                rows_to_write: Vec::new(),
                clocks: None,
                source: crate::journal::RunSource::Both,
                damaged: false,
            },
            None,
            Some(Duration::from_millis(300)),
        ));
        let took = started.elapsed();
        assert!(matches!(result, Err(StoreError::Busy)), "{result:?}");
        assert!(took < Duration::from_secs(3), "waited {took:?}");
        other.execute_batch("ROLLBACK").unwrap();
    }
}
