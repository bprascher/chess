//! DD-ST-009 `GameRepo`, part D2: the shared pieces of every transition out of `in_progress`
//! (slot and lease deletion, the queued analysis job, the void) and `reset_player`. The void
//! function here is the one function that `reset_player`, `void_unreadable_game` and (part D3)
//! `apply_records` use (D-78 (5), XC-ST-A8-4). Integer keys stay inside this module (D-40).

use std::path::Path;

use chess_domain::{GameId, PlayerId, Track};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::api::{EndOutcome, VoidSpec};
use crate::error::{StoreError, map_io};
use crate::fault;
use crate::repo::{game_key, parse_uuid, player_key};

/// `GAME.termination` of a game voided by a reset (SF-10, architecture 7.2).
pub(crate) const TERMINATION_VOIDED_BY_RESET: &str = "voided by reset";
/// `GAME.termination` of a game voided as unreadable (DAT-7).
pub(crate) const TERMINATION_VOIDED_UNREADABLE: &str = "voided: unreadable";
/// `GAME.termination` of an aborted game (the domain's `EndReason::Aborted` text).
pub(crate) const TERMINATION_ABORTED: &str = "aborted";

/// A failpoint call. `ctx` is the database path as the data directory names it: the path that
/// SQLite reports for an open connection can be a canonical form that a test's `FaultPlan`
/// root does not match (`/private/var` against `/var` on macOS).
pub(crate) fn fault_check(name: &'static str, ctx: &Path) -> Result<(), StoreError> {
    fault::check(name, ctx).map_err(|e| map_io(&e))
}

/// The `in_progress` games that name the Player, in start order.
pub(crate) fn unfinished_game_ids(
    conn: &Connection,
    player_key: i64,
) -> Result<Vec<GameId>, StoreError> {
    let mut statement = conn.prepare(
        "SELECT uuid FROM game WHERE status = 'in_progress' \
         AND (white_id = ?1 OR black_id = ?1) ORDER BY id",
    )?;
    let rows = statement.query_map([player_key], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(GameId::from_uuid(parse_uuid(&row?)?));
    }
    Ok(out)
}

/// Every slot and lease row of the game, whichever Player holds it, and a queued analysis job
/// (architecture 7.2, 7.4). Called in the transaction that makes the game terminal.
pub(crate) fn release_and_queue(conn: &Connection, game: i64) -> Result<(), StoreError> {
    conn.execute("DELETE FROM unfinished_slot WHERE game_id = ?1", [game])?;
    conn.execute("DELETE FROM game_lease WHERE game_id = ?1", [game])?;
    // OR IGNORE: a void that follows a finalize (a repair pair) finds the job already queued.
    conn.execute(
        "INSERT OR IGNORE INTO analysis_job (game_id, state) VALUES (?1, 'queued')",
        [game],
    )?;
    Ok(())
}

/// The state a void writes (SDD 3.11.4).
pub(crate) struct VoidFields<'a> {
    pub termination: &'static str,
    pub voided_by: Option<i64>,
    pub ended_at: i64,
    pub content_hash: &'a str,
}

/// The void: `status='voided'`, no result, termination, `voided_by`, `ended_at`, the hash, slots
/// and leases deleted, a job queued. No rating row and no counter changes for anyone (D-30).
/// The caller has checked the precondition and holds the write transaction.
///
/// **Refused with `InvariantViolated`, nothing written:** an empty hash, an `ended_at` at or
/// below zero (D3 ruling 2026-11-24, the same bound as the journal format and `finalize_in_tx`),
/// and (FIND-D2R-05) a row that is already `completed` with settled ratings (an Elo column or a
/// rating row of the game): the void would clear the result and keep the settled ratings, a game
/// that says "void" and counts. There is no bypass; the repair pair of D-87 never reaches this
/// function with a settled row (its `GameEnd` is skipped before the void).
pub(crate) fn void_game(
    conn: &Connection,
    game: i64,
    fields: &VoidFields<'_>,
) -> Result<(), StoreError> {
    if fields.content_hash.is_empty() || fields.ended_at <= 0 {
        return Err(StoreError::InvariantViolated);
    }
    let settled: bool = conn.query_row(
        "SELECT status = 'completed' AND (white_elo_after IS NOT NULL \
         OR black_elo_after IS NOT NULL \
         OR EXISTS (SELECT 1 FROM rating_history WHERE game_id = game.id)) \
         FROM game WHERE id = ?1",
        [game],
        |r| r.get(0),
    )?;
    if settled {
        return Err(StoreError::InvariantViolated);
    }
    conn.execute(
        "UPDATE game SET status = 'voided', result = NULL, termination = ?2, voided_by = ?3, \
         ended_at = ?4, content_hash = ?5 WHERE id = ?1",
        params![
            game,
            fields.termination,
            fields.voided_by,
            fields.ended_at,
            fields.content_hash
        ],
    )?;
    release_and_queue(conn, game)
}

pub(crate) fn game_status(conn: &Connection, game: i64) -> Result<String, StoreError> {
    Ok(
        conn.query_row("SELECT status FROM game WHERE id = ?1", [game], |r| {
            r.get(0)
        })?,
    )
}

/// `void_unreadable_game` (DAT-7, D-35): `voided: unreadable`, `voided_by` NULL, the hash of
/// whatever moves are readable (zero allowed, never NULL), `ended_at` = the void time `now`.
/// A game that is already terminal is left alone.
pub(crate) fn void_unreadable_game(
    conn: &mut Connection,
    id: GameId,
    content_hash: &str,
    now: i64,
    ctx: &Path,
) -> Result<EndOutcome, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let game = game_key(&tx, id)?;
    match game_status(&tx, game)?.as_str() {
        "in_progress" => {}
        "completed" | "aborted" | "voided" => return Ok(EndOutcome::AlreadyFinal),
        _ => return Err(StoreError::InvariantViolated),
    }
    void_game(
        &tx,
        game,
        &VoidFields {
            termination: TERMINATION_VOIDED_UNREADABLE,
            voided_by: None,
            ended_at: now,
            content_hash,
        },
    )?;
    fault_check("void.before_commit", ctx)?;
    tx.commit()?;
    Ok(EndOutcome::Applied)
}

/// `reset_player` (PLY-7, D-30; SDD 3.11.4): one `BEGIN IMMEDIATE`. Any `in_progress` game of
/// the Player that is not listed rolls everything back (`UnlistedUnfinishedGames`). Otherwise
/// `reset_at = now`, the preferences blob is replaced, one reset point per track is appended
/// (`game_id` NULL, before = after = current, `k` = 0 by F-A04) and each listed game is voided
/// (`voided_by` = the Player, `ended_at` = `now`, the hash from its `VoidSpec`). Ratings and
/// counters are untouched.
///
/// A listed game that is already terminal is skipped (it finished between the confirmation and
/// the reset); a listed game that is `in_progress` but does not name the Player, or an unknown
/// game, is an error.
pub(crate) fn reset_player(
    conn: &mut Connection,
    id: PlayerId,
    voids: &[VoidSpec],
    default_preferences_json: &str,
    now: i64,
    ctx: &Path,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = player_key(&tx, id)?;
    let kind: String =
        tx.query_row("SELECT kind FROM player WHERE id = ?1", [key], |r| r.get(0))?;
    if kind != "human" {
        return Err(StoreError::InvariantViolated); // reset is for a human Player (schema CHECK)
    }
    let unfinished = unfinished_game_ids(&tx, key)?;
    let unlisted: Vec<GameId> = unfinished
        .iter()
        .copied()
        .filter(|g| !voids.iter().any(|v| v.game == *g))
        .collect();
    if !unlisted.is_empty() {
        return Err(StoreError::UnlistedUnfinishedGames { ids: unlisted });
    }
    tx.execute(
        "UPDATE player SET reset_at = ?2 WHERE id = ?1",
        params![key, now],
    )?;
    tx.execute(
        "INSERT INTO preferences (player_id, json) VALUES (?1, ?2) \
         ON CONFLICT(player_id) DO UPDATE SET json = excluded.json",
        params![key, default_preferences_json],
    )?;
    for track in [Track::Strict, Track::Training] {
        let column = match track {
            Track::Strict => "strict_elo",
            Track::Training => "training_elo",
        };
        let current: i64 = tx.query_row(
            &format!("SELECT {column} FROM player WHERE id = ?1"),
            [key],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO rating_history (player_id, track, game_id, rating_before, rating_after, k, at) \
             VALUES (?1, ?2, NULL, ?3, ?3, 0, ?4)",
            params![key, track.as_str(), current, now],
        )?;
    }
    for spec in voids {
        // A game listed twice is voided once: the second entry finds it terminal and is skipped.
        let game = game_key(&tx, spec.game)?;
        match game_status(&tx, game)?.as_str() {
            "completed" | "aborted" | "voided" => continue,
            "in_progress" => {}
            _ => return Err(StoreError::InvariantViolated),
        }
        let names_player: bool = tx
            .query_row(
                "SELECT 1 FROM game WHERE id = ?1 AND (white_id = ?2 OR black_id = ?2)",
                params![game, key],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !names_player {
            return Err(StoreError::InvariantViolated);
        }
        void_game(
            &tx,
            game,
            &VoidFields {
                termination: TERMINATION_VOIDED_BY_RESET,
                voided_by: Some(key),
                ended_at: now,
                content_hash: &spec.content_hash,
            },
        )?;
    }
    fault_check("reset.before_commit", ctx)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
        conn
    }

    fn game(conn: &Connection, id: i64, status: &str, elo_after: Option<i64>) {
        let (result, hash) = if status == "in_progress" {
            (None, None)
        } else {
            (Some("1-0"), Some("h"))
        };
        conn.execute(
            "INSERT INTO game (id, uuid, white_id, black_id, matchup, mode, rated, self_play, \
             status, result, content_hash, white_elo_after) \
             VALUES (?1, ?2, 1, 2, 'hh', 'strict', 1, 0, ?3, ?4, ?5, ?6)",
            params![
                id,
                format!("10000000-0000-0000-0000-{id:012}"),
                status,
                result,
                hash,
                elo_after
            ],
        )
        .unwrap();
    }

    fn fields() -> VoidFields<'static> {
        VoidFields {
            termination: TERMINATION_VOIDED_UNREADABLE,
            voided_by: None,
            ended_at: 9,
            content_hash: "h2",
        }
    }

    fn status(conn: &Connection, id: i64) -> String {
        game_status(conn, id).unwrap()
    }

    /// FIND-D2R-05: a completed row with a settled Elo column is refused and unchanged.
    #[test]
    fn void_game_refuses_a_completed_row_with_a_settled_elo_column() {
        let conn = world();
        game(&conn, 1, "completed", Some(1204));
        assert!(matches!(
            void_game(&conn, 1, &fields()),
            Err(StoreError::InvariantViolated)
        ));
        assert_eq!(status(&conn, 1), "completed");
        let result: Option<String> = conn
            .query_row("SELECT result FROM game WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(result.as_deref(), Some("1-0"));
    }

    /// FIND-D2R-05: a rating row of the game is settled ratings too, whatever the columns say.
    #[test]
    fn void_game_refuses_a_completed_row_with_a_rating_row() {
        let conn = world();
        game(&conn, 1, "completed", None);
        conn.execute(
            "INSERT INTO rating_history (player_id, track, game_id, rating_before, rating_after, \
             k, at) VALUES (1, 'strict', 1, 1200, 1204, 40, 1)",
            [],
        )
        .unwrap();
        assert!(matches!(
            void_game(&conn, 1, &fields()),
            Err(StoreError::InvariantViolated)
        ));
        assert_eq!(status(&conn, 1), "completed");
    }

    /// An in-progress row, and a completed row that settled nothing (unrated), are voided.
    #[test]
    fn void_game_voids_an_in_progress_row_and_a_completed_row_without_ratings() {
        let conn = world();
        game(&conn, 1, "in_progress", None);
        void_game(&conn, 1, &fields()).unwrap();
        assert_eq!(status(&conn, 1), "voided");
        game(&conn, 2, "completed", None);
        void_game(&conn, 2, &fields()).unwrap();
        assert_eq!(status(&conn, 2), "voided");
    }

    /// Only `completed` is protected: a row that is already `voided` (a replayed void) keeps the
    /// behaviour of before, even when the repair pair left Elo columns on it.
    #[test]
    fn void_game_on_an_already_voided_row_is_not_refused() {
        let conn = world();
        game(&conn, 1, "completed", Some(1204));
        conn.execute(
            "UPDATE game SET status = 'voided', result = NULL WHERE id = 1",
            [],
        )
        .unwrap();
        void_game(&conn, 1, &fields()).unwrap();
        assert_eq!(status(&conn, 1), "voided");
    }

    /// N01 (FIND-D2R-04): the job insert is `OR IGNORE`: a void that follows a finalize finds the
    /// job queued already and must not fail on the primary key.
    #[test]
    fn release_and_queue_twice_keeps_one_job_and_does_not_fail() {
        let conn = world();
        game(&conn, 1, "in_progress", None);
        release_and_queue(&conn, 1).unwrap();
        conn.execute("UPDATE analysis_job SET state = 'running', owner = 'w'", [])
            .unwrap();
        release_and_queue(&conn, 1).unwrap();
        let (n, state): (i64, String) = conn
            .query_row("SELECT count(*), min(state) FROM analysis_job", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(
            (n, state.as_str()),
            (1, "running"),
            "the second call changes nothing"
        );
    }

    /// D3 ruling 2026-11-24: an `ended_at` at or below zero is refused and nothing is written;
    /// 1 is accepted. (The live paths and the journal apply use this one function.)
    #[test]
    fn void_game_refuses_an_ended_at_at_or_below_zero() {
        let conn = world();
        game(&conn, 1, "in_progress", None);
        for bad in [0, -1, i64::MIN] {
            let refused = VoidFields {
                ended_at: bad,
                ..fields()
            };
            assert!(matches!(
                void_game(&conn, 1, &refused),
                Err(StoreError::InvariantViolated)
            ));
            assert_eq!(status(&conn, 1), "in_progress", "{bad}");
        }
        let one = VoidFields {
            ended_at: 1,
            ..fields()
        };
        void_game(&conn, 1, &one).unwrap();
        assert_eq!(status(&conn, 1), "voided");
    }

    /// An empty hash is refused, and an unknown row is an error, not a void.
    #[test]
    fn void_game_refuses_an_empty_hash_and_an_unknown_row() {
        let conn = world();
        game(&conn, 1, "in_progress", None);
        let empty = VoidFields {
            content_hash: "",
            ..fields()
        };
        assert!(matches!(
            void_game(&conn, 1, &empty),
            Err(StoreError::InvariantViolated)
        ));
        assert!(void_game(&conn, 99, &fields()).is_err());
        assert_eq!(status(&conn, 1), "in_progress");
    }
}
