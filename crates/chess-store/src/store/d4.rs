//! Part D4 methods of `Store`, crate-internal (SDD 3.7.3 `open_for_resume`, 3.9.1, 3.11.3):
//! what the resume and the startup recovery need from the database. A child module of `store`,
//! like `d3` and `d5`, so that it uses the private write helpers. Every call runs on the store
//! thread **after the queued batches of the game were applied** (D3-09), so a resume reads the
//! rows that the journal already produced; a parked game answers `GameParked` and a batch that
//! cannot be applied yet answers `Busy`.

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
    pub(crate) async fn apply_reconcile(
        &self,
        game: GameId,
        plan: Plan,
        journal_seq: Option<u64>,
    ) -> Result<(), StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write_after_flush(vec![game], move |conn| {
            apply::apply_reconcile_plan(conn, game, &plan, journal_seq, &ctx)
        })
        .await
    }
}
