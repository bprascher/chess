//! DD-ST-011 statistics rows (SDD 3.7.2 "Statistics (rows only, D-44)"). Rows are filtered only
//! by structural keys: the Player (a seat of the game), `started_at >= window`, the side and a
//! `LIMIT`. No predicate on `self_play`, `status` or `custom_start`, and no aggregate: the
//! counting predicate and the empty-state rule belong to `chess-domain` (`aggregate`). There is
//! no SQL view (TR-7, D-44). Rows come from `GAME` only, so a `RATING_HISTORY` point with a
//! `NULL` `game_id` (initial or reset point, F-A04 to F-A08, decision of 2026-11-11) is never a
//! game here.
//!
//! An `imported` game is never a row (RULE-7: it changes no statistic), also when the row names
//! both seats (FIND-F2-01, ruling 2026-11-24).
//!
//! Order: `(started_at, id)` ascending, like `list_games` (ruling 2026-11-23, FIND-D5R-03);
//! `limit` keeps the oldest N by start. A restored game has a later `id` than games that started
//! after it, so `id` alone is not start order.

use chess_domain::{
    Category, GameResult, GameStatus, Mode, OpponentRef, PlayerId, PlayerKind, ResultForViewed,
    WallMs,
};
use rusqlite::Connection;

use crate::api_analysis::{StatsGameRow, StatsSide};
use crate::error::StoreError;
use crate::repo::{from_i64_u32, opt_u64, parse_text, parse_uuid, player_key};

const SELECT: &str = "SELECT g.status, g.self_play, g.custom_start, g.mode, g.result, \
    g.started_at, g.total_active_ms, g.white_id = ?1, \
    (SELECT count(*) FROM move m WHERE m.game_id = g.id), \
    w.uuid, w.kind, w.strict_elo, b.uuid, b.kind, b.strict_elo \
    FROM game g JOIN player w ON w.id = g.white_id JOIN player b ON b.id = g.black_id \
    WHERE ((?3 IS NULL AND (g.white_id = ?1 OR g.black_id = ?1)) \
           OR (?3 = 'white' AND g.white_id = ?1) OR (?3 = 'black' AND g.black_id = ?1)) \
    AND (?5 IS NULL OR (g.white_id = ?1 AND g.black_id = ?5) OR (g.black_id = ?1 AND g.white_id = ?5)) \
    AND (?2 IS NULL OR g.started_at >= ?2) \
    AND g.status <> 'imported' \
    ORDER BY g.started_at, g.id LIMIT ?4";

type RawRow = (
    String,
    i64,
    i64,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    bool,
    i64,
    String,
    String,
    i64,
    String,
    String,
    i64,
);

fn opponent(uuid: &str, kind: &str, strict_elo: i64) -> Result<OpponentRef, StoreError> {
    let kind: PlayerKind = parse_text(kind)?;
    Ok(OpponentRef {
        player: PlayerId::from_uuid(parse_uuid(uuid)?),
        kind,
        // A computer's label is its Strict rating's band (F-4.4, Q79); a human has none.
        category_now: (kind == PlayerKind::Computer)
            .then(|| Category::from_rating(strict_elo_i32(strict_elo))),
    })
}

fn strict_elo_i32(value: i64) -> i32 {
    // The column holds an i32-range rating; a wider value is clamped for the band lookup only
    // (the bands are open at both ends, so the band is unchanged).
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

fn rows(
    conn: &Connection,
    player: PlayerId,
    other: Option<PlayerId>,
    window: Option<WallMs>,
    side: Option<StatsSide>,
    limit: Option<u32>,
) -> Result<Vec<StatsGameRow>, StoreError> {
    let key = player_key(conn, player)?;
    let other_key = other.map(|p| player_key(conn, p)).transpose()?;
    let limit = limit.map_or(-1, i64::from); // SQLite: a negative LIMIT is no limit
    let mut statement = conn.prepare(SELECT)?;
    let raw = statement.query_map(
        rusqlite::params![
            key,
            window.map(|w| w.0),
            side.map(StatsSide::as_str),
            limit,
            other_key
        ],
        |row| {
            Ok::<RawRow, rusqlite::Error>((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get(12)?,
                row.get(13)?,
                row.get(14)?,
            ))
        },
    )?;
    let mut out = Vec::new();
    for item in raw {
        let (
            status,
            self_play,
            custom_start,
            mode,
            result,
            started_at,
            active,
            viewed_is_white,
            plies,
            w_uuid,
            w_kind,
            w_elo,
            b_uuid,
            b_kind,
            b_elo,
        ) = item?;
        let color = match side {
            Some(side) => side,
            None if viewed_is_white => StatsSide::White,
            None => StatsSide::Black,
        };
        let opp = match color {
            StatsSide::White => opponent(&b_uuid, &b_kind, b_elo)?,
            StatsSide::Black => opponent(&w_uuid, &w_kind, w_elo)?,
        };
        let result_for_viewed = result
            .as_deref()
            .map(parse_text::<GameResult>)
            .transpose()?
            .map(|r| match (r, color) {
                (GameResult::Draw, _) => ResultForViewed::Draw,
                (GameResult::WhiteWins, StatsSide::White)
                | (GameResult::BlackWins, StatsSide::Black) => ResultForViewed::Win,
                _ => ResultForViewed::Loss,
            });
        out.push(StatsGameRow {
            status: parse_text::<GameStatus>(&status)?,
            self_play: self_play != 0,
            custom_start: custom_start != 0,
            // Imported games are excluded in the SQL (RULE-7, FIND-F2-01), so the game has a mode.
            mode: mode
                .as_deref()
                .map(parse_text::<Mode>)
                .transpose()?
                .ok_or(StoreError::Corrupt)?,
            color,
            result_for_viewed,
            started_at: WallMs(started_at.ok_or(StoreError::Corrupt)?),
            plies: from_i64_u32(plies)?,
            active_ms: opt_u64(active)?.unwrap_or(0),
            opponent: opp,
        });
    }
    Ok(out)
}

/// `stats_rows(player, window, side?, limit?)`: every game that names the Player on the given
/// side (either side when `None`), started at or after `window`.
pub(crate) fn stats_rows(
    conn: &Connection,
    player: PlayerId,
    window: Option<WallMs>,
    side: Option<StatsSide>,
    limit: Option<u32>,
) -> Result<Vec<StatsGameRow>, StoreError> {
    rows(conn, player, None, window, side, limit)
}

/// `head_to_head_rows(player, opponent, window, limit?)`: the games of `player` against
/// `opponent` on either side, viewed from `player`. `player == opponent` gives the self-play
/// games of that Player.
pub(crate) fn head_to_head_rows(
    conn: &Connection,
    player: PlayerId,
    opponent: PlayerId,
    window: Option<WallMs>,
    limit: Option<u32>,
) -> Result<Vec<StatsGameRow>, StoreError> {
    rows(conn, player, Some(opponent), window, None, limit)
}
