//! DD-ST-012 evaluations and variations (SDD 3.7.2 "Analysis"). One upsert per ply; evals are
//! from White's point of view (decision 12, CR-9.3), `cp_loss` from the mover's (ANA-3).
//! Variations: one saved line per row, no Player, `from_ply` = main-line plies before the
//! line's first move (decision 13, CR-9.4). Storage of imported nested variations is OPEN
//! (OQ-ST-003): this module stores one line per row and decides nothing about nesting.
//!
//! Every write ends with the failpoint `eval.before_commit` or `variation.before_commit`
//! (CT-ST-007): an error there rolls the transaction back.

use std::path::Path;

use chess_domain::{GameId, WallMs};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::api_analysis::{
    EvalFlag, GameAnalysis, JobState, MAX_VARIATION_PGN_BYTES, MoveEvalRow, VariationRecord,
};
use crate::error::StoreError;
use crate::repo::{from_i64_u32, game_key, opt_i32, parse_text, parse_uuid};

/// The failpoint call placed before each commit of part D5. `ctx` is the data directory's
/// `chess.db` path as the caller built it (a `FaultPlan` is scoped by path prefix, and a
/// connection's own path may be spelled differently, for example through a symlink). With the
/// feature off this is a no-op.
pub(crate) fn failpoint(ctx: &Path, name: &'static str) -> Result<(), StoreError> {
    crate::fault::check(name, ctx).map_err(StoreError::from)
}

/// Inserts or replaces the evaluation of one ply. `NotFound` for an unknown game;
/// `InvariantViolated` when both `cp` and `mate` are set, or when the game has no stored move at
/// `ply` (the foreign key).
pub(crate) fn upsert_move_eval(
    conn: &mut Connection,
    eval: &MoveEvalRow,
    ctx: &Path,
) -> Result<(), StoreError> {
    if eval.cp.is_some() && eval.mate.is_some() {
        return Err(StoreError::InvariantViolated);
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, eval.game)?;
    tx.execute(
        "INSERT INTO move_eval (game_id, ply, cp, mate, best_uci, cp_loss, flag, depth, \
         engine_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
         ON CONFLICT (game_id, ply) DO UPDATE SET cp = excluded.cp, mate = excluded.mate, \
         best_uci = excluded.best_uci, cp_loss = excluded.cp_loss, flag = excluded.flag, \
         depth = excluded.depth, engine_version = excluded.engine_version",
        params![
            key,
            i64::from(eval.ply),
            eval.cp,
            eval.mate,
            eval.best_uci,
            eval.cp_loss,
            eval.flag.map(EvalFlag::as_str),
            eval.depth,
            eval.engine_version,
        ],
    )?;
    failpoint(ctx, "eval.before_commit")?;
    tx.commit()?;
    Ok(())
}

/// The stored evaluations (ply order), the job state, `evaluated` and `total`, all from one
/// read snapshot. `NotFound` for an unknown game.
pub(crate) fn get_game_analysis(
    conn: &Connection,
    game: GameId,
) -> Result<GameAnalysis, StoreError> {
    let tx = conn.unchecked_transaction()?;
    let key = game_key(&tx, game)?;
    let mut statement = tx.prepare(
        "SELECT ply, cp, mate, best_uci, cp_loss, flag, depth, engine_version \
         FROM move_eval WHERE game_id = ?1 ORDER BY ply",
    )?;
    let rows = statement.query_map([key], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, Option<i64>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<i64>>(6)?,
            row.get::<_, String>(7)?,
        ))
    })?;
    let mut evals = Vec::new();
    for item in rows {
        let (ply, cp, mate, best_uci, cp_loss, flag, depth, engine_version) = item?;
        evals.push(MoveEvalRow {
            game,
            ply: from_i64_u32(ply)?,
            cp: opt_i32(cp)?,
            mate: opt_i32(mate)?,
            best_uci,
            cp_loss: opt_i32(cp_loss)?,
            flag: flag.as_deref().map(parse_text::<EvalFlag>).transpose()?,
            depth: depth.map(from_i64_u32).transpose()?,
            engine_version,
        });
    }
    drop(statement);
    let job = tx
        .query_row(
            "SELECT state FROM analysis_job WHERE game_id = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .as_deref()
        .map(parse_text::<JobState>)
        .transpose()?;
    let total: i64 = tx.query_row(
        "SELECT count(*) FROM move WHERE game_id = ?1",
        [key],
        |row| row.get(0),
    )?;
    let evaluated = u32::try_from(evals.len()).map_err(|_| StoreError::Corrupt)?;
    Ok(GameAnalysis {
        evals,
        job,
        evaluated,
        total: from_i64_u32(total)?,
    })
}

/// Stores one variation line, identified by `(game, from_ply, pgn)` (FIND-D5-02 B). In one
/// immediate transaction: an unknown game is `NotFound`; an empty text is `InvariantViolated`; a
/// text above `MAX_VARIATION_PGN_BYTES` is `VariationTooLarge` (checked first, before the game); a
/// stored key is returned as it is, before any other check (a repeat never fails, and nothing is
/// written); a new key with `from_ply` at or beyond the game's stored ply count (a branch from
/// the final position) is `VariationAnchor` (decision 13: the backstop behind the app's own
/// check; the count is the stored `MOVE` rows, so plies still in the journal backlog are not
/// counted); otherwise the row is inserted. No UNIQUE index: the immediate transaction is the
/// guarantee (a unique index would be a stored-format change).
pub(crate) fn save_variation(
    conn: &mut Connection,
    game: GameId,
    from_ply: u32,
    pgn: &str,
    now: i64,
    ctx: &Path,
) -> Result<VariationRecord, StoreError> {
    if pgn.is_empty() {
        return Err(StoreError::InvariantViolated);
    }
    if pgn.len() > MAX_VARIATION_PGN_BYTES {
        return Err(StoreError::VariationTooLarge {
            max_bytes: MAX_VARIATION_PGN_BYTES,
        });
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, game)?;
    let stored: Option<i64> = tx
        .query_row(
            "SELECT created_at FROM variation WHERE game_id = ?1 AND from_ply = ?2 AND pgn = ?3 \
             ORDER BY id LIMIT 1",
            params![key, i64::from(from_ply), pgn],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(created_at) = stored {
        return Ok(VariationRecord {
            game,
            from_ply,
            pgn: pgn.to_owned(),
            created_at: WallMs(created_at),
        });
    }
    let plies: i64 = tx.query_row(
        "SELECT count(*) FROM move WHERE game_id = ?1",
        [key],
        |row| row.get(0),
    )?;
    if i64::from(from_ply) >= plies {
        return Err(StoreError::VariationAnchor);
    }
    tx.execute(
        "INSERT INTO variation (game_id, from_ply, pgn, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![key, i64::from(from_ply), pgn, now],
    )?;
    failpoint(ctx, "variation.before_commit")?;
    tx.commit()?;
    Ok(VariationRecord {
        game,
        from_ply,
        pgn: pgn.to_owned(),
        created_at: WallMs(now),
    })
}

/// Deletes the row with this key. `NotFound` for an unknown game; true when a row was deleted,
/// false when none matched (idempotent, like `lease_delete`).
pub(crate) fn delete_variation(
    conn: &mut Connection,
    game: GameId,
    from_ply: u32,
    pgn: &str,
    ctx: &Path,
) -> Result<bool, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, game)?;
    let changed = tx.execute(
        "DELETE FROM variation WHERE game_id = ?1 AND from_ply = ?2 AND pgn = ?3",
        params![key, i64::from(from_ply), pgn],
    )?;
    failpoint(ctx, "variation.before_commit")?;
    tx.commit()?;
    Ok(changed > 0)
}

/// The variations of a game, by `from_ply` then creation order. `NotFound` for an unknown game.
pub(crate) fn list_variations(
    conn: &Connection,
    game: GameId,
) -> Result<Vec<VariationRecord>, StoreError> {
    let key = game_key(conn, game)?;
    let mut statement = conn.prepare(
        "SELECT v.id, g.uuid, v.from_ply, v.pgn, v.created_at \
         FROM variation v JOIN game g ON g.id = v.game_id \
         WHERE v.game_id = ?1 ORDER BY v.from_ply, v.id",
    )?;
    let rows = statement.query_map([key], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    let mut out = Vec::new();
    for item in rows {
        let (_id, uuid, from_ply, pgn, created_at) = item?;
        out.push(VariationRecord {
            game: GameId::from_uuid(parse_uuid(&uuid)?),
            from_ply: from_i64_u32(from_ply)?,
            pgn,
            created_at: WallMs(created_at),
        });
    }
    Ok(out)
}
