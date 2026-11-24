//! DD-ST-010 `RatingRepo`, the invariant check (part D2). ELO-1: for every Player and track the
//! stored rating is the last point plus the deltas of the game rows after it. A row with
//! `game_id IS NULL` is a **point**, not a game (F-A04, D-116): the initial point and a reset
//! point. `k` has no meaning on such a row and is not read here. Rows are read in append order
//! (`id`).

use chess_domain::{PlayerId, Track};
use rusqlite::Connection;

use crate::api::{RatingViolation, RatingViolationKind};
use crate::error::StoreError;
use crate::repo::{from_i64_i32, player_key};

struct Row {
    is_point: bool,
    before: i32,
    after: i32,
}

fn rows(conn: &Connection, player: i64, track: Track) -> Result<Vec<Row>, StoreError> {
    let mut statement = conn.prepare(
        "SELECT game_id IS NULL, rating_before, rating_after FROM rating_history \
         WHERE player_id = ?1 AND track = ?2 ORDER BY id",
    )?;
    let mapped = statement.query_map(rusqlite::params![player, track.as_str()], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in mapped {
        let (point, before, after) = row?;
        out.push(Row {
            is_point: point != 0,
            before: from_i64_i32(before)?,
            after: from_i64_i32(after)?,
        });
    }
    Ok(out)
}

fn check_track(rows: &[Row], stored: i32) -> Option<RatingViolationKind> {
    let first = rows.first()?;
    if !first.is_point || first.before != first.after {
        return Some(RatingViolationKind::NoInitialPoint);
    }
    let mut previous_after = first.after;
    let mut expected = first.after;
    for (index, row) in rows.iter().enumerate().skip(1) {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        if row.before != previous_after {
            return Some(RatingViolationKind::ChainBroken { index });
        }
        if row.is_point {
            if row.before != row.after {
                return Some(RatingViolationKind::PointChangesRating { index });
            }
            expected = row.after; // the latest point restarts the sum
        } else {
            expected = expected.saturating_add(row.after.saturating_sub(row.before));
        }
        previous_after = row.after;
    }
    (expected != stored).then_some(RatingViolationKind::CurrentMismatch {
        expected,
        found: stored,
    })
}

/// Checks both tracks of each Player; returns every violation found (empty means the invariant
/// holds). An unknown Player is `NotFound`.
pub(crate) fn check_rating_invariant(
    conn: &Connection,
    players: &[PlayerId],
) -> Result<Vec<RatingViolation>, StoreError> {
    // One read transaction (a WAL snapshot at the first read): `strict_elo` and the rating rows
    // must come from the same committed state, or a finalize between the two reads would look
    // like corruption (FIND-D2R-01).
    let tx = conn.unchecked_transaction()?;
    let conn: &Connection = &tx;
    let mut out = Vec::new();
    for &player in players {
        let key = player_key(conn, player)?;
        for track in [Track::Strict, Track::Training] {
            let column = match track {
                Track::Strict => "strict_elo",
                Track::Training => "training_elo",
            };
            let stored: i64 = conn.query_row(
                &format!("SELECT {column} FROM player WHERE id = ?1"),
                [key],
                |r| r.get(0),
            )?;
            let rows = rows(conn, key, track)?;
            let kind = if rows.is_empty() {
                Some(RatingViolationKind::NoInitialPoint)
            } else {
                check_track(&rows, from_i64_i32(stored)?)
            };
            if let Some(kind) = kind {
                out.push(RatingViolation {
                    player,
                    track,
                    kind,
                });
            }
        }
    }
    Ok(out)
}
