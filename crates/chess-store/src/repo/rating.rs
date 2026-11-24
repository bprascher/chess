//! DD-ST-010 `RatingRepo`, read side (part A). Rating rows are written inside the register,
//! roster and (part B) finalize/reset transactions; the Elo invariant check
//! (`check_rating_invariant`) belongs with finalize and is not in part A.

use chess_domain::{GameId, PlayerId, RatingPoint, Track, WallMs};
use rusqlite::Connection;

use crate::error::StoreError;
use crate::repo::{from_i64_i32, from_i64_u64, parse_uuid, player_key};

/// The rating history of one Player on one track, in append order (`id` ascending).
pub(crate) fn rating_history(
    conn: &Connection,
    player: PlayerId,
    track: Track,
) -> Result<Vec<RatingPoint>, StoreError> {
    let key = player_key(conn, player)?;
    let mut statement = conn.prepare(
        "SELECT h.id, h.at, g.uuid, h.rating_before, h.rating_after, h.k \
         FROM rating_history h LEFT JOIN game g ON g.id = h.game_id \
         WHERE h.player_id = ?1 AND h.track = ?2 ORDER BY h.id",
    )?;
    let rows = statement.query_map(rusqlite::params![key, track.as_str()], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, at, game, before, after, k) = row?;
        out.push(RatingPoint {
            id: from_i64_u64(id)?,
            at: WallMs(at),
            track,
            game_id: game
                .map(|text| parse_uuid(&text).map(GameId::from_uuid))
                .transpose()?,
            rating_before: from_i64_i32(before)?,
            rating_after: from_i64_i32(after)?,
            k: from_i64_i32(k)?,
        });
    }
    Ok(out)
}
