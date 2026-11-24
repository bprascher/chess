//! Part D5 methods of `Store` (SDD 3.7.2: Statistics, Analysis, Jobs). A child module of
//! `store` so that it uses the private `read`/`write` helpers without editing the part A
//! method list. Reads go through the read pool, writes through the store thread; the clock is
//! the injected one (IFR-ST-05), so a test drives the 10 s claim lease.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use chess_domain::{GameId, PlayerId, WallMs};

use super::Store;
use crate::api::{GameFilter, GameRecord};
use crate::api_analysis::{GameAnalysis, MoveEvalRow, StatsGameRow, StatsSide, VariationRecord};
use crate::error::StoreError;
use crate::repo::{analysis, game, jobs, stats};

impl Store {
    // ---- Statistics rows (R, D-44) ----

    /// The games that name `player` on `side` (either side when `None`), started at or after
    /// `window`, in start order, at most `limit`. No predicate on self-play, status or custom
    /// start: `chess-domain::aggregate` owns that.
    pub async fn stats_rows(
        &self,
        player: PlayerId,
        window: Option<WallMs>,
        side: Option<StatsSide>,
        limit: Option<u32>,
    ) -> Result<Vec<StatsGameRow>, StoreError> {
        self.read(move |conn| stats::stats_rows(conn, player, window, side, limit))
            .await
    }

    /// The games of `player` against `opponent` on either side, viewed from `player`.
    pub async fn head_to_head_rows(
        &self,
        player: PlayerId,
        opponent: PlayerId,
        window: Option<WallMs>,
        limit: Option<u32>,
    ) -> Result<Vec<StatsGameRow>, StoreError> {
        self.read(move |conn| stats::head_to_head_rows(conn, player, opponent, window, limit))
            .await
    }

    /// Whole `GAME` rows of the games that name `player`, started at or after `window`, in start
    /// order, at most `limit`: history lists include everything (UI-2, UI-5).
    pub async fn games_for_history(
        &self,
        player: PlayerId,
        window: Option<WallMs>,
        limit: Option<u32>,
    ) -> Result<Vec<GameRecord>, StoreError> {
        let filter = GameFilter {
            player: Some(player),
            status: None,
            from: window,
            limit,
        };
        self.read(move |conn| game::list_games(conn, &filter)).await
    }

    // ---- Evaluations and variations ----

    /// One upsert per ply; `cp` and `mate` are from White's point of view.
    pub async fn upsert_move_eval(&self, eval: MoveEvalRow) -> Result<(), StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| analysis::upsert_move_eval(conn, &eval, &ctx))
            .await
    }

    /// Stored evaluations, job state, `evaluated` and `total`.
    pub async fn get_game_analysis(&self, game: GameId) -> Result<GameAnalysis, StoreError> {
        self.read(move |conn| analysis::get_game_analysis(conn, game))
            .await
    }

    /// Saves one line, identified by `(game, from_ply, pgn)` (exact text). Idempotent: a key that
    /// is already stored returns the stored record (original `created_at`) and writes nothing.
    /// A new key with `from_ply` at or beyond the stored ply count is `VariationAnchor`; an
    /// empty text is `InvariantViolated`, and a text above `MAX_VARIATION_PGN_BYTES` is
    /// `VariationTooLarge`. `created_at` comes from the injected clock.
    pub async fn save_variation(
        &self,
        game: GameId,
        from_ply: u32,
        pgn: String,
    ) -> Result<VariationRecord, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| analysis::save_variation(conn, game, from_ply, &pgn, clock(), &ctx))
            .await
    }

    /// Deletes the row with this key. True when a row was deleted, false when the game exists and
    /// holds no such line. `NotFound` for an unknown game.
    pub async fn delete_variation(
        &self,
        game: GameId,
        from_ply: u32,
        pgn: String,
    ) -> Result<bool, StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| analysis::delete_variation(conn, game, from_ply, &pgn, &ctx))
            .await
    }

    pub async fn list_variations(&self, game: GameId) -> Result<Vec<VariationRecord>, StoreError> {
        self.read(move |conn| analysis::list_variations(conn, game))
            .await
    }

    // ---- Job queue (W, 3.11.7) ----

    /// Queues the game's job; true when a row was created.
    pub async fn enqueue_job(&self, game: GameId) -> Result<bool, StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| jobs::enqueue_job(conn, game, &ctx))
            .await
    }

    /// Queues every finished game with fewer evaluations than moves and no job; returns how many.
    pub async fn enqueue_unevaluated_games(&self) -> Result<u32, StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| jobs::enqueue_unevaluated_games(conn, &ctx))
            .await
    }

    /// Claims the lowest claimable job (queued, or running with a heartbeat older than
    /// `ANALYSIS_LEASE`) for `owner`.
    pub async fn claim_job(&self, owner: String) -> Result<Option<GameId>, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| jobs::claim_job(conn, &owner, clock(), &ctx))
            .await
    }

    /// Refreshes the claim; `NotOwner` for a worker that lost the job.
    pub async fn job_heartbeat(&self, game: GameId, owner: String) -> Result<(), StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| jobs::job_heartbeat(conn, game, &owner, clock(), &ctx))
            .await
    }

    /// `running -> done`; `NotOwner` for a worker that does not hold the job.
    pub async fn complete_job(&self, game: GameId, owner: String) -> Result<(), StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| jobs::complete_job(conn, game, &owner, &ctx))
            .await
    }

    /// `running -> failed`; `NotOwner` for a worker that does not hold the job.
    pub async fn fail_job(&self, game: GameId, owner: String) -> Result<(), StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write(move |conn| jobs::fail_job(conn, game, &owner, &ctx))
            .await
    }

    /// Moves every `failed` job back to `queued`, **once per start** of this `Store` (bounded
    /// retry without an attempt counter column). Later calls on the same open return 0.
    pub async fn enqueue_failed_once_per_start(&self) -> Result<u32, StoreError> {
        let flag = &self.inner.failed_jobs_requeued;
        if flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(0);
        }
        let ctx = self.inner.data_dir.db_path();
        let result = self
            .write(move |conn| jobs::requeue_failed(conn, &ctx))
            .await;
        if result.is_err() {
            // The retry did not happen: allow another attempt in this start.
            flag.store(false, Ordering::SeqCst);
        }
        result
    }
}
