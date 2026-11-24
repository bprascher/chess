//! Part D3 methods of `Store` (SDD 3.7.2: `apply_records`, `sync_backlog`; SDD 3.12 backlog).
//! A child module of `store`, like `d5`, so that the part A method list stays as it was. Every
//! call goes to the store thread, which owns the backlog and its retry timer.

use std::time::Duration;

use chess_domain::GameId;

use super::Store;
use crate::api::{ApplyOutcome, BacklogStatus, ParkedGame};
use crate::error::StoreError;
use crate::journal::JournalRecord;

impl Store {
    /// Applies journal records to the database, in order, in one transaction (SDD 3.7.2, 3.11.1).
    /// The caller has already appended and synced the records (journal first, SEQ-ST-02); the
    /// database follows the journal.
    ///
    /// - `Ok(Applied)`: committed.
    /// - `Ok(Queued)`: the database is busy or unavailable, or an earlier batch of this game still
    ///   waits. The batch keeps its place in the game's queue and is retried (0.1 s doubling to
    ///   5 s, 100 ms lock wait per attempt). Play does not wait for it.
    /// - `Err`: the batch itself is refused (for example an unknown player or a `Move` for a game
    ///   that has no row): nothing was written, the batch stays at the head of the game's queue
    ///   and the game is **parked** (D3 ruling 2026-11-24), on the first attempt as on a retry.
    ///   Later batches of a parked game are refused with `GameParked`; `unpark_game` or the next
    ///   start (which reconciles from the journal) ends the park. `BacklogFull` when the game already
    ///   holds `MAX_BATCHES_PER_GAME` batches or the backlog `MAX_PENDING_BATCHES` (256 and 4 096,
    ///   ESTIMATE): the batch is not queued and a game that has a queue is parked.
    ///
    /// A `GameEnd` that has a `Voided` after it in the same batch is not finalized (D-87).
    pub async fn apply_records(
        &self,
        game: GameId,
        records: Vec<JournalRecord>,
    ) -> Result<ApplyOutcome, StoreError> {
        self.inner.writer.apply(game, records).await
    }

    /// Waits until the apply backlog is empty (a barrier before an export, SDD 3.11.9). It tries
    /// the waiting batches at once, and again every quarter of `timeout` (at least 100 ms), instead
    /// of waiting for the timer. `GameParked` as soon as a game is parked (the barrier cannot be
    /// met until `unpark_game` or a restart); `Busy` when the backlog is not empty after
    /// `timeout`.
    pub async fn sync_backlog(&self, timeout: Duration) -> Result<(), StoreError> {
        self.inner.writer.sync(timeout).await
    }

    /// The size and age of the apply backlog; `store_backlog` is the `StoreBacklog` condition
    /// (oldest batch 30 s old or more).
    pub async fn backlog_status(&self) -> Result<BacklogStatus, StoreError> {
        self.inner.writer.status().await
    }

    /// The games whose database updates are parked, with what each failed with, how many
    /// batches wait and for how long (D3 ruling 2026-11-24).
    pub async fn parked_games(&self) -> Result<Vec<ParkedGame>, StoreError> {
        self.inner.writer.parked().await
    }

    /// Releases a parked game and **drops the queue it held** (D-145): the number of batches
    /// dropped, `Ok(None)` when the game was not parked. Nothing is lost: the journal holds every
    /// record, so the caller passes the whole journal of the game in one `apply_records` call
    /// (idempotent) afterwards, and must not pass a later batch alone (the database would skip
    /// the dropped records). If that call fails for good the game is parked again; the way out
    /// then is the next start, which reconciles the rows from the journal.
    pub async fn unpark_game(&self, game: GameId) -> Result<Option<usize>, StoreError> {
        self.inner.writer.unpark(game).await
    }
}
