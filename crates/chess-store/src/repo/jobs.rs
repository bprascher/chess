//! DD-ST-013 `JobQueue` (SDD 3.11.7, STATE-ST-02). Functions over a connection; `now` is an
//! argument, so a test drives the 10 s claim lease with an injected clock.
//!
//! States: `queued -> running` (claim), `running -> done | failed` (complete, fail),
//! `running -> running` (a claim by a new owner once the heartbeat is older than the lease),
//! `failed -> queued` (only `enqueue_failed_once_per_start`). A job never leaves `done`.
//! `job_heartbeat`, `complete_job` and `fail_job` update only `WHERE game_id = ? AND owner = ?
//! AND state = 'running'` (compare-and-set), so a worker that lost the job gets `NotOwner` and
//! cannot overwrite the new owner. `complete_job` and `fail_job` are idempotent for the owner
//! that already ended the job in that state (a retry after a lost reply answers `Ok`).
//!
//! The store does not check that the game is finished: "takes only jobs of finished games" is
//! the analysis worker's rule (SDD-chess-app, DAT-3); `enqueue_unevaluated_games` selects only
//! finished games itself.

use std::path::Path;

use chess_domain::GameId;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::api_analysis::JobState;
use crate::error::StoreError;
use crate::repo::analysis::failpoint;
use crate::repo::{game_key, parse_uuid};

fn lease_ms() -> i64 {
    i64::try_from(crate::repo::coord::ANALYSIS_LEASE.as_millis()).unwrap_or(i64::MAX)
}

/// Queues the game's job. True when a row was created. An existing job in any state is left
/// alone (a `done` job never goes back, a `failed` one is retried only by
/// `enqueue_failed_once_per_start`). `NotFound` for an unknown game.
pub(crate) fn enqueue_job(
    conn: &mut Connection,
    game: GameId,
    ctx: &Path,
) -> Result<bool, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, game)?;
    let created = tx.execute(
        "INSERT INTO analysis_job (game_id, state) VALUES (?1, 'queued') \
         ON CONFLICT (game_id) DO NOTHING",
        [key],
    )?;
    failpoint(ctx, "job.before_commit")?;
    tx.commit()?;
    Ok(created > 0)
}

/// Queues a job for every finished game (status other than `in_progress`) that has no job, has
/// at least one stored move and has fewer evaluations than moves. Returns the number queued.
pub(crate) fn enqueue_unevaluated_games(
    conn: &mut Connection,
    ctx: &Path,
) -> Result<u32, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let queued = tx.execute(
        "INSERT INTO analysis_job (game_id, state) \
         SELECT g.id, 'queued' FROM game g \
         WHERE g.status <> 'in_progress' \
           AND NOT EXISTS (SELECT 1 FROM analysis_job j WHERE j.game_id = g.id) \
           AND EXISTS (SELECT 1 FROM move m WHERE m.game_id = g.id) \
           AND (SELECT count(*) FROM move_eval e WHERE e.game_id = g.id) \
               < (SELECT count(*) FROM move m WHERE m.game_id = g.id) \
         ORDER BY g.id",
        [],
    )?;
    failpoint(ctx, "job.before_commit")?;
    tx.commit()?;
    u32::try_from(queued).map_err(|_| StoreError::Internal)
}

/// Claims the lowest-numbered claimable job for `owner`: `queued`, or `running` with a
/// heartbeat older than the 10 s lease (or none). `None` when nothing is claimable. A heartbeat
/// dated in the future counts as fresh (a clock step must not steal a live job).
pub(crate) fn claim_job(
    conn: &mut Connection,
    owner: &str,
    now: i64,
    ctx: &Path,
) -> Result<Option<GameId>, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let stale_before = now.saturating_sub(lease_ms());
    let found = tx
        .query_row(
            "SELECT j.game_id, g.uuid FROM analysis_job j JOIN game g ON g.id = j.game_id \
             WHERE j.state = 'queued' \
                OR (j.state = 'running' AND (j.heartbeat_at IS NULL OR j.heartbeat_at < ?1)) \
             ORDER BY j.game_id LIMIT 1",
            [stale_before],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((key, uuid)) = found else {
        return Ok(None);
    };
    tx.execute(
        "UPDATE analysis_job SET state = 'running', owner = ?2, heartbeat_at = ?3 \
         WHERE game_id = ?1",
        params![key, owner, now],
    )?;
    failpoint(ctx, "job.before_commit")?;
    tx.commit()?;
    Ok(Some(GameId::from_uuid(parse_uuid(&uuid)?)))
}

/// One compare-and-set update of a job this owner holds. `NotFound` for an unknown game,
/// `NotOwner` when the job is absent, not running, or owned by someone else. A repeat of
/// `complete_job` or `fail_job` by the owner that already did it is `Ok` (idempotent).
fn update_owned(
    conn: &mut Connection,
    game: GameId,
    owner: &str,
    new_state: JobState,
    now: Option<i64>,
    ctx: &Path,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, game)?;
    let changed = match now {
        Some(now) => tx.execute(
            "UPDATE analysis_job SET state = ?3, heartbeat_at = ?4 \
             WHERE game_id = ?1 AND owner = ?2 AND state = 'running'",
            params![key, owner, new_state.as_str(), now],
        )?,
        None => tx.execute(
            "UPDATE analysis_job SET state = ?3 \
             WHERE game_id = ?1 AND owner = ?2 AND state = 'running'",
            params![key, owner, new_state.as_str()],
        )?,
    };
    if changed == 0 {
        // FIND-D5R-05: a worker whose reply was lost retries the same command. The job is
        // already in the state that command asks for and this owner holds it: the answer is
        // success again and nothing is written. Any other owner, or another state, is
        // `NotOwner`. `Running` is not in the list: a heartbeat after the end stays refused.
        let repeat = matches!(new_state, JobState::Done | JobState::Failed)
            && tx
                .query_row(
                    "SELECT 1 FROM analysis_job WHERE game_id = ?1 AND owner = ?2 AND state = ?3",
                    params![key, owner, new_state.as_str()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
        return if repeat {
            Ok(())
        } else {
            Err(StoreError::NotOwner)
        };
    }
    failpoint(ctx, "job.before_commit")?;
    tx.commit()?;
    Ok(())
}

/// Refreshes the claim (the job stays `running`).
pub(crate) fn job_heartbeat(
    conn: &mut Connection,
    game: GameId,
    owner: &str,
    now: i64,
    ctx: &Path,
) -> Result<(), StoreError> {
    update_owned(conn, game, owner, JobState::Running, Some(now), ctx)
}

/// `running -> done`.
pub(crate) fn complete_job(
    conn: &mut Connection,
    game: GameId,
    owner: &str,
    ctx: &Path,
) -> Result<(), StoreError> {
    update_owned(conn, game, owner, JobState::Done, None, ctx)
}

/// `running -> failed`.
pub(crate) fn fail_job(
    conn: &mut Connection,
    game: GameId,
    owner: &str,
    ctx: &Path,
) -> Result<(), StoreError> {
    update_owned(conn, game, owner, JobState::Failed, None, ctx)
}

/// `failed -> queued` for every failed job, clearing `owner` and `heartbeat_at`. Returns how
/// many moved. "Once per start" is the caller's rule (`Store` calls it once per open).
pub(crate) fn requeue_failed(conn: &mut Connection, ctx: &Path) -> Result<u32, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let moved = tx.execute(
        "UPDATE analysis_job SET state = 'queued', owner = NULL, heartbeat_at = NULL \
         WHERE state = 'failed'",
        [],
    )?;
    failpoint(ctx, "job.before_commit")?;
    tx.commit()?;
    u32::try_from(moved).map_err(|_| StoreError::Internal)
}
