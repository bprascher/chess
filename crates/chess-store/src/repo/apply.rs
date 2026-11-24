//! DD-ST-009 / DD-ST-018 apply path (part D3): `apply_records` (SDD 3.7.2, 3.11.1, 3.11.4) and
//! the applier of a `ReconcilePlan` (SDD 3.11.3). Repository functions over `&mut Connection`:
//! no thread, no timer, no channel (SDD 3.8). The backlog and its retry timer live in
//! `backlog.rs` and `thread.rs`; this module is what one attempt does.
//!
//! **Atomic.** One `BEGIN IMMEDIATE` per call: the rows, the clocks and `journal_seq` of the whole
//! batch commit together or not at all (a typed error rolls the batch back; nothing half is
//! written). **Idempotent.** Applying a batch (or a plan) a second time leaves every table as the
//! first time left it, so a crash after the commit and a retry are harmless (CT-ST-007).
//!
//! **Per kind** (SDD 3.11.1 "DB effect when applied"):
//! - `Start`: nothing if the `GAME` row exists; otherwise the row is created from the payload
//!   (a restore), `started_at` = the record's `wall_ms` (SF-16). No slot rows: restore inserts
//!   them after the finalize pass (SDD 3.11.10, R-ST-7).
//! - `Move`: upsert of the `MOVE` row at **journal ply + 1** (D-92: the journal counts plies
//!   already played, `MOVE.ply` is one-based), then the `GAME` clock columns.
//! - `Takeback`: delete the `MOVE` rows above `to_ply` (no conversion: `to_ply` is the number of
//!   plies kept).
//! - `Clock`: the `GAME` clock columns.
//! - `GameEnd`, `Abort`, `Voided`: the D2 transactions (`finalize_in_tx`, `abort_in_tx`,
//!   `void_game`), so a live finalize and a finalize from the journal alone are the same code
//!   (UT-ST-082).
//! - every other kind changes no row.
//!
//! **D-87.** A `GameEnd` that is followed by a `Voided` in the same batch is not finalized: the
//! journal is a voided journal and no rating is settled for it.

use std::path::Path;
use std::time::Duration;

use chess_domain::{ClockSpec, GameId, GameResult, Matchup, Mode, PlayerId, PlayerKind, WallMs};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::api::{AbortRequest, FinalizeRequest};
use crate::error::StoreError;
use crate::journal::format::{
    AbortPayload, ClockPayload, GameEndPayload, JournalRecord, MovePayload, Payload, SeatKind,
    StartPayload, VoidReason, VoidedPayload, matchup_code,
};
use crate::journal::reconcile::{Plan, StoredMove};
use crate::repo::finalize::{abort_in_tx, finalize_in_tx};
use crate::repo::lifecycle::{
    TERMINATION_VOIDED_BY_RESET, TERMINATION_VOIDED_UNREADABLE, VoidFields, fault_check,
    game_status, void_game,
};
use crate::repo::{game_key, parse_text, player_key, to_i64, uuid_text};

/// `busy_timeout` of one apply attempt (SDD 3.12): the store thread waits at most this long for
/// the database lock, and the backlog retries later. The connection's normal value (5 s) comes
/// back after the attempt.
pub(crate) const APPLY_BUSY_TIMEOUT: Duration = Duration::from_millis(100);

/// Runs `f` with the short busy timeout and restores the previous value, also after an error.
fn with_short_busy<T>(
    conn: &mut Connection,
    f: impl FnOnce(&mut Connection) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let previous: i64 = conn.pragma_query_value(None, "busy_timeout", |row| row.get(0))?;
    conn.busy_timeout(APPLY_BUSY_TIMEOUT)?;
    let result = f(conn);
    let restored = conn.busy_timeout(Duration::from_millis(u64::try_from(previous).unwrap_or(0)));
    if restored.is_err() {
        tracing::error!("the busy timeout of the write connection could not be restored");
    }
    result
}

/// `apply_records(game, records)`: one transaction for the batch (SDD 3.7.2). Records must be in
/// strictly increasing `seq` order; a batch that is not is a caller bug (`InvariantViolated`).
/// An empty batch changes nothing.
pub(crate) fn apply_records(
    conn: &mut Connection,
    game: GameId,
    records: &[JournalRecord],
    ctx: &Path,
) -> Result<(), StoreError> {
    let Some(last) = records.last() else {
        return Ok(());
    };
    if records.windows(2).any(|pair| {
        pair.first()
            .zip(pair.get(1))
            .is_none_or(|(a, b)| a.seq >= b.seq)
    }) {
        return Err(StoreError::InvariantViolated);
    }
    let last_seq = last.seq;
    with_short_busy(conn, |conn| {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(
            first @ JournalRecord {
                payload: Payload::Start(_),
                ..
            },
        ) = records.first()
        {
            ensure_start(&tx, game, first)?;
        }
        let key = game_key(&tx, game)?;
        let before: i64 =
            tx.query_row("SELECT journal_seq FROM game WHERE id = ?1", [key], |r| {
                r.get(0)
            })?;
        // D-87: a `GameEnd` with a `Voided` after it in this batch is superseded by the void.
        let void_at = records
            .iter()
            .position(|r| matches!(r.payload, Payload::Voided(_)));
        for (index, record) in records.iter().enumerate() {
            let superseded = void_at.is_some_and(|v| index < v);
            apply_one(&tx, game, key, record, superseded)?;
        }
        // Monotone: a batch never lowers the sequence the row already carries (D-66 compares it
        // with the journal). Only the plan applier may set it lower (rebase step 2).
        let target = before.max(to_i64(last_seq)?);
        tx.execute(
            "UPDATE game SET journal_seq = ?2 WHERE id = ?1",
            params![key, target],
        )?;
        fault_check("apply.before_commit", ctx)?;
        tx.commit()?;
        fault_check("apply.after_commit", ctx)
    })
}

fn apply_one(
    tx: &Connection,
    game: GameId,
    key: i64,
    record: &JournalRecord,
    superseded_end: bool,
) -> Result<(), StoreError> {
    // D3 ruling 2026-11-24: an end time at or below zero is refused for every terminal record,
    // also for one that is superseded (D-87) or meets an already terminal row. The format
    // refuses it earlier; this is the second line for a record built in memory or a hostile file.
    if let Payload::GameEnd(GameEndPayload { ended_at, .. })
    | Payload::Abort(AbortPayload { ended_at, .. })
    | Payload::Voided(VoidedPayload { ended_at, .. }) = &record.payload
        && *ended_at <= 0
    {
        return Err(StoreError::InvariantViolated);
    }
    match &record.payload {
        Payload::Start(payload) => {
            // The row exists by now; a second `Start` is a no-op, but it must name this game
            // and pass the same checks as the first.
            if payload.game.0 != game.as_uuid() {
                return Err(StoreError::InvariantViolated);
            }
            check_start(payload, record.wall_ms)
        }
        Payload::Move(payload) => write_move(tx, key, payload, record.wall_ms),
        Payload::Takeback(payload) => delete_moves_above(tx, key, i64::from(payload.to_ply)),
        Payload::Clock(ClockPayload {
            white_ms, black_ms, ..
        }) => set_clocks(tx, key, *white_ms, *black_ms),
        Payload::GameEnd(payload) if !superseded_end => {
            apply_game_end(tx, game, payload, record.seq)
        }
        Payload::Abort(payload) => {
            abort_in_tx(
                tx,
                &AbortRequest {
                    game,
                    ended_at: WallMs(payload.ended_at),
                    content_hash: payload.content_hash.clone(),
                    final_journal_seq: record.seq,
                },
            )?;
            Ok(())
        }
        Payload::Voided(payload) => apply_voided(tx, key, payload),
        // A superseded `GameEnd` and the kinds that change no row (`DrawOffer`, `DrawDeclined`,
        // `DrawDropped`, `Pause`, `Resume`, `Suspend`, `SuspendResume`).
        Payload::GameEnd(_)
        | Payload::DrawOffer(_)
        | Payload::DrawDeclined(_)
        | Payload::DrawDropped(_)
        | Payload::Pause
        | Payload::Resume
        | Payload::Suspend(_)
        | Payload::SuspendResume => Ok(()),
    }
}

// ------------------------------------------------------------------------------------ Start

/// What the journal format already refuses and the store refuses again (a record built in
/// memory, a hostile file): `wall_ms` becomes `GAME.started_at` and must be above zero
/// (FIND-D3R-07), and `matchup` must be the code of the two kinds (D3-06 ruling 2026-11-24).
fn check_start(payload: &StartPayload, wall_ms: i64) -> Result<(), StoreError> {
    if wall_ms <= 0 || payload.matchup != matchup_code(payload.white_kind, payload.black_kind) {
        return Err(StoreError::InvariantViolated);
    }
    Ok(())
}

/// Creates the `GAME` row from a `Start` record when it is absent (a restore). The fields
/// `rated` and `self_play` are the frozen history of the record and are never recomputed
/// (D-64). Both Players are matched by uuid and must exist (`NotFound` otherwise); their kinds
/// must agree with the record's. The `matchup` column is derived from the two kinds; the
/// payload's text must equal that code (`check_start`).
fn ensure_start(tx: &Connection, game: GameId, record: &JournalRecord) -> Result<(), StoreError> {
    let Payload::Start(payload) = &record.payload else {
        return Err(StoreError::InvariantViolated);
    };
    if payload.game.0 != game.as_uuid() {
        return Err(StoreError::InvariantViolated);
    }
    check_start(payload, record.wall_ms)?;
    let exists = tx
        .query_row(
            "SELECT 1 FROM game WHERE uuid = ?1",
            [uuid_text(game.as_uuid())],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        return Ok(());
    }
    insert_game(tx, payload, record)
}

fn seat_kind(kind: SeatKind) -> PlayerKind {
    match kind {
        SeatKind::Human => PlayerKind::Human,
        SeatKind::Computer => PlayerKind::Computer,
    }
}

fn stored_kind(tx: &Connection, key: i64) -> Result<PlayerKind, StoreError> {
    let text: String =
        tx.query_row("SELECT kind FROM player WHERE id = ?1", [key], |r| r.get(0))?;
    parse_text(&text)
}

fn insert_game(
    tx: &Connection,
    payload: &StartPayload,
    record: &JournalRecord,
) -> Result<(), StoreError> {
    let white = player_key(tx, PlayerId::from_uuid(payload.white.0))?;
    let black = player_key(tx, PlayerId::from_uuid(payload.black.0))?;
    let (white_kind, black_kind) = (seat_kind(payload.white_kind), seat_kind(payload.black_kind));
    if stored_kind(tx, white)? != white_kind || stored_kind(tx, black)? != black_kind {
        return Err(StoreError::InvariantViolated);
    }
    let matchup = match (white_kind, black_kind) {
        (PlayerKind::Human, PlayerKind::Human) => Matchup::HumanHuman,
        (PlayerKind::Computer, PlayerKind::Computer) => Matchup::ComputerComputer,
        _ => Matchup::HumanComputer,
    };
    let mode: Mode = parse_text(&payload.mode)?;
    let clock = payload
        .clock_spec
        .as_ref()
        .map(|c| ClockSpec::from_spec_text(c.as_str()))
        .transpose()
        .map_err(|_| StoreError::InvariantViolated)?;
    let base_ms = clock.map(|c| to_i64(c.base_ms())).transpose()?;
    tx.execute(
        "INSERT INTO game (uuid, white_id, black_id, matchup, mode, rated, self_play, \
         custom_start, start_fen, clock_spec, white_clock_ms, black_clock_ms, status, started_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 'in_progress', ?12)",
        params![
            uuid_text(payload.game.0),
            white,
            black,
            matchup.as_str(),
            mode.as_str(),
            i64::from(payload.rated),
            i64::from(payload.self_play),
            i64::from(payload.custom_start),
            payload.start_fen,
            clock.map(ClockSpec::to_spec_text),
            base_ms,
            record.wall_ms,
        ],
    )?;
    Ok(())
}

// ------------------------------------------------------------------- Move, Takeback, Clock

fn opt_ms(value: Option<u64>) -> Result<Option<i64>, StoreError> {
    value.map(to_i64).transpose()
}

/// `MOVE.ply = journal ply + 1` (D-92, SDD 3.11.3 "Ply base"). The `+ 1` lives here and only
/// here: the codec does not add it. The journal wins: an existing row at the ply is replaced.
fn write_move(
    tx: &Connection,
    key: i64,
    payload: &MovePayload,
    played_at: i64,
) -> Result<(), StoreError> {
    let ply = payload
        .ply
        .checked_add(1)
        .ok_or(StoreError::InvariantViolated)?;
    let (white, black) = (opt_ms(payload.white_ms)?, opt_ms(payload.black_ms)?);
    upsert_move_row(
        tx,
        key,
        &StoredMove {
            ply,
            uci: payload.uci.clone(),
            san: payload.san.clone(),
            fen_after: payload.fen_after.clone(),
            think_ms: Some(payload.think_ms),
            clocks: payload.white_ms.zip(payload.black_ms),
            played_at: Some(played_at),
        },
    )?;
    // A half-null pair is refused by the table CHECK (InvariantViolated), as in the row above.
    tx.execute(
        "UPDATE game SET white_clock_ms = ?2, black_clock_ms = ?3 WHERE id = ?1",
        params![key, white, black],
    )?;
    Ok(())
}

fn upsert_move_row(tx: &Connection, key: i64, row: &StoredMove) -> Result<(), StoreError> {
    let (white, black) = match row.clocks {
        Some((w, b)) => (Some(to_i64(w)?), Some(to_i64(b)?)),
        None => (None, None),
    };
    tx.execute(
        "INSERT INTO move (game_id, ply, san, uci, fen_after, think_ms, white_clock_ms, \
         black_clock_ms, played_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
         ON CONFLICT (game_id, ply) DO UPDATE SET san = excluded.san, uci = excluded.uci, \
         fen_after = excluded.fen_after, think_ms = excluded.think_ms, \
         white_clock_ms = excluded.white_clock_ms, black_clock_ms = excluded.black_clock_ms, \
         played_at = excluded.played_at",
        params![
            key,
            row.ply,
            row.san,
            row.uci,
            row.fen_after,
            opt_ms(row.think_ms)?,
            white,
            black,
            row.played_at
        ],
    )?;
    Ok(())
}

/// Deletes the rows above `ply` and the stored evaluations of those plies: a `move_eval` row
/// names its `MOVE` row by a foreign key, so it cannot outlive it (FIND-D3-04).
fn delete_moves_above(tx: &Connection, key: i64, ply: i64) -> Result<(), StoreError> {
    tx.execute(
        "DELETE FROM move_eval WHERE game_id = ?1 AND ply > ?2",
        params![key, ply],
    )?;
    tx.execute(
        "DELETE FROM move WHERE game_id = ?1 AND ply > ?2",
        params![key, ply],
    )?;
    Ok(())
}

fn set_clocks(
    tx: &Connection,
    key: i64,
    white: Option<u64>,
    black: Option<u64>,
) -> Result<(), StoreError> {
    tx.execute(
        "UPDATE game SET white_clock_ms = ?2, black_clock_ms = ?3 WHERE id = ?1",
        params![key, opt_ms(white)?, opt_ms(black)?],
    )?;
    Ok(())
}

// ------------------------------------------------------------- GameEnd, Abort and Voided

fn apply_game_end(
    tx: &Connection,
    game: GameId,
    payload: &GameEndPayload,
    seq: u64,
) -> Result<(), StoreError> {
    let result: GameResult =
        parse_text(&payload.result).map_err(|_| StoreError::InvariantViolated)?;
    // SF-15: the journal code maps one to one to the unchanged `GAME.termination` text.
    let reason = chess_domain::EndReason::from_journal_code(payload.reason.as_str())
        .map_err(|_| StoreError::InvariantViolated)?;
    finalize_in_tx(
        tx,
        &FinalizeRequest {
            game,
            result,
            termination: reason.termination_text().to_owned(),
            ended_at: WallMs(payload.ended_at),
            white_time_used_ms: payload.white_time_used_ms,
            black_time_used_ms: payload.black_time_used_ms,
            total_active_ms: payload.total_active_ms,
            eco: payload.eco.clone(),
            opening_name: payload.opening_name.clone(),
            analysis_used: payload.analysis_used,
            content_hash: payload.content_hash.clone(),
            final_journal_seq: seq,
        },
    )?;
    Ok(())
}

/// A `Voided` record (D-69, SF-06; SDD 3.11.4). The sequence number is set by the caller of this
/// function for every kind. Only a row that is not yet `voided` is voided; an already voided row
/// is left as the live void wrote it.
///
/// **What is refused, and why (D-87, FIND-D2R-05, FIND-F2-09).** A `completed` row with settled
/// ratings is refused by `void_game` (`InvariantViolated`, nothing written): a void changes no
/// rating (D-30), so a record must never turn a settled game into a "void" that still counts.
/// D-87 does not ask for that: a pair `GameEnd`, `Voided` is a voided journal whose `GameEnd` is
/// never finalized (it is skipped above, so no rating is settled), and the pair cannot arise from
/// a row whose ratings were settled by a live finalize, because `reset_player` voids only
/// `in_progress` games. An `aborted` or
/// `imported` row is refused too: the domain accepts `Voided` only from `in_progress` and
/// `completed` (T18, D-87 last sentence).
fn apply_voided(tx: &Connection, key: i64, payload: &VoidedPayload) -> Result<(), StoreError> {
    match game_status(tx, key)?.as_str() {
        "voided" => Ok(()),
        "in_progress" | "completed" => {
            let voided_by = payload
                .reset_player
                .map(|p| player_key(tx, PlayerId::from_uuid(p.0)))
                .transpose()?;
            let termination = match payload.reason {
                VoidReason::Reset => TERMINATION_VOIDED_BY_RESET,
                VoidReason::Unreadable => TERMINATION_VOIDED_UNREADABLE,
            };
            void_game(
                tx,
                key,
                &VoidFields {
                    termination,
                    voided_by,
                    ended_at: payload.ended_at,
                    content_hash: &payload.content_hash,
                },
            )
        }
        _ => Err(StoreError::InvariantViolated),
    }
}

// ------------------------------------------------------------------ the reconcile applier

/// Applies a `ReconcilePlan` in one transaction (SDD 3.11.3): delete every stored row from the
/// plan's `delete_from()` (and the evaluations of those plies), upsert `rows_to_write` (their
/// `ply` is already one-based: the plan is built from the fold, which added the `+ 1`), set the
/// clocks if the plan has some, and set `journal_seq` **exactly** to `journal_seq` when given
/// (the rebase sets the new file's last `seq`, which can be lower than the old value; `None`
/// leaves it). Idempotent: the plan applied twice leaves what it left once.
pub(crate) fn apply_reconcile_plan(
    conn: &mut Connection,
    game: GameId,
    plan: &Plan,
    journal_seq: Option<u64>,
    ctx: &Path,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, game)?;
    if let Some(from) = plan.delete_from() {
        // Rows from `from` on go: ply `from - 1` is the last one kept.
        delete_moves_above(&tx, key, i64::from(from).saturating_sub(1))?;
    }
    for row in &plan.rows_to_write {
        upsert_move_row(&tx, key, row)?;
    }
    if let Some(clocks) = plan.clocks {
        let (white, black) = match clocks {
            Some((w, b)) => (Some(w), Some(b)),
            None => (None, None),
        };
        set_clocks(&tx, key, white, black)?;
    }
    if let Some(seq) = journal_seq {
        tx.execute(
            "UPDATE game SET journal_seq = ?2 WHERE id = ?1",
            params![key, to_i64(seq)?],
        )?;
    }
    fault_check("apply.before_commit", ctx)?;
    tx.commit()?;
    fault_check("apply.after_commit", ctx)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;

    use proptest::prelude::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;
    use crate::journal::reconcile::{
        Folded, ReconcileInput, RunSource, apply_plan, fold, reconcile,
    };

    const GAME: i64 = 1;

    fn game_id() -> GameId {
        GameId::from_uuid(Uuid::from_u128(1))
    }

    /// A file database with the schema, two humans and one `in_progress` game (key 1).
    fn world(dir: &TempDir) -> (Connection, PathBuf) {
        let path = dir.path().join("chess.db");
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn.execute_batch(crate::migrate::MIGRATIONS[0].sql)
            .unwrap();
        for (id, name) in [(1, "A"), (2, "B")] {
            conn.execute(
                "INSERT INTO player (id, uuid, kind, name, start_category, strict_elo, \
                 training_elo, created_at) VALUES (?1, ?2, 'human', ?3, 'novice', 1200, 1200, 0)",
                params![id, format!("00000000-0000-0000-0000-{id:012}"), name],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO game (id, uuid, white_id, black_id, matchup, mode, rated, self_play, \
             status, started_at, white_clock_ms, black_clock_ms, journal_seq) \
             VALUES (1, ?1, 1, 2, 'hh', 'strict', 1, 0, 'in_progress', 5, 100, 200, 9)",
            [uuid_text(Uuid::from_u128(1))],
        )
        .unwrap();
        (conn, path)
    }

    fn sm(ply: u32, uci: &str) -> StoredMove {
        StoredMove {
            ply,
            uci: uci.into(),
            san: format!("s-{uci}"),
            fen_after: format!("f-{ply}-{uci}"),
            think_ms: Some(5),
            clocks: Some((10, 20)),
            played_at: Some(77),
        }
    }

    fn seed(conn: &Connection, rows: &[StoredMove]) {
        for row in rows {
            upsert_move_row(conn, GAME, row).unwrap();
        }
    }

    fn stored(conn: &Connection) -> Vec<StoredMove> {
        let mut st = conn
            .prepare(
                "SELECT ply, uci, san, fen_after, think_ms, white_clock_ms, black_clock_ms, \
                 played_at FROM move WHERE game_id = 1 ORDER BY ply",
            )
            .unwrap();
        st.query_map([], |r| {
            let white: Option<i64> = r.get(5)?;
            let black: Option<i64> = r.get(6)?;
            let think: Option<i64> = r.get(4)?;
            Ok(StoredMove {
                ply: r.get(0)?,
                uci: r.get(1)?,
                san: r.get(2)?,
                fen_after: r.get(3)?,
                think_ms: think.map(|v| u64::try_from(v).unwrap()),
                clocks: white
                    .zip(black)
                    .map(|(w, b)| (u64::try_from(w).unwrap(), u64::try_from(b).unwrap())),
                played_at: r.get(7)?,
            })
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
    }

    fn game_cols(conn: &Connection) -> (Option<i64>, Option<i64>, i64) {
        conn.query_row(
            "SELECT white_clock_ms, black_clock_ms, journal_seq FROM game WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    }

    fn plan(from: Option<u32>, write: Vec<StoredMove>, clocks: Option<ClockPair>) -> Plan {
        Plan {
            replace_from: from,
            rows_to_delete_from: None,
            rows_to_write: write,
            clocks,
            source: RunSource::Journal,
            damaged: false,
        }
    }

    type ClockPair = Option<(u64, u64)>;

    /// The plan writes its rows at the ply it names (no second `+ 1`: the fold added it), deletes
    /// from `delete_from()`, sets the clocks and sets `journal_seq` exactly, also lower (rebase
    /// step 2).
    #[test]
    fn the_plan_applier_writes_deletes_sets_clocks_and_sets_journal_seq_exactly() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        seed(&conn, &[sm(1, "a"), sm(2, "b"), sm(3, "c"), sm(4, "d")]);
        let p = plan(
            Some(3),
            vec![sm(3, "C"), sm(4, "D"), sm(5, "E")],
            Some(Some((1, 2))),
        );
        apply_reconcile_plan(&mut conn, game_id(), &p, Some(4), &path).unwrap();
        let rows = stored(&conn);
        assert_eq!(
            rows.iter()
                .map(|r| (r.ply, r.uci.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "a"), (2, "b"), (3, "C"), (4, "D"), (5, "E")]
        );
        assert_eq!(
            game_cols(&conn),
            (Some(1), Some(2), 4),
            "journal_seq 9 -> 4"
        );
    }

    #[test]
    fn the_plan_applier_leaves_what_the_plan_does_not_name() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        seed(&conn, &[sm(1, "a"), sm(2, "b")]);
        apply_reconcile_plan(&mut conn, game_id(), &plan(None, vec![], None), None, &path).unwrap();
        assert_eq!(stored(&conn).len(), 2);
        assert_eq!(
            game_cols(&conn),
            (Some(100), Some(200), 9),
            "clocks and seq kept"
        );
        // `Some(None)` is an untimed game's clocks: both columns become NULL.
        apply_reconcile_plan(
            &mut conn,
            game_id(),
            &plan(None, vec![], Some(None)),
            None,
            &path,
        )
        .unwrap();
        assert_eq!(game_cols(&conn), (None, None, 9));
    }

    /// `rows_to_delete_from` alone (the journal is shorter than the store) deletes from there.
    #[test]
    fn the_plan_applier_deletes_rows_beyond_the_journal() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        seed(&conn, &[sm(1, "a"), sm(2, "b"), sm(3, "c")]);
        let mut p = plan(None, vec![], None);
        p.rows_to_delete_from = Some(3);
        apply_reconcile_plan(&mut conn, game_id(), &p, None, &path).unwrap();
        assert_eq!(stored(&conn).len(), 2);
    }

    /// Applying a plan twice leaves what applying it once left (acceptance: idempotence).
    #[test]
    fn the_plan_applied_twice_equals_the_plan_applied_once() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        seed(&conn, &[sm(1, "a"), sm(2, "b"), sm(3, "c")]);
        let p = plan(Some(2), vec![sm(2, "B"), sm(3, "C")], Some(Some((3, 4))));
        apply_reconcile_plan(&mut conn, game_id(), &p, Some(6), &path).unwrap();
        let once = (stored(&conn), game_cols(&conn));
        apply_reconcile_plan(&mut conn, game_id(), &p, Some(6), &path).unwrap();
        assert_eq!(once, (stored(&conn), game_cols(&conn)));
    }

    /// An evaluation names its move by a foreign key. A removed ply takes its evaluation with it
    /// (FIND-D3-04); a kept ply keeps its evaluation.
    #[test]
    fn deleted_plies_take_their_evaluations_and_kept_plies_keep_them() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        seed(&conn, &[sm(1, "a"), sm(2, "b"), sm(3, "c")]);
        for ply in 1..=3 {
            conn.execute(
                "INSERT INTO move_eval (game_id, ply, cp, engine_version) VALUES (1, ?1, 10, 'sf')",
                [ply],
            )
            .unwrap();
        }
        let mut p = plan(None, vec![], None);
        p.rows_to_delete_from = Some(3);
        apply_reconcile_plan(&mut conn, game_id(), &p, None, &path).unwrap();
        let evals: Vec<i64> = conn
            .prepare("SELECT ply FROM move_eval ORDER BY ply")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(evals, vec![1, 2]);
    }

    #[test]
    fn a_plan_for_an_unknown_game_is_not_found_and_a_bad_ply_is_a_typed_error() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        let other = GameId::from_uuid(Uuid::from_u128(99));
        assert!(matches!(
            apply_reconcile_plan(&mut conn, other, &plan(None, vec![], None), None, &path),
            Err(StoreError::NotFound { .. })
        ));
        // MOVE.ply >= 1: ply 0 is refused by the table and nothing else of the plan is written.
        let bad = plan(Some(1), vec![sm(1, "x"), sm(0, "y")], Some(Some((1, 1))));
        seed(&conn, &[sm(1, "a")]);
        assert!(matches!(
            apply_reconcile_plan(&mut conn, game_id(), &bad, Some(1), &path),
            Err(StoreError::InvariantViolated)
        ));
        assert_eq!(stored(&conn)[0].uci, "a", "rolled back");
        assert_eq!(game_cols(&conn), (Some(100), Some(200), 9));
    }

    fn model_rows(rows: &[StoredMove]) -> Vec<(u32, String)> {
        rows.iter().map(|r| (r.ply, r.uci.clone())).collect()
    }

    proptest! {
        /// The SQL applier equals the model `apply_plan` of `reconcile.rs` for any stored rows
        /// and any journal (also with gaps in the stored plies), and reconciling the result
        /// again gives an empty plan.
        #[test]
        fn the_sql_applier_equals_the_model_and_leaves_an_empty_plan(
            stored_plies in proptest::collection::btree_set(1u32..8, 0..7),
            journal_len in 0u32..8,
            flip in proptest::collection::btree_set(1u32..8, 0..4),
            scan_damaged in any::<bool>(),
            db_seq in 0u64..6,
            last in 0u64..6,
        ) {
            let dir = TempDir::new().unwrap();
            let (mut conn, path) = world(&dir);
            let rows: Vec<StoredMove> = stored_plies
                .iter()
                .map(|p| sm(*p, if flip.contains(p) { "x" } else { "m" }))
                .collect();
            seed(&conn, &rows);
            let journal = Folded {
                moves: (1..=journal_len).map(|p| (p, sm(p, "m"))).collect(),
                clocks: Some(Some((10, 20))),
            };
            let stored_clocks = Some((100u64, 200u64));
            let p = reconcile(&ReconcileInput {
                journal: &journal,
                stored: &rows,
                scan_damaged,
                db_seq,
                last_intact_seq: last,
                stored_clocks,
            });
            let (model, model_clocks) = apply_plan(&rows, stored_clocks, &p);
            apply_reconcile_plan(&mut conn, game_id(), &p, None, &path).unwrap();
            let after = stored(&conn);
            prop_assert_eq!(model_rows(&after), model_rows(&model));
            let (w, b, _) = game_cols(&conn);
            prop_assert_eq!(w.zip(b).map(|(w, b)| (u64::try_from(w).unwrap(), u64::try_from(b).unwrap())), model_clocks);
            apply_reconcile_plan(&mut conn, game_id(), &p, None, &path).unwrap();
            prop_assert_eq!(model_rows(&stored(&conn)), model_rows(&model));
            let again = reconcile(&ReconcileInput {
                journal: &journal,
                stored: &after,
                scan_damaged,
                db_seq,
                last_intact_seq: last,
                stored_clocks: model_clocks,
            });
            prop_assert!(again.is_empty(), "{again:?}");
        }
    }

    /// `apply_records` waits at most 100 ms for a lock another connection holds (SDD 3.12) and
    /// puts the connection's 5 s back afterwards, so the store thread is never held for 5 s by
    /// an apply attempt (part D3 acceptance).
    #[test]
    fn an_apply_attempt_waits_at_most_100_ms_for_the_lock_and_restores_the_5_s() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        let other = Connection::open(&path).unwrap();
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        let record = JournalRecord {
            seq: 10,
            wall_ms: 1,
            payload: Payload::Clock(ClockPayload {
                white_ms: Some(1),
                black_ms: Some(2),
                running_side: None,
            }),
        };
        let begun = Instant::now();
        let result = apply_records(&mut conn, game_id(), &[record], &path);
        let waited = begun.elapsed();
        assert!(matches!(result, Err(StoreError::Busy)), "{result:?}");
        assert!(waited >= Duration::from_millis(90), "{waited:?}");
        assert!(waited < Duration::from_millis(1_500), "{waited:?}");
        let timeout: i64 = conn
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
            .unwrap();
        assert_eq!(timeout, 5_000, "restored after a failed attempt");
        other.execute_batch("ROLLBACK").unwrap();
    }

    /// Also restored after a success (the early return and the normal return take different
    /// paths through the helper).
    #[test]
    fn the_busy_timeout_is_restored_after_a_success_too() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        apply_records(&mut conn, game_id(), &[], &path).unwrap();
        let record = JournalRecord {
            seq: 10,
            wall_ms: 1,
            payload: Payload::Pause,
        };
        apply_records(&mut conn, game_id(), &[record], &path).unwrap();
        let timeout: i64 = conn
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
            .unwrap();
        assert_eq!(timeout, 5_000);
        assert_eq!(game_cols(&conn).2, 10, "journal_seq 9 -> 10");
    }

    #[cfg(feature = "test-support")]
    mod faults {
        use super::*;
        use crate::fault::FaultPlan;

        /// CT-ST-007 for the plan applier: a fault before the commit leaves every row as it
        /// was; one after the commit leaves the plan applied, and applying it again is a no-op.
        #[test]
        fn a_fault_before_the_commit_changes_nothing_and_one_after_changes_everything_once() {
            let dir = TempDir::new().unwrap();
            let (mut conn, path) = world(&dir);
            seed(&conn, &[sm(1, "a"), sm(2, "b")]);
            let p = plan(Some(2), vec![sm(2, "B")], Some(Some((1, 2))));
            let before = (stored(&conn), game_cols(&conn));
            {
                let guard = FaultPlan::new()
                    .errno("apply.before_commit", 5)
                    .install(dir.path())
                    .unwrap();
                let err = apply_reconcile_plan(&mut conn, game_id(), &p, Some(3), &path);
                assert!(
                    matches!(err, Err(StoreError::Unavailable { .. })),
                    "{err:?}"
                );
                assert_eq!(guard.fired("apply.before_commit"), 1);
            }
            assert_eq!(before, (stored(&conn), game_cols(&conn)));
            {
                let guard = FaultPlan::new()
                    .errno("apply.after_commit", 5)
                    .install(dir.path())
                    .unwrap();
                let err = apply_reconcile_plan(&mut conn, game_id(), &p, Some(3), &path);
                assert!(
                    matches!(err, Err(StoreError::Unavailable { .. })),
                    "{err:?}"
                );
                assert_eq!(guard.fired("apply.after_commit"), 1);
            }
            let committed = (stored(&conn), game_cols(&conn));
            assert_ne!(before, committed);
            apply_reconcile_plan(&mut conn, game_id(), &p, Some(3), &path).unwrap();
            assert_eq!(committed, (stored(&conn), game_cols(&conn)));
        }
    }

    /// The model fold of a journal and the rows `apply_records` writes are the same rows.
    #[test]
    fn fold_and_apply_records_agree_on_the_ply_base() {
        let dir = TempDir::new().unwrap();
        let (mut conn, path) = world(&dir);
        let mv = |seq: u64, ply: u32| JournalRecord {
            seq,
            wall_ms: 1_000 + i64::try_from(seq).unwrap(),
            payload: Payload::Move(MovePayload {
                ply,
                uci: format!("u{ply}"),
                think_ms: 3,
                white_ms: Some(10),
                black_ms: Some(20),
                san: format!("s{ply}"),
                fen_after: format!("f{ply}"),
            }),
        };
        let records = [mv(10, 0), mv(11, 1), mv(12, 2)];
        apply_records(&mut conn, game_id(), &records, &path).unwrap();
        let folded = fold(&records);
        assert_eq!(
            stored(&conn),
            folded.moves.values().cloned().collect::<Vec<_>>()
        );
        assert_eq!(
            stored(&conn).first().map(|r| r.ply),
            Some(1),
            "journal ply 0 is ply 1"
        );
    }
}
