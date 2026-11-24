//! DD-ST-009 `GameRepo`, part A: start, read, list, moves read, unfinished list and
//! `analysis_used`. Finalize, abort, void, reset, apply (journal records) and takeback follow
//! with the journal (part B) and are not in this module yet.

use chess_domain::{
    ClockSpec, GameId, GameResult, GameStatus, Matchup, Mode, PlayerId, PlayerKind, WallMs,
};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};

use crate::api::{GameFilter, GameRecord, MoveRow, NewGame};
use crate::error::{Entity, StoreError};
use crate::repo::{
    from_i64_u32, from_i64_u64, game_key, opt_i32, opt_u64, parse_text, parse_uuid, player_key,
    to_i64, uuid_text,
};

const GAME_SELECT: &str = "SELECT g.uuid, w.uuid, b.uuid, g.matchup, g.mode, g.rated, \
    g.self_play, g.custom_start, g.start_fen, g.clock_spec, g.white_clock_ms, g.black_clock_ms, \
    g.status, g.result, g.termination, v.uuid, g.eco, g.opening_name, g.started_at, g.ended_at, \
    g.white_time_used_ms, g.black_time_used_ms, g.total_active_ms, g.analysis_used, \
    g.white_elo_before, g.white_elo_after, g.black_elo_before, g.black_elo_after, \
    g.content_hash, g.journal_seq \
    FROM game g LEFT JOIN player w ON w.id = g.white_id \
    LEFT JOIN player b ON b.id = g.black_id LEFT JOIN player v ON v.id = g.voided_by";

fn player_id(text: Option<String>) -> Result<Option<PlayerId>, StoreError> {
    text.map(|t| parse_uuid(&t).map(PlayerId::from_uuid))
        .transpose()
}

fn opt_parse<T: std::str::FromStr>(text: Option<String>) -> Result<Option<T>, StoreError> {
    text.map(|t| parse_text(&t)).transpose()
}

fn record_from_row(row: &Row<'_>) -> Result<GameRecord, StoreError> {
    let clock = match row.get::<_, Option<String>>(9)? {
        None => None,
        Some(text) => Some(ClockSpec::from_spec_text(&text).map_err(|_| StoreError::Corrupt)?),
    };
    Ok(GameRecord {
        id: GameId::from_uuid(parse_uuid(&row.get::<_, String>(0)?)?),
        white: player_id(row.get(1)?)?,
        black: player_id(row.get(2)?)?,
        matchup: opt_parse::<Matchup>(row.get(3)?)?,
        mode: opt_parse::<Mode>(row.get(4)?)?,
        rated: row.get::<_, i64>(5)? != 0,
        self_play: row.get::<_, i64>(6)? != 0,
        custom_start: row.get::<_, i64>(7)? != 0,
        start_fen: row.get(8)?,
        clock,
        white_clock_ms: opt_u64(row.get(10)?)?,
        black_clock_ms: opt_u64(row.get(11)?)?,
        status: parse_text::<GameStatus>(&row.get::<_, String>(12)?)?,
        result: opt_parse::<GameResult>(row.get(13)?)?,
        termination: row.get(14)?,
        voided_by: player_id(row.get(15)?)?,
        eco: row.get(16)?,
        opening_name: row.get(17)?,
        started_at: row.get::<_, Option<i64>>(18)?.map(WallMs),
        ended_at: row.get::<_, Option<i64>>(19)?.map(WallMs),
        white_time_used_ms: opt_u64(row.get(20)?)?,
        black_time_used_ms: opt_u64(row.get(21)?)?,
        total_active_ms: opt_u64(row.get(22)?)?,
        analysis_used: row.get::<_, i64>(23)? != 0,
        white_elo_before: opt_i32(row.get(24)?)?,
        white_elo_after: opt_i32(row.get(25)?)?,
        black_elo_before: opt_i32(row.get(26)?)?,
        black_elo_after: opt_i32(row.get(27)?)?,
        content_hash: row.get(28)?,
        journal_seq: from_i64_u64(row.get(29)?)?,
    })
}

fn find_game(conn: &Connection, id: GameId) -> Result<Option<GameRecord>, StoreError> {
    let sql = format!("{GAME_SELECT} WHERE g.uuid = ?1");
    conn.query_row(&sql, [uuid_text(id.as_uuid())], |row| {
        Ok(record_from_row(row))
    })
    .optional()?
    .transpose()
}

pub(crate) fn get_game(conn: &Connection, id: GameId) -> Result<GameRecord, StoreError> {
    find_game(conn, id)?.ok_or(StoreError::NotFound {
        entity: Entity::Game,
        id: id.as_uuid(),
    })
}

fn collect(
    conn: &Connection,
    sql: &str,
    args: &[&dyn rusqlite::ToSql],
) -> Result<Vec<GameRecord>, StoreError> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(args, |row| Ok(record_from_row(row)))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row??);
    }
    Ok(out)
}

/// Rows by structural keys only (D-44), in start order: `(started_at, id)` ascending (ruling
/// 2026-11-23, FIND-D5R-03), so `limit` is the oldest N by start even for a restored game whose
/// `id` is later than games that started after it. A game with no `started_at` (an imported
/// one) comes **last** (FIND-F2-02, ruling 2026-11-24), so "the oldest N" are played games.
/// The reset window is `started_at >= from`.
pub(crate) fn list_games(
    conn: &Connection,
    filter: &GameFilter,
) -> Result<Vec<GameRecord>, StoreError> {
    let player = filter.player.map(|p| player_key(conn, p)).transpose()?;
    let sql = format!(
        "{GAME_SELECT} WHERE (?1 IS NULL OR g.white_id = ?1 OR g.black_id = ?1) \
         AND (?2 IS NULL OR g.status = ?2) AND (?3 IS NULL OR g.started_at >= ?3) \
         ORDER BY g.started_at IS NULL, g.started_at, g.id LIMIT ?4"
    );
    let limit = filter.limit.map_or(-1, i64::from); // SQLite: a negative LIMIT is no limit
    collect(
        conn,
        &sql,
        &[
            &player,
            &filter.status.map(GameStatus::as_str),
            &filter.from.map(|w| w.0),
            &limit,
        ],
    )
}

/// The resume list of a Player: `in_progress` games that name the Player, read from `GAME`, not
/// from the slot table (architecture 4.4; SDD 3.7.2).
pub(crate) fn list_unfinished(
    conn: &Connection,
    player: PlayerId,
) -> Result<Vec<GameRecord>, StoreError> {
    list_games(
        conn,
        &GameFilter {
            player: Some(player),
            status: Some(GameStatus::InProgress),
            ..GameFilter::default()
        },
    )
}

pub(crate) fn game_moves(conn: &Connection, id: GameId) -> Result<Vec<MoveRow>, StoreError> {
    let key = game_key(conn, id)?;
    let mut statement = conn.prepare(
        "SELECT ply, san, uci, fen_after, think_ms, white_clock_ms, black_clock_ms, played_at \
         FROM move WHERE game_id = ?1 ORDER BY ply",
    )?;
    let rows = statement.query_map([key], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, Option<i64>>(6)?,
            row.get::<_, Option<i64>>(7)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (ply, san, uci, fen_after, think, white, black, played) = row?;
        let clocks_ms = match (white, black) {
            (Some(w), Some(b)) => Some((from_i64_u64(w)?, from_i64_u64(b)?)),
            (None, None) => None,
            _ => return Err(StoreError::Corrupt),
        };
        out.push(MoveRow {
            ply: from_i64_u32(ply)?,
            san,
            uci,
            fen_after,
            think_ms: opt_u64(think)?,
            clocks_ms,
            played_at: played.map(WallMs),
        });
    }
    Ok(out)
}

/// Idempotent, and a latch (FIND-D2R-02): setting the flag twice leaves it set, and nothing
/// clears it (`finalize_game` stores the OR). A game that is already final is refused with
/// `InvariantViolated` (FIND-F2-05, ruling 2026-11-24): the flag of a finished game is part of
/// what its hash and statistics were settled on. Setting a flag that is already set stays `Ok`.
pub(crate) fn set_analysis_used(conn: &mut Connection, id: GameId) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, id)?;
    let (status, already): (String, i64) = tx.query_row(
        "SELECT status, analysis_used FROM game WHERE id = ?1",
        [key],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if status != "in_progress" {
        return if already != 0 {
            Ok(())
        } else {
            Err(StoreError::InvariantViolated)
        };
    }
    tx.execute("UPDATE game SET analysis_used = 1 WHERE id = ?1", [key])?;
    tx.commit()?;
    Ok(())
}

fn kind_of(conn: &Connection, key: i64) -> Result<PlayerKind, StoreError> {
    let text: String =
        conn.query_row("SELECT kind FROM player WHERE id = ?1", [key], |r| r.get(0))?;
    parse_text(&text)
}

/// `GAME` + the slot rows in one `BEGIN IMMEDIATE`. Idempotent on `GameId`: a known id returns
/// the existing row and changes nothing (its `started_at` stays). Slot holders (architecture
/// 7.4): h-h each human, h-c the human only, c-c each computer; one row for self-play. A held
/// slot is `SlotTaken` naming the game that holds it. A matchup that disagrees with the kinds of
/// the two Players is `InvariantViolated`.
pub(crate) fn start_game(conn: &mut Connection, new: &NewGame) -> Result<GameRecord, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(existing) = find_game(&tx, new.id)? {
        return Ok(existing);
    }
    let white = player_key(&tx, new.white)?;
    let black = player_key(&tx, new.black)?;
    let (white_kind, black_kind) = (kind_of(&tx, white)?, kind_of(&tx, black)?);
    let kinds_match = match new.matchup {
        Matchup::HumanHuman => white_kind == PlayerKind::Human && black_kind == PlayerKind::Human,
        Matchup::ComputerComputer => {
            white_kind == PlayerKind::Computer && black_kind == PlayerKind::Computer
        }
        Matchup::HumanComputer => white_kind != black_kind,
    };
    if !kinds_match {
        return Err(StoreError::InvariantViolated);
    }
    // (key, id) of every slot holder, without a duplicate for self-play.
    let mut holders: Vec<(i64, PlayerId)> = Vec::new();
    let candidates = [
        (white, new.white, white_kind),
        (black, new.black, black_kind),
    ];
    for (key, id, kind) in candidates {
        let holds_slot = match new.matchup {
            Matchup::HumanHuman | Matchup::HumanComputer => kind == PlayerKind::Human,
            Matchup::ComputerComputer => true,
        };
        if holds_slot && !holders.iter().any(|(k, _)| *k == key) {
            holders.push((key, id));
        }
    }
    for (key, id) in &holders {
        let holder = tx
            .query_row(
                "SELECT g.uuid FROM unfinished_slot s JOIN game g ON g.id = s.game_id \
                 WHERE s.player_id = ?1 AND s.mode = ?2",
                params![key, new.mode.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(uuid) = holder {
            return Err(StoreError::SlotTaken {
                game_id: GameId::from_uuid(parse_uuid(&uuid)?),
                mode: new.mode,
                player_id: *id,
            });
        }
    }
    let base_ms = new.clock.map(|c| to_i64(c.base_ms())).transpose()?;
    tx.execute(
        "INSERT INTO game (uuid, white_id, black_id, matchup, mode, rated, self_play, custom_start, \
         start_fen, clock_spec, white_clock_ms, black_clock_ms, status, started_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 'in_progress', ?12)",
        params![
            uuid_text(new.id.as_uuid()),
            white,
            black,
            new.matchup.as_str(),
            new.mode.as_str(),
            i64::from(new.rated),
            i64::from(white == black),
            i64::from(new.start_fen.is_some()),
            new.start_fen,
            new.clock.map(ClockSpec::to_spec_text),
            base_ms,
            new.started_at.0,
        ],
    )?;
    let game = tx.last_insert_rowid();
    for (key, _) in &holders {
        tx.execute(
            "INSERT INTO unfinished_slot (player_id, mode, game_id) VALUES (?1, ?2, ?3)",
            params![key, new.mode.as_str(), game],
        )?;
    }
    let record = get_game(&tx, new.id)?;
    tx.commit()?;
    Ok(record)
}
