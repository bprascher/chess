//! DD-ST-009 / DD-ST-010 write path (part D2): `finalize_game` with the Elo settlement and the
//! rating rows, and `abort_game` (SDD 3.11.4). One `BEGIN IMMEDIATE` each; a game that is
//! already terminal is answered `AlreadyFinal` and changed in no way (architecture 7.4).
//! The Elo arithmetic is `chess-domain`'s (`EloCalc::settle`, whole integers, half away from
//! zero, ELO-1); this module reads the rows, calls it once per game and writes the result.

use std::path::Path;

use chess_domain::{EloCalc, GameFacts, GameStatus, Mode, PlayerId, RatedSide, RatingTrack, Track};
use rusqlite::{Connection, TransactionBehavior, params};

use crate::api::{AbortRequest, EndOutcome, FinalizeOutcome, FinalizeRequest, RatingChange};
use crate::error::StoreError;
use crate::repo::lifecycle::{TERMINATION_ABORTED, fault_check, game_status, release_and_queue};
use crate::repo::{from_i64_i32, from_i64_u32, game_key, parse_text, parse_uuid, to_i64};

/// The `PLAYER` columns of one track.
fn track_columns(track: Track) -> (&'static str, &'static str) {
    match track {
        Track::Strict => ("strict_elo", "strict_rated_games"),
        Track::Training => ("training_elo", "training_rated_games"),
    }
}

struct GameRow {
    rated: bool,
    self_play: bool,
    custom_start: bool,
    mode: Option<String>,
    white: Option<i64>,
    black: Option<i64>,
}

fn load_game(conn: &Connection, game: i64) -> Result<GameRow, StoreError> {
    Ok(conn.query_row(
        "SELECT rated, self_play, custom_start, mode, white_id, black_id FROM game WHERE id = ?1",
        [game],
        |row| {
            Ok(GameRow {
                rated: row.get::<_, i64>(0)? != 0,
                self_play: row.get::<_, i64>(1)? != 0,
                custom_start: row.get::<_, i64>(2)? != 0,
                mode: row.get(3)?,
                white: row.get(4)?,
                black: row.get(5)?,
            })
        },
    )?)
}

/// A Player's rating and rated-game counter on the track, re-read inside the transaction.
fn read_side(conn: &Connection, player: i64, track: Track) -> Result<RatedSide, StoreError> {
    let (elo, counter) = track_columns(track);
    let (rating, games): (i64, i64) = conn.query_row(
        &format!("SELECT {elo}, {counter} FROM player WHERE id = ?1"),
        [player],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(RatedSide {
        rating: from_i64_i32(rating)?,
        rated_games_before: from_i64_u32(games)?,
    })
}

fn player_uuid(conn: &Connection, key: i64) -> Result<PlayerId, StoreError> {
    let text: String =
        conn.query_row("SELECT uuid FROM player WHERE id = ?1", [key], |r| r.get(0))?;
    Ok(PlayerId::from_uuid(parse_uuid(&text)?))
}

/// One rating row and the player update of one side.
fn write_side(
    conn: &Connection,
    player: i64,
    track: Track,
    game: i64,
    update: &chess_domain::SideUpdate,
    at: i64,
) -> Result<RatingChange, StoreError> {
    let (elo, counter) = track_columns(track);
    conn.execute(
        "INSERT INTO rating_history (player_id, track, game_id, rating_before, rating_after, k, at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            player,
            track.as_str(),
            game,
            update.before,
            update.after,
            update.k,
            at
        ],
    )?;
    conn.execute(
        &format!("UPDATE player SET {elo} = ?2, {counter} = ?3 WHERE id = ?1"),
        params![player, update.after, update.counter_after],
    )?;
    Ok(RatingChange {
        player: player_uuid(conn, player)?,
        track,
        before: update.before,
        after: update.after,
        k: update.k,
    })
}

/// `analysis_used` is a latch (FIND-D2R-02, ruling 2026-11-23): the stored value is the OR of the
/// stored flag and the request's, so a `GameEnd` rebuilt after a crash never clears a grant.
///
/// SDD 3.11.4. Rating work happens only for a `rated` game that `EloCalc::rating_track` puts on a
/// track (self-play and custom starts are unrated whatever `rated` says, ELO-2).
pub(crate) fn finalize_game(
    conn: &mut Connection,
    req: &FinalizeRequest,
    ctx: &Path,
) -> Result<FinalizeOutcome, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let outcome = finalize_in_tx(&tx, req)?;
    if outcome == FinalizeOutcome::AlreadyFinal {
        return Ok(outcome); // nothing was written; the transaction rolls back on drop
    }
    fault_check("finalize.before_commit", ctx)?;
    tx.commit()?;
    Ok(outcome)
}

/// The body of the finalize inside a transaction the caller owns (part D3: `apply_records` runs
/// the `GameEnd` of a journal through this same function, UT-ST-082). The caller commits.
pub(crate) fn finalize_in_tx(
    tx: &Connection,
    req: &FinalizeRequest,
) -> Result<FinalizeOutcome, StoreError> {
    let game = game_key(tx, req.game)?;
    match game_status(tx, game)?.as_str() {
        "in_progress" => {}
        "completed" | "aborted" | "voided" => return Ok(FinalizeOutcome::AlreadyFinal),
        _ => return Err(StoreError::InvariantViolated),
    }
    if req.content_hash.is_empty() {
        return Err(StoreError::InvariantViolated); // a terminal row always has a hash (SF-17)
    }
    // FIND-D2R-03 (ruling 2026-11-23): the store checks the shape of terminal data and nothing
    // else. The order of times, the journal sequence and who may abort are not store rules.
    if req.ended_at.0 <= 0 || req.termination.is_empty() {
        return Err(StoreError::InvariantViolated);
    }
    let row = load_game(tx, game)?;
    let mode: Mode = parse_text(row.mode.as_deref().ok_or(StoreError::Corrupt)?)?;
    let facts = GameFacts {
        status: GameStatus::Completed,
        mode,
        self_play: row.self_play,
        custom_start: row.custom_start,
    };
    let (mut white_change, mut black_change) = (None, None);
    let mut elos: Option<(i32, i32, i32, i32)> = None;
    if let (true, RatingTrack::Rated(track)) = (row.rated, EloCalc::rating_track(&facts)) {
        let (white, black) = row.white.zip(row.black).ok_or(StoreError::Corrupt)?;
        let settlement = EloCalc::settle(
            read_side(tx, white, track)?,
            read_side(tx, black, track)?,
            req.result,
        )
        .map_err(|_| StoreError::InvariantViolated)?;
        white_change = Some(write_side(
            tx,
            white,
            track,
            game,
            &settlement.white,
            req.ended_at.0,
        )?);
        black_change = Some(write_side(
            tx,
            black,
            track,
            game,
            &settlement.black,
            req.ended_at.0,
        )?);
        elos = Some((
            settlement.white.before,
            settlement.white.after,
            settlement.black.before,
            settlement.black.after,
        ));
    }
    tx.execute(
        "UPDATE game SET status = 'completed', result = ?2, termination = ?3, ended_at = ?4, \
         white_time_used_ms = ?5, black_time_used_ms = ?6, total_active_ms = ?7, eco = ?8, \
         opening_name = ?9, analysis_used = max(analysis_used, ?10), content_hash = ?11, \
         white_elo_before = ?12, white_elo_after = ?13, black_elo_before = ?14, \
         black_elo_after = ?15, journal_seq = ?16 WHERE id = ?1",
        params![
            game,
            req.result.as_str(),
            req.termination,
            req.ended_at.0,
            to_i64(req.white_time_used_ms)?,
            to_i64(req.black_time_used_ms)?,
            to_i64(req.total_active_ms)?,
            req.eco,
            req.opening_name,
            i64::from(req.analysis_used),
            req.content_hash,
            elos.map(|e| e.0),
            elos.map(|e| e.1),
            elos.map(|e| e.2),
            elos.map(|e| e.3),
            to_i64(req.final_journal_seq)?,
        ],
    )?;
    release_and_queue(tx, game)?;
    Ok(FinalizeOutcome::Finalized {
        white: white_change,
        black: black_change,
    })
}

/// `abort_game`: the finalize without rating work; the hash is set, the slots and the lease are
/// released and a job is queued. `termination` is the domain's `aborted`.
pub(crate) fn abort_game(
    conn: &mut Connection,
    req: &AbortRequest,
    ctx: &Path,
) -> Result<EndOutcome, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let outcome = abort_in_tx(&tx, req)?;
    if outcome == EndOutcome::AlreadyFinal {
        return Ok(outcome);
    }
    fault_check("abort.before_commit", ctx)?;
    tx.commit()?;
    Ok(outcome)
}

/// The body of the abort inside a transaction the caller owns (part D3). The caller commits.
pub(crate) fn abort_in_tx(tx: &Connection, req: &AbortRequest) -> Result<EndOutcome, StoreError> {
    let game = game_key(tx, req.game)?;
    match game_status(tx, game)?.as_str() {
        "in_progress" => {}
        "completed" | "aborted" | "voided" => return Ok(EndOutcome::AlreadyFinal),
        _ => return Err(StoreError::InvariantViolated),
    }
    if req.content_hash.is_empty() || req.ended_at.0 <= 0 {
        return Err(StoreError::InvariantViolated); // shape only (FIND-D2R-03)
    }
    tx.execute(
        "UPDATE game SET status = 'aborted', result = NULL, termination = ?2, ended_at = ?3, \
         content_hash = ?4, journal_seq = ?5 WHERE id = ?1",
        params![
            game,
            TERMINATION_ABORTED,
            req.ended_at.0,
            req.content_hash,
            to_i64(req.final_journal_seq)?
        ],
    )?;
    release_and_queue(tx, game)?;
    Ok(EndOutcome::Applied)
}
