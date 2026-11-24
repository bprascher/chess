//! Part D3: `apply_records` through the public `Store` API on a real temporary database
//! (SDD 3.7.2, 3.11.1, 3.11.3, 3.11.4): every record kind, the ply base (D-92), UT-ST-082
//! (finalize live equals finalize from `GameEnd` alone), UT-ST-085 (`Voided`), UT-ST-089
//! (a `Start`-only journal), idempotence, atomicity, the D-87 pair and the refusals of terminal
//! data (FIND-F2-04).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use chess_domain::{GameResult, GameStatus, Matchup, Mode, WallMs};
use chess_store::journal::fold;
use chess_store::journal::format::{
    AbortPayload, ClockText, EndReason, GameEndPayload, JournalRecord, Payload, SeatKind, Side,
    TakebackPayload, UuidText, VoidReason, VoidedPayload,
};
use chess_store::{
    ApplyOutcome, Entity, FinalizeRequest, GameRecord, Store, StoreError, StoreOptions,
};
use common::{block_on, computer, gid, human, jb, new_game, open_in, pid, raw};
use rusqlite::Connection;
use tempfile::TempDir;
use uuid::Uuid;

const HASH: &str = "sha256:v1:aa";
const ENDED: i64 = 9_000_000;

/// Humans 1 (Ann) and 2 (Bob), computers 10 and 11.
fn world() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    for (n, name) in [(1, "Ann"), (2, "Bob")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![
        computer(10, "Bot A", 1, 0, 3),
        computer(11, "Bot B", 1, 1, 4),
    ]))
    .unwrap();
    (dir, s)
}

/// `world` plus the live game 1, Ann (white) against Bob, strict, rated.
fn live() -> (TempDir, Store) {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
    (d, s)
}

fn rec(seq: u64, payload: Payload) -> JournalRecord {
    JournalRecord {
        seq,
        wall_ms: 1_000 + i64::try_from(seq).unwrap(),
        payload,
    }
}

fn game_end(analysis_used: bool) -> Payload {
    Payload::GameEnd(GameEndPayload {
        result: "1-0".into(),
        reason: EndReason::Checkmate,
        ended_at: ENDED,
        eco: Some("C20".into()),
        opening_name: Some("King's Pawn".into()),
        analysis_used,
        content_hash: HASH.into(),
        white_time_used_ms: 111,
        black_time_used_ms: 222,
        total_active_ms: 333,
    })
}

fn voided(reason: VoidReason, by: Option<u128>) -> Payload {
    Payload::Voided(VoidedPayload {
        reason,
        reset_player: by.map(|n| UuidText(Uuid::from_u128(n))),
        content_hash: "sha256:v1:void".into(),
        ended_at: 8_000_000,
    })
}

fn apply(s: &Store, game: u128, records: Vec<JournalRecord>) -> Result<ApplyOutcome, StoreError> {
    block_on(s.apply_records(gid(game), records))
}

fn dump(conn: &Connection) -> Vec<String> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut out = Vec::new();
    for table in tables {
        let mut st = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = st.column_count();
        let rows: Vec<String> = st
            .query_map([], |r| {
                let mut cells = Vec::new();
                for i in 0..columns {
                    let v: rusqlite::types::Value = r.get(i)?;
                    cells.push(format!("{v:?}"));
                }
                Ok(cells.join("|"))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        out.extend(rows.into_iter().map(|r| format!("{table}: {r}")));
    }
    out.sort();
    out
}

fn game(s: &Store, n: u128) -> GameRecord {
    block_on(s.get_game(gid(n))).unwrap()
}

// ------------------------------------------------------------------------ Move and the ply base

/// The SDD 3.11.3 warning: a round trip passes for either base, so this case states the base.
/// Journal `Move{ply: 0}` is `MOVE.ply = 1`; ply 1 is 2.
#[test]
fn a_journal_move_with_ply_zero_is_stored_at_ply_one() {
    let (_d, s) = live();
    let out = apply(&s, 1, vec![rec(2, jb::mv(0))]).unwrap();
    assert_eq!(out, ApplyOutcome::Applied);
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(rows.iter().map(|r| r.ply).collect::<Vec<_>>(), vec![1]);
    assert_eq!(rows[0].uci, "u0");
    apply(&s, 1, vec![rec(3, jb::mv(1))]).unwrap();
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(rows.iter().map(|r| r.ply).collect::<Vec<_>>(), vec![1, 2]);
    // The raw table never holds ply 0 (its CHECK would refuse it).
    let min: i64 = raw(_d.path())
        .query_row("SELECT min(ply) FROM move", [], |r| r.get(0))
        .unwrap();
    assert_eq!(min, 1);
}

/// A `Move` writes the row and the clocks, `played_at` is the record's `wall_ms`, and
/// `journal_seq` follows the batch.
#[test]
fn a_move_writes_the_row_the_clocks_and_the_journal_sequence() {
    let (_d, s) = live();
    apply(&s, 1, vec![rec(2, jb::mv(0)), rec(3, jb::mv(1))]).unwrap();
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(rows[1].san, "s1");
    assert_eq!(rows[1].fen_after, "fen1");
    assert_eq!(rows[1].think_ms, Some(11));
    assert_eq!(rows[1].clocks_ms, Some((299_999, 300_001)));
    assert_eq!(rows[1].played_at, Some(WallMs(1_003)));
    let g = game(&s, 1);
    assert_eq!(
        (g.white_clock_ms, g.black_clock_ms),
        (Some(299_999), Some(300_001))
    );
    assert_eq!(g.journal_seq, 3);
}

#[test]
fn an_untimed_move_leaves_null_clocks_on_the_row_and_the_game() {
    let (_d, s) = live();
    apply(&s, 1, vec![rec(2, jb::untimed_move(0))]).unwrap();
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(rows[0].clocks_ms, None);
    let g = game(&s, 1);
    assert_eq!((g.white_clock_ms, g.black_clock_ms), (None, None));
}

/// The journal wins at a ply: a second `Move` at the same ply replaces the row.
#[test]
fn a_move_at_a_stored_ply_replaces_the_row() {
    let (_d, s) = live();
    apply(&s, 1, vec![rec(2, jb::mv(0))]).unwrap();
    let Payload::Move(mut other) = jb::mv(0) else {
        panic!()
    };
    other.uci = "zz".into();
    apply(&s, 1, vec![rec(3, Payload::Move(other))]).unwrap();
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!((rows.len(), rows[0].uci.as_str()), (1, "zz"));
}

#[test]
fn a_takeback_deletes_the_rows_above_to_ply_and_keeps_to_ply() {
    let (d, s) = live();
    apply(
        &s,
        1,
        vec![
            rec(2, jb::mv(0)),
            rec(3, jb::mv(1)),
            rec(4, jb::mv(2)),
            rec(5, jb::mv(3)),
        ],
    )
    .unwrap();
    // A stored evaluation of a removed ply must not block the delete (foreign key).
    raw(d.path())
        .execute(
            "INSERT INTO move_eval (game_id, ply, cp, engine_version) VALUES (1, 4, 3, 'sf')",
            [],
        )
        .unwrap();
    apply(
        &s,
        1,
        vec![rec(6, Payload::Takeback(TakebackPayload { to_ply: 2 }))],
    )
    .unwrap();
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(rows.iter().map(|r| r.ply).collect::<Vec<_>>(), vec![1, 2]);
    let evals: i64 = raw(d.path())
        .query_row("SELECT count(*) FROM move_eval", [], |r| r.get(0))
        .unwrap();
    assert_eq!(evals, 0);
    // Takeback to 0 empties the game.
    apply(
        &s,
        1,
        vec![rec(7, Payload::Takeback(TakebackPayload { to_ply: 0 }))],
    )
    .unwrap();
    assert!(block_on(s.game_moves(gid(1))).unwrap().is_empty());
}

#[test]
fn a_clock_record_sets_the_game_clocks_and_an_untimed_one_clears_them() {
    let (_d, s) = live();
    apply(&s, 1, vec![rec(2, jb::clock(111, 222))]).unwrap();
    let g = game(&s, 1);
    assert_eq!((g.white_clock_ms, g.black_clock_ms), (Some(111), Some(222)));
    apply(
        &s,
        1,
        vec![rec(
            3,
            Payload::Clock(chess_store::journal::format::ClockPayload {
                white_ms: None,
                black_ms: None,
                running_side: None,
            }),
        )],
    )
    .unwrap();
    let g = game(&s, 1);
    assert_eq!((g.white_clock_ms, g.black_clock_ms), (None, None));
}

/// The kinds that "change nothing" change no row of any table; only `journal_seq` moves.
#[test]
fn the_other_kinds_change_no_row_but_the_journal_sequence() {
    let (d, s) = live();
    let before = dump(&raw(d.path()));
    let by = chess_store::journal::format::DrawByPayload { by: Side::White };
    let others = vec![
        Payload::DrawOffer(by.clone()),
        Payload::DrawDeclined(by),
        Payload::DrawDropped(chess_store::journal::format::DrawDroppedPayload {
            by: Side::Black,
            cause: chess_store::journal::format::DropCause::Suspend,
        }),
        Payload::Pause,
        Payload::Resume,
        Payload::Suspend(chess_store::journal::format::SuspendPayload {
            cause: chess_store::journal::format::SuspendCause::Detach,
        }),
        Payload::SuspendResume,
    ];
    let records: Vec<JournalRecord> = others
        .into_iter()
        .zip(20u64..)
        .map(|(p, seq)| rec(seq, p))
        .collect();
    apply(&s, 1, records).unwrap();
    let after = dump(&raw(d.path()));
    let changed: Vec<&String> = before.iter().filter(|r| !after.contains(r)).collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].starts_with("game:"));
    assert_eq!(game(&s, 1).journal_seq, 26);
}

/// The batch is the fold: whatever `fold` says the rows are, `apply_records` stores (the model
/// of 3.11.3 and the SQL are the same function of the journal).
#[test]
fn applying_a_whole_journal_gives_the_rows_of_the_fold() {
    let (_d, s) = live();
    let records = vec![
        rec(2, jb::mv(0)),
        rec(3, jb::mv(1)),
        rec(4, jb::mv(2)),
        rec(5, jb::mv(3)),
        rec(6, Payload::Takeback(TakebackPayload { to_ply: 2 })),
        rec(7, jb::mv(2)),
        rec(8, jb::clock(5, 6)),
    ];
    apply(&s, 1, records.clone()).unwrap();
    let folded = fold(&records);
    let rows = block_on(s.game_moves(gid(1))).unwrap();
    assert_eq!(rows.len(), folded.moves.len());
    for row in rows {
        let m = &folded.moves[&row.ply];
        assert_eq!(
            (row.uci.as_str(), row.san.as_str(), row.fen_after.as_str()),
            (m.uci.as_str(), m.san.as_str(), m.fen_after.as_str())
        );
        assert_eq!((row.think_ms, row.clocks_ms), (m.think_ms, m.clocks));
        assert_eq!(row.played_at.map(|w| w.0), m.played_at);
    }
    let g = game(&s, 1);
    assert_eq!((g.white_clock_ms, g.black_clock_ms), (Some(5), Some(6)));
}

// ------------------------------------------------------------------------------ UT-ST-082

fn rows_of(conn: &Connection, sql: &str) -> Vec<String> {
    let mut st = conn.prepare(sql).unwrap();
    let n = st.column_count();
    st.query_map([], |r| {
        let mut cells = Vec::new();
        for i in 0..n {
            let v: rusqlite::types::Value = r.get(i)?;
            cells.push(format!("{v:?}"));
        }
        Ok(cells.join("|"))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

/// UT-ST-082: the store half of FF-3. A live `finalize_game` and the finalize that `apply_records`
/// runs from the `GameEnd` record alone leave **every column** of `GAME`, `PLAYER`,
/// `RATING_HISTORY`, the slots, the leases and the job queue the same. The three time totals
/// come from the record (D-63), `analysis_used` is carried, the termination text is the
/// domain's.
#[test]
fn ut_st_082_finalize_live_equals_finalize_from_game_end_alone() {
    let (da, a) = live();
    let (db, b) = live();
    let request = FinalizeRequest {
        game: gid(1),
        result: GameResult::WhiteWins,
        termination: "checkmate".into(),
        ended_at: WallMs(ENDED),
        white_time_used_ms: 111,
        black_time_used_ms: 222,
        total_active_ms: 333,
        eco: Some("C20".into()),
        opening_name: Some("King's Pawn".into()),
        analysis_used: true,
        content_hash: HASH.into(),
        final_journal_seq: 12,
    };
    block_on(a.finalize_game(request)).unwrap();
    apply(&b, 1, vec![rec(12, game_end(true))]).unwrap();
    for table in [
        "game",
        "player",
        "rating_history",
        "unfinished_slot",
        "game_lease",
        "analysis_job",
        "preferences",
    ] {
        let sql = format!("SELECT * FROM {table} ORDER BY 1, 2");
        assert_eq!(
            rows_of(&raw(da.path()), &sql),
            rows_of(&raw(db.path()), &sql),
            "{table}"
        );
    }
    let g = game(&b, 1);
    assert_eq!(g.status, GameStatus::Completed);
    assert_eq!(g.termination.as_deref(), Some("checkmate"));
    assert_eq!(
        (
            g.white_time_used_ms,
            g.black_time_used_ms,
            g.total_active_ms
        ),
        (Some(111), Some(222), Some(333))
    );
    assert_eq!(g.journal_seq, 12);
    assert!(g.analysis_used);
    assert_eq!(
        block_on(b.get_player(pid(1))).unwrap().strict_elo,
        1220,
        "settled once"
    );
}

/// Every journal reason maps to the unchanged `GAME.termination` text (SF-15).
#[test]
fn each_game_end_reason_writes_the_domain_termination_text() {
    for (reason, text) in [
        (EndReason::Checkmate, "checkmate"),
        (EndReason::Stalemate, "stalemate"),
        (EndReason::InsufficientMaterial, "insufficient material"),
        (EndReason::ThreefoldRepetition, "threefold repetition"),
        (EndReason::FiftyMoveRule, "fifty-move rule"),
        (EndReason::TimeForfeit, "time forfeit"),
        (
            EndReason::TimeoutOpponentCannotMate,
            "time forfeit, opponent cannot mate",
        ),
        (EndReason::Resignation, "resignation"),
        (EndReason::DrawAgreed, "draw agreed"),
    ] {
        let (_d, s) = live();
        let Payload::GameEnd(mut p) = game_end(false) else {
            panic!()
        };
        p.reason = reason;
        apply(&s, 1, vec![rec(5, Payload::GameEnd(p))]).unwrap();
        assert_eq!(game(&s, 1).termination.as_deref(), Some(text));
    }
}

/// The three results and a bad one: a draw settles both sides, a bad text is refused whole.
#[test]
fn each_result_text_is_applied_and_a_bad_one_is_refused() {
    for (text, result, white_after) in [
        ("1-0", GameResult::WhiteWins, 1220),
        ("0-1", GameResult::BlackWins, 1180),
        ("1/2-1/2", GameResult::Draw, 1200),
    ] {
        let (_d, s) = live();
        let Payload::GameEnd(mut p) = game_end(false) else {
            panic!()
        };
        p.result = text.into();
        apply(&s, 1, vec![rec(5, Payload::GameEnd(p))]).unwrap();
        assert_eq!(game(&s, 1).result, Some(result));
        assert_eq!(
            block_on(s.get_player(pid(1))).unwrap().strict_elo,
            white_after,
            "{text}"
        );
    }
    let (d, s) = live();
    let before = dump(&raw(d.path()));
    let Payload::GameEnd(mut p) = game_end(false) else {
        panic!()
    };
    p.result = "2-0".into();
    let out = apply(&s, 1, vec![rec(5, Payload::GameEnd(p))]);
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    assert_eq!(before, dump(&raw(d.path())));
}

/// A `GameEnd` replay never clears `analysis_used` (FIND-D2R-02 latch): the flag was set
/// through `set_analysis_used`, and a `GameEnd` that says `false` leaves it set.
#[test]
fn a_game_end_replay_does_not_clear_analysis_used() {
    let (_d, s) = live();
    block_on(s.set_analysis_used(gid(1))).unwrap();
    apply(&s, 1, vec![rec(5, game_end(false))]).unwrap();
    assert!(game(&s, 1).analysis_used);
    apply(&s, 1, vec![rec(5, game_end(false))]).unwrap();
    assert!(game(&s, 1).analysis_used);
}

// ------------------------------------------------------------------------------ UT-ST-085

/// UT-ST-085: a `Voided` record on an `in_progress` row (a restore) voids it from the record:
/// `ended_at`, `content_hash`, `reset_player`, termination from the reason, slots and the lease
/// deleted, a job queued (the same function as the live void); `journal_seq` is the record's.
#[test]
fn ut_st_085_a_voided_record_voids_a_row_that_is_not_yet_voided() {
    let (d, s) = live();
    block_on(s.lease_upsert(gid(1), "cli:1".into())).unwrap();
    apply(&s, 1, vec![rec(9, voided(VoidReason::Reset, Some(1)))]).unwrap();
    let g = game(&s, 1);
    assert_eq!(g.status, GameStatus::Voided);
    assert_eq!(g.result, None);
    assert_eq!(g.termination.as_deref(), Some("voided by reset"));
    assert_eq!(g.voided_by, Some(pid(1)));
    assert_eq!(g.ended_at, Some(WallMs(8_000_000)));
    assert_eq!(g.content_hash.as_deref(), Some("sha256:v1:void"));
    assert_eq!(g.journal_seq, 9);
    let conn = raw(d.path());
    for (table, expected) in [
        ("unfinished_slot", 0),
        ("game_lease", 0),
        ("analysis_job", 1),
    ] {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, expected, "{table}");
    }
    // No rating row and no counter changed (D-30).
    let ratings: i64 = conn
        .query_row(
            "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ratings, 0);
}

/// An already voided row only gets `journal_seq`: the live void's values stay.
#[test]
fn ut_st_085_an_already_voided_row_only_gets_the_journal_sequence() {
    let (_d, s) = live();
    block_on(s.reset_player(
        pid(1),
        vec![chess_store::VoidSpec {
            game: gid(1),
            content_hash: "sha256:v1:live".into(),
        }],
        "{}".into(),
    ))
    .unwrap();
    let live_row = game(&s, 1);
    assert_eq!(live_row.journal_seq, 0, "reset_player cannot set it");
    apply(&s, 1, vec![rec(9, voided(VoidReason::Reset, Some(1)))]).unwrap();
    let after = game(&s, 1);
    assert_eq!(after.journal_seq, 9);
    assert_eq!(
        GameRecord {
            journal_seq: 0,
            ..after
        },
        live_row
    );
}

#[test]
fn an_unreadable_void_record_writes_the_unreadable_termination_and_no_voided_by() {
    let (_d, s) = live();
    apply(&s, 1, vec![rec(9, voided(VoidReason::Unreadable, None))]).unwrap();
    let g = game(&s, 1);
    assert_eq!(g.termination.as_deref(), Some("voided: unreadable"));
    assert_eq!(g.voided_by, None);
}

// -------------------------------------------------------------------------------- D-87

/// D-87: `GameEnd` then `Voided` in one batch is a voided journal. The `GameEnd` is never
/// finalized: no rating row, no counter, no result; the void carries the record's values.
#[test]
fn d87_a_game_end_followed_by_voided_is_not_finalized() {
    let (d, s) = live();
    apply(
        &s,
        1,
        vec![
            rec(2, jb::mv(0)),
            rec(3, game_end(true)),
            rec(4, voided(VoidReason::Reset, Some(1))),
        ],
    )
    .unwrap();
    let g = game(&s, 1);
    assert_eq!(g.status, GameStatus::Voided);
    assert_eq!(g.result, None);
    assert_eq!(g.white_elo_after, None);
    assert_eq!(
        g.white_time_used_ms, None,
        "the GameEnd totals were not written"
    );
    assert!(!g.analysis_used, "nor was its analysis flag");
    assert_eq!(g.journal_seq, 4);
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().strict_elo, 1200);
    assert_eq!(
        block_on(s.get_player(pid(1))).unwrap().strict_rated_games,
        0
    );
    let ratings: i64 = raw(d.path())
        .query_row(
            "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ratings, 0);
    assert_eq!(
        block_on(s.game_moves(gid(1))).unwrap().len(),
        1,
        "the moves are applied"
    );
}

/// A pair split over two calls: the `GameEnd` was applied (ratings settled), then a `Voided`
/// arrives. A void changes no rating (D-30), so the record is refused, the row and the ratings
/// are untouched and `journal_seq` stays: a `Voided` never un-settles a game
/// (FIND-D2R-05, FIND-F2-09). `void_game_repair_pair` is not reachable from the apply path.
#[test]
fn d87_a_voided_record_never_un_settles_a_game_whose_ratings_were_settled() {
    let (d, s) = live();
    apply(&s, 1, vec![rec(3, game_end(false))]).unwrap();
    let before = dump(&raw(d.path()));
    let refused = apply(&s, 1, vec![rec(4, voided(VoidReason::Reset, Some(1)))]);
    assert!(
        matches!(refused, Err(StoreError::InvariantViolated)),
        "{refused:?}"
    );
    assert_eq!(
        before,
        dump(&raw(d.path())),
        "nothing changed, not even journal_seq"
    );
    let g = game(&s, 1);
    assert_eq!(
        (g.status, g.result),
        (GameStatus::Completed, Some(GameResult::WhiteWins))
    );
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().strict_elo, 1220);
}

/// A completed game with no settled rating (unrated: self-play here) may be voided by a record:
/// the precondition protects ratings, not the status.
#[test]
fn a_completed_unrated_game_can_be_voided_by_a_record() {
    let (_d, s) = world();
    let mut ng = new_game(1, pid(1), pid(1), Matchup::HumanHuman, Mode::Strict);
    ng.rated = true;
    block_on(s.start_game(ng)).unwrap();
    apply(&s, 1, vec![rec(3, game_end(false))]).unwrap();
    assert_eq!(
        game(&s, 1).white_elo_after,
        None,
        "self-play is never rated"
    );
    apply(&s, 1, vec![rec(4, voided(VoidReason::Unreadable, None))]).unwrap();
    assert_eq!(game(&s, 1).status, GameStatus::Voided);
}

/// `Abort` then `Voided` is not a pair this design produces (D-87, RR-A-2): refused.
#[test]
fn an_abort_followed_by_voided_is_refused() {
    let (_d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap();
    apply(
        &s,
        1,
        vec![rec(
            3,
            Payload::Abort(AbortPayload {
                ended_at: 5,
                content_hash: HASH.into(),
            }),
        )],
    )
    .unwrap();
    assert_eq!(game(&s, 1).status, GameStatus::Aborted);
    let refused = apply(&s, 1, vec![rec(4, voided(VoidReason::Reset, Some(1)))]);
    assert!(
        matches!(refused, Err(StoreError::InvariantViolated)),
        "{refused:?}"
    );
    assert_eq!(game(&s, 1).status, GameStatus::Aborted);
}

// ------------------------------------------------------------------------- FIND-F2-04 and atomicity

/// A journal `GameEnd` or `Abort` with `ended_at <= 0` cannot be applied (FIND-F2-04, not
/// ruled): a typed error, no panic, no half record: the batch rolls back, also the `Move` that
/// came before it in the same batch; the store keeps working.
#[test]
fn a_terminal_record_with_a_non_positive_ended_at_is_refused_and_writes_nothing() {
    for bad in [0, -5] {
        let (d, s) = live();
        let before = dump(&raw(d.path()));
        let Payload::GameEnd(mut p) = game_end(false) else {
            panic!()
        };
        p.ended_at = bad;
        let out = apply(&s, 1, vec![rec(2, jb::mv(0)), rec(3, Payload::GameEnd(p))]);
        assert!(
            matches!(out, Err(StoreError::InvariantViolated)),
            "{bad}: {out:?}"
        );
        assert_eq!(before, dump(&raw(d.path())), "{bad}");
        // The refusal parked the game (D3 ruling 2026-11-24); the store itself keeps working.
        assert!(matches!(
            apply(&s, 1, vec![rec(2, jb::mv(0))]),
            Err(StoreError::GameParked { .. })
        ));
        block_on(s.register_player(human(5, "Dee"))).unwrap();
    }
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap();
    let before = dump(&raw(d.path()));
    let out = apply(
        &s,
        1,
        vec![rec(
            3,
            Payload::Abort(AbortPayload {
                ended_at: -1,
                content_hash: HASH.into(),
            }),
        )],
    );
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    assert_eq!(before, dump(&raw(d.path())));
}

/// D3 ruling 2026-11-24 (FIND-D3R-01, probe p14): a batch that the store refuses parks its game
/// on the first attempt. The batch behind it is refused with `GameParked` instead of applying
/// over the hole, so `journal_seq` never claims records the database did not get; the error of
/// the first batch reaches the caller; `sync_backlog` names the park; `parked_games` says what
/// failed; `unpark_game` drops the held batch (D-145) and the caller passes the whole journal.
#[test]
fn d3_a_refused_batch_parks_the_game_and_nothing_applies_over_the_hole() {
    let (d, s) = live();
    apply(&s, 1, vec![rec(2, jb::mv(0))]).unwrap();
    let Payload::Move(mut half) = jb::mv(1) else {
        panic!()
    };
    half.black_ms = None; // a half-null clock pair: the table CHECK refuses it
    let refused = apply(
        &s,
        1,
        vec![rec(3, Payload::Move(half)), rec(4, jb::clock(1, 2))],
    );
    assert!(
        matches!(refused, Err(StoreError::InvariantViolated)),
        "{refused:?}"
    );
    let behind = apply(&s, 1, vec![rec(5, jb::mv(2))]);
    assert!(
        matches!(behind, Err(StoreError::GameParked { game }) if game == gid(1)),
        "{behind:?}"
    );
    let rows: Vec<String> = block_on(s.game_moves(gid(1)))
        .unwrap()
        .into_iter()
        .map(|m| m.uci)
        .collect();
    assert_eq!(
        rows,
        vec!["u0".to_owned()],
        "ply 2 is not applied over the hole"
    );
    assert_eq!(game(&s, 1).journal_seq, 2, "no claim over the hole");
    let status = block_on(s.backlog_status()).unwrap();
    assert_eq!(
        (status.pending_batches, status.parked_games),
        (1, 1),
        "the refused batch is kept, the one behind it is not queued"
    );
    let sync = block_on(s.sync_backlog(std::time::Duration::from_secs(1)));
    assert!(
        matches!(sync, Err(StoreError::GameParked { game }) if game == gid(1)),
        "{sync:?}"
    );
    let parked = block_on(s.parked_games()).unwrap();
    assert_eq!(parked.len(), 1);
    assert_eq!(
        (parked[0].game, parked[0].cause, parked[0].pending_batches),
        (gid(1), chess_store::ParkCause::InvariantViolated, 1)
    );
    // Unpark drops the held batch; passing the bad batch again parks the game again.
    assert_eq!(block_on(s.unpark_game(gid(1))).unwrap(), Some(1));
    block_on(s.sync_backlog(std::time::Duration::from_secs(1))).unwrap();
    assert!(block_on(s.parked_games()).unwrap().is_empty());
    assert_eq!(
        game(&s, 1).journal_seq,
        2,
        "nothing was applied by the unpark"
    );
    assert!(
        block_on(s.unpark_game(gid(9))).unwrap().is_none(),
        "not parked: false"
    );
    drop(d);
}

/// The way out of a park that has a cure: a Player the batch names is missing, the park says
/// `NotFound`, the Player is registered, `unpark_game` drops the held queue, the whole journal is
/// passed again in one call, and the game is current with no record skipped (D-145, D-152).
#[test]
fn d3_unpark_after_the_cause_is_fixed_applies_the_parked_batch() {
    let (_d, s) = live();
    let missing = apply(
        &s,
        1,
        vec![
            rec(2, jb::mv(0)),
            rec(3, voided(VoidReason::Reset, Some(77))),
        ],
    );
    assert!(
        matches!(
            missing,
            Err(StoreError::NotFound {
                entity: Entity::Player,
                ..
            })
        ),
        "{missing:?}"
    );
    let parked = block_on(s.parked_games()).unwrap();
    assert_eq!(parked[0].cause, chess_store::ParkCause::NotFound);
    assert!(matches!(
        apply(&s, 1, vec![rec(4, jb::mv(1))]),
        Err(StoreError::GameParked { .. })
    ));
    block_on(s.register_player(human(77, "Cy"))).unwrap();
    assert_eq!(block_on(s.unpark_game(gid(1))).unwrap(), Some(1));
    block_on(s.sync_backlog(std::time::Duration::from_secs(5))).unwrap();
    assert_eq!(game(&s, 1).journal_seq, 0, "the unpark applied nothing");
    // The caller passes the whole journal again, in one call.
    let again = apply(
        &s,
        1,
        vec![
            rec(2, jb::mv(0)),
            rec(3, voided(VoidReason::Reset, Some(77))),
        ],
    )
    .unwrap();
    assert_eq!(again, ApplyOutcome::Applied);
    let g = game(&s, 1);
    assert_eq!(g.status, GameStatus::Voided);
    assert_eq!(g.journal_seq, 3);
    assert!(block_on(s.parked_games()).unwrap().is_empty());
    assert_eq!(block_on(s.backlog_status()).unwrap().pending_batches, 0);
}

/// All of a batch or none of it: the last record fails, the earlier records leave no row.
#[test]
fn a_batch_is_atomic() {
    let (d, s) = live();
    let before = dump(&raw(d.path()));
    let out = apply(
        &s,
        1,
        vec![
            rec(2, jb::mv(0)),
            rec(3, jb::clock(1, 2)),
            rec(4, voided(VoidReason::Reset, Some(77))),
        ],
    );
    assert!(
        matches!(
            out,
            Err(StoreError::NotFound {
                entity: Entity::Player,
                ..
            })
        ),
        "{out:?}"
    );
    assert_eq!(before, dump(&raw(d.path())));
}

#[test]
fn a_batch_for_a_game_without_a_row_is_not_found_and_an_empty_batch_is_a_no_op() {
    let (d, s) = world();
    let before = dump(&raw(d.path()));
    let out = apply(&s, 5, vec![rec(2, jb::mv(0))]);
    assert!(
        matches!(
            out,
            Err(StoreError::NotFound {
                entity: Entity::Game,
                ..
            })
        ),
        "{out:?}"
    );
    assert_eq!(apply(&s, 5, vec![]).unwrap(), ApplyOutcome::Applied);
    assert_eq!(before, dump(&raw(d.path())));
}

#[test]
fn records_must_be_in_strictly_increasing_sequence_order() {
    // A refused batch parks its game (D3 ruling 2026-11-24), so each case has its own store.
    for records in [
        vec![rec(3, jb::mv(0)), rec(3, jb::mv(1))],
        vec![rec(4, jb::mv(0)), rec(3, jb::mv(1))],
    ] {
        let (d, s) = live();
        let before = dump(&raw(d.path()));
        let out = apply(&s, 1, records);
        assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
        assert_eq!(before, dump(&raw(d.path())));
    }
}

/// `journal_seq` never goes down through `apply_records`: a late, older batch leaves it.
#[test]
fn a_batch_never_lowers_the_journal_sequence() {
    let (_d, s) = live();
    apply(&s, 1, vec![rec(20, jb::mv(0))]).unwrap();
    apply(&s, 1, vec![rec(5, Payload::Pause)]).unwrap();
    assert_eq!(game(&s, 1).journal_seq, 20);
    apply(&s, 1, vec![rec(21, Payload::Pause)]).unwrap();
    assert_eq!(game(&s, 1).journal_seq, 21);
}

// -------------------------------------------------------------------------------- idempotence

/// Applying the same batches again changes no table: the journal of a finished, rated game with
/// moves, a takeback and clocks, applied twice, then a third time in pieces.
#[test]
fn applying_a_journal_again_changes_nothing() {
    let (d, s) = live();
    let records = vec![
        rec(2, jb::mv(0)),
        rec(3, jb::mv(1)),
        rec(4, jb::mv(2)),
        rec(5, Payload::Takeback(TakebackPayload { to_ply: 2 })),
        rec(6, jb::mv(2)),
        rec(7, jb::clock(9, 8)),
        rec(8, game_end(true)),
    ];
    apply(&s, 1, records.clone()).unwrap();
    let once = dump(&raw(d.path()));
    apply(&s, 1, records.clone()).unwrap();
    assert_eq!(once, dump(&raw(d.path())), "the whole journal again");
    // One record at a time, in order: the same end state (the clocks pass through the values of
    // the older records on the way, as any in-order replay does).
    for record in records {
        apply(&s, 1, vec![record]).unwrap();
    }
    assert_eq!(once, dump(&raw(d.path())), "one record at a time");
    assert_eq!(
        block_on(s.get_player(pid(1))).unwrap().strict_rated_games,
        1,
        "rated once"
    );
}

// -------------------------------------------------------------------------- Start and UT-ST-089

fn start_payload() -> Payload {
    jb::start(1)
}

/// UT-ST-089: a restore of a `Start`-only journal creates the row with `started_at` equal to
/// the record's `wall_ms` (SF-16), the frozen `rated` and `self_play`, the clock from the
/// `clock_spec`, `journal_seq` the record's; a second apply changes nothing.
#[test]
fn ut_st_089_a_start_only_journal_creates_the_row_with_started_at_equal_to_wall_ms() {
    let (d, s) = world();
    let start = JournalRecord {
        seq: 1,
        wall_ms: 7_654_321,
        payload: start_payload(),
    };
    apply(&s, 1, vec![start.clone()]).unwrap();
    let g = game(&s, 1);
    assert_eq!(g.started_at, Some(WallMs(7_654_321)));
    assert_eq!(g.status, GameStatus::InProgress);
    assert_eq!((g.white, g.black), (Some(pid(1)), Some(pid(2))));
    assert_eq!(
        (g.matchup, g.mode),
        (Some(Matchup::HumanHuman), Some(Mode::Strict))
    );
    assert!(g.rated && !g.self_play && !g.custom_start);
    assert_eq!(g.start_fen, None);
    assert_eq!(g.clock.map(|c| c.to_spec_text()).as_deref(), Some("300+2"));
    assert_eq!(
        (g.white_clock_ms, g.black_clock_ms),
        (Some(300_000), Some(300_000))
    );
    assert_eq!(g.journal_seq, 1);
    let conn = raw(d.path());
    let slots: i64 = conn
        .query_row("SELECT count(*) FROM unfinished_slot", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        slots, 0,
        "restore inserts slots in its own last pass (SDD 3.11.10, R-ST-7)"
    );
    let once = dump(&conn);
    apply(&s, 1, vec![start]).unwrap();
    assert_eq!(once, dump(&conn));
}

/// A live row keeps its own `started_at` and slots when a `Start` is applied (the journal's
/// `wall_ms` is the same stamp by D-84; a different one never overwrites).
#[test]
fn a_start_record_for_an_existing_row_changes_nothing_but_the_sequence() {
    let (d, s) = live();
    let before = dump(&raw(d.path()));
    apply(
        &s,
        1,
        vec![JournalRecord {
            seq: 1,
            wall_ms: 99,
            payload: start_payload(),
        }],
    )
    .unwrap();
    let after = dump(&raw(d.path()));
    let changed: Vec<&String> = before.iter().filter(|r| !after.contains(r)).collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(game(&s, 1).started_at, Some(WallMs(5_001)));
}

/// A restore of a full journal: `Start`, moves, `GameEnd` in one batch rebuilds the game from
/// nothing, and equals the live game on every column of `GAME` except none.
#[test]
fn a_whole_journal_restores_a_game_that_has_no_row() {
    let (da, a) = live();
    let (db, b) = world();
    let records = vec![
        JournalRecord {
            seq: 1,
            wall_ms: 5_001,
            payload: start_payload(),
        },
        rec(2, jb::mv(0)),
        rec(3, jb::mv(1)),
        rec(4, game_end(false)),
    ];
    // The live game: the store writes the row, then the same records follow.
    apply(&a, 1, records[1..].to_vec()).unwrap();
    apply(&b, 1, records).unwrap();
    // The live row has no clock (`new_game`), the restored one has `300+2`: compare the rest.
    let sql = "SELECT id, uuid, white_id, black_id, rated, self_play, custom_start, status, \
               result, termination, ended_at, white_time_used_ms, black_time_used_ms, \
               total_active_ms, content_hash, journal_seq, white_elo_before, white_elo_after \
               FROM game";
    assert_eq!(rows_of(&raw(da.path()), sql), rows_of(&raw(db.path()), sql));
    assert_eq!(
        rows_of(&raw(da.path()), "SELECT * FROM move ORDER BY ply"),
        rows_of(&raw(db.path()), "SELECT * FROM move ORDER BY ply")
    );
}

#[test]
fn a_start_with_a_wrong_game_an_unknown_player_or_wrong_kinds_is_refused() {
    // Every refusal parks the game, so each case runs on a store of its own and must leave the
    // database as it was.
    let refuse = |payload: Payload| {
        let (d, s) = world();
        let before = dump(&raw(d.path()));
        let out = apply(&s, 1, vec![rec(1, payload)]);
        assert_eq!(before, dump(&raw(d.path())));
        out
    };
    // The record names game 2, the call names game 1.
    let out = refuse(jb::start(2));
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    // An unknown player.
    let Payload::Start(mut p) = jb::start(1) else {
        panic!()
    };
    p.white = UuidText(Uuid::from_u128(404));
    let out = refuse(Payload::Start(p));
    assert!(
        matches!(
            out,
            Err(StoreError::NotFound {
                entity: Entity::Player,
                ..
            })
        ),
        "{out:?}"
    );
    // The record says computer, the player row says human.
    let Payload::Start(mut p) = jb::start(1) else {
        panic!()
    };
    p.black_kind = SeatKind::Computer;
    p.matchup = "hc".into();
    let out = refuse(Payload::Start(p));
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    // A mode the store does not know.
    let Payload::Start(mut p) = jb::start(1) else {
        panic!()
    };
    p.mode = "blitz".into();
    assert!(refuse(Payload::Start(p)).is_err());
    // A custom start whose flag and FEN disagree is stopped by the table.
    let Payload::Start(mut p) = jb::start(1) else {
        panic!()
    };
    p.custom_start = true;
    let out = refuse(Payload::Start(p));
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
}

/// D3-06 ruling 2026-11-24: `Start.matchup` is checked against the two kinds at apply too. A
/// text that is not the code of the kinds is refused (a hostile file; the codec refuses it
/// earlier), nothing is written, and the matching text applies.
#[test]
fn d3_a_start_whose_matchup_text_disagrees_with_the_kinds_is_refused_at_apply() {
    let start = |matchup: &str| {
        let Payload::Start(mut p) = jb::start(1) else {
            panic!()
        };
        p.white = UuidText(Uuid::from_u128(1));
        p.black = UuidText(Uuid::from_u128(10));
        p.black_kind = SeatKind::Computer;
        p.mode = "training".into();
        p.clock_spec = None::<ClockText>;
        p.matchup = matchup.into();
        Payload::Start(p)
    };
    for bad in ["any text", "hh", "cc", "human-computer", ""] {
        let (d, s) = world();
        let before = dump(&raw(d.path()));
        let out = apply(&s, 1, vec![rec(1, start(bad))]);
        assert!(
            matches!(out, Err(StoreError::InvariantViolated)),
            "{bad}: {out:?}"
        );
        assert_eq!(before, dump(&raw(d.path())), "{bad}: nothing written");
    }
    let (_d, s) = world();
    apply(&s, 1, vec![rec(1, start("hc"))]).unwrap();
    assert_eq!(game(&s, 1).matchup, Some(Matchup::HumanComputer));
}

/// The same check for a `Start` of a game that already has its row (the no-op arm).
#[test]
fn d3_a_second_start_with_a_wrong_matchup_or_a_non_positive_wall_ms_is_refused() {
    let wrong_matchup = {
        let Payload::Start(mut p) = jb::start(1) else {
            panic!()
        };
        p.matchup = "cc".into();
        rec(1, Payload::Start(p))
    };
    let cases = [
        wrong_matchup,
        JournalRecord {
            seq: 1,
            wall_ms: 0,
            payload: jb::start(1),
        },
        JournalRecord {
            seq: 1,
            wall_ms: -3,
            payload: jb::start(1),
        },
    ];
    for record in cases {
        let (d, s) = live();
        let before = dump(&raw(d.path()));
        let out = apply(&s, 1, vec![record]);
        assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
        assert_eq!(before, dump(&raw(d.path())));
    }
}

/// FIND-D3R-07: a `Start` with `wall_ms <= 0` would store `started_at <= 0`; refused, no row.
#[test]
fn d3_a_start_with_a_non_positive_wall_ms_is_refused_and_creates_no_game() {
    for bad in [0, -3, i64::MIN] {
        let (d, s) = world();
        let before = dump(&raw(d.path()));
        let record = JournalRecord {
            seq: 1,
            wall_ms: bad,
            payload: jb::start(1),
        };
        let out = apply(&s, 1, vec![record]);
        assert!(
            matches!(out, Err(StoreError::InvariantViolated)),
            "{bad}: {out:?}"
        );
        assert_eq!(before, dump(&raw(d.path())), "{bad}");
    }
}

/// FIND-D3R-07 and the D3 ruling: a `Voided` with `ended_at <= 0` is refused like `GameEnd` and
/// `Abort`; the whole batch rolls back (the `Move` before it too) and the row stays as it was.
#[test]
fn d3_a_voided_record_with_a_non_positive_ended_at_is_refused_and_writes_nothing() {
    for bad in [0, -7, i64::MIN] {
        let (d, s) = live();
        let before = dump(&raw(d.path()));
        let Payload::Voided(mut v) = voided(VoidReason::Unreadable, None) else {
            panic!()
        };
        v.ended_at = bad;
        let out = apply(&s, 1, vec![rec(2, jb::mv(0)), rec(3, Payload::Voided(v))]);
        assert!(
            matches!(out, Err(StoreError::InvariantViolated)),
            "{bad}: {out:?}"
        );
        assert_eq!(before, dump(&raw(d.path())), "{bad}");
        assert_eq!(game(&s, 1).status, GameStatus::InProgress);
    }
}

/// The end-time rule covers every terminal record, also one that changes nothing: a `Voided`
/// meeting a row that is already voided, and a `GameEnd` that a later `Voided` supersedes.
#[test]
fn d3_the_end_time_rule_also_covers_a_record_that_would_change_no_row() {
    let (d, s) = live();
    apply(&s, 1, vec![rec(2, voided(VoidReason::Unreadable, None))]).unwrap();
    let before = dump(&raw(d.path()));
    let Payload::Voided(mut again) = voided(VoidReason::Unreadable, None) else {
        panic!()
    };
    again.ended_at = 0;
    let out = apply(&s, 1, vec![rec(3, Payload::Voided(again))]);
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    assert_eq!(before, dump(&raw(d.path())));

    let (d, s) = live();
    let before = dump(&raw(d.path()));
    let Payload::GameEnd(mut end) = game_end(false) else {
        panic!()
    };
    end.ended_at = -1;
    let out = apply(
        &s,
        1,
        vec![
            rec(2, Payload::GameEnd(end)),
            rec(3, voided(VoidReason::Reset, Some(1))),
        ],
    );
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    assert_eq!(before, dump(&raw(d.path())));
}

/// The D-87 pair with a bad `Voided`: the superseded `GameEnd` is not finalized either, and
/// nothing is half written.
#[test]
fn d3_a_game_end_voided_pair_with_a_bad_void_time_writes_nothing() {
    let (d, s) = live();
    let before = dump(&raw(d.path()));
    let Payload::Voided(mut v) = voided(VoidReason::Reset, Some(1)) else {
        panic!()
    };
    v.ended_at = 0;
    let out = apply(
        &s,
        1,
        vec![rec(2, game_end(false)), rec(3, Payload::Voided(v))],
    );
    assert!(matches!(out, Err(StoreError::InvariantViolated)), "{out:?}");
    assert_eq!(before, dump(&raw(d.path())));
}

/// Opening a second handle on the same directory is not needed for these tests, but the options
/// type stays constructible with its defaults (guards the private field added in part D3).
#[test]
fn store_options_new_still_builds() {
    let d = TempDir::new().unwrap();
    let _ = StoreOptions::new(chess_store::DataDir::new(d.path()));
}
