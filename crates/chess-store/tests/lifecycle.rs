//! Part D2: game lifecycle writes and ratings through the public `Store` API on a real
//! temporary database: `finalize_game` (UT-ST-050..056, UT-ST-035), `abort_game`,
//! `void_unreadable_game` (UT-ST-086 void half), `reset_player` (CT-ST-028),
//! `check_rating_invariant`, and fault injection in each command (CT-ST-007, feature
//! `test-support`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use chess_domain::{
    EloCalc, GameResult, GameStatus, Matchup, Mode, PlayerId, RatedSide, Track, WallMs,
    check_history,
};
use chess_store::{
    AbortRequest, EndOutcome, Entity, FinalizeOutcome, FinalizeRequest, GameRecord, LeaseBeat,
    RatingViolationKind, Store, StoreError, VoidSpec,
};
use common::{block_on, computer, gid, human, new_game, open_in, pid, raw};
use rusqlite::Connection;
use tempfile::TempDir;

const HASH: &str = "sha256:v1:aaaa";

fn at(n: u128) -> i64 {
    i64::try_from(n).unwrap()
}

/// Players: humans 1, 2, 3 (1200 / 1200); computers 10, 11 (800 / 800).
fn world() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    for (n, name) in [(1, "Ann"), (2, "Bob"), (3, "Cy")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![
        computer(10, "Bot A", 1, 0, 3),
        computer(11, "Bot B", 1, 1, 4),
    ]))
    .unwrap();
    (dir, s)
}

fn req(game: u128, result: GameResult, ended_at: i64) -> FinalizeRequest {
    FinalizeRequest {
        game: gid(game),
        result,
        termination: "checkmate".into(),
        ended_at: WallMs(ended_at),
        white_time_used_ms: 111,
        black_time_used_ms: 222,
        total_active_ms: 333,
        eco: Some("C20".into()),
        opening_name: Some("King's Pawn".into()),
        analysis_used: true,
        content_hash: HASH.into(),
        final_journal_seq: 42,
    }
}

fn start(s: &Store, n: u128, w: PlayerId, b: PlayerId, matchup: Matchup, mode: Mode) {
    block_on(s.start_game(new_game(n, w, b, matchup, mode))).unwrap();
}

fn game(s: &Store, n: u128) -> GameRecord {
    block_on(s.get_game(gid(n))).unwrap()
}

fn elos(s: &Store, p: u128) -> (i32, i32, u32, u32) {
    let r = block_on(s.get_player(pid(p))).unwrap();
    (
        r.strict_elo,
        r.training_elo,
        r.strict_rated_games,
        r.training_rated_games,
    )
}

fn count(d: &TempDir, sql: &str) -> i64 {
    raw(d.path()).query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Every row of every table, as text, in a fixed order: "the content" of CT-ST-007.
fn dump(conn: &Connection) -> Vec<String> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
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
        for row in rows {
            out.push(format!("{table}: {row}"));
        }
    }
    out.sort();
    out
}

fn points(s: &Store, p: u128, track: Track) -> Vec<chess_domain::RatingPoint> {
    block_on(s.rating_history(pid(p), track)).unwrap()
}

// ---------------------------------------------------------------- finalize: ratings

#[test]
fn ut_st_050_strict_rated_game_settles_both_sides_and_writes_rows_and_game_columns() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    // Hand-computed: E(1200 v 800) = 0.909090..; K = 40 on both sides (0 rated games).
    // White wins: +40 * 0.0909 = 3.64 -> +4; black -3.64 -> -4.
    let out = block_on(s.finalize_game(req(1, GameResult::WhiteWins, 9_000))).unwrap();
    let FinalizeOutcome::Finalized {
        white: Some(w),
        black: Some(b),
    } = out
    else {
        panic!("{out:?}")
    };
    assert_eq!(
        (w.player, w.track, w.before, w.after, w.k),
        (pid(1), Track::Strict, 1200, 1204, 40)
    );
    assert_eq!(
        (b.player, b.track, b.before, b.after, b.k),
        (pid(10), Track::Strict, 800, 796, 40)
    );
    assert_eq!(elos(&s, 1), (1204, 1200, 1, 0));
    assert_eq!(elos(&s, 10), (796, 800, 1, 0));
    let g = game(&s, 1);
    assert_eq!(g.status, GameStatus::Completed);
    assert_eq!(g.result, Some(GameResult::WhiteWins));
    assert_eq!(g.termination.as_deref(), Some("checkmate"));
    assert_eq!(g.ended_at, Some(WallMs(9_000)));
    assert_eq!(
        (
            g.white_time_used_ms,
            g.black_time_used_ms,
            g.total_active_ms
        ),
        (Some(111), Some(222), Some(333))
    );
    assert_eq!(g.eco.as_deref(), Some("C20"));
    assert_eq!(g.opening_name.as_deref(), Some("King's Pawn"));
    assert!(g.analysis_used);
    assert_eq!(g.content_hash.as_deref(), Some(HASH));
    assert_eq!(g.journal_seq, 42);
    assert_eq!(
        (
            g.white_elo_before,
            g.white_elo_after,
            g.black_elo_before,
            g.black_elo_after
        ),
        (Some(1200), Some(1204), Some(800), Some(796))
    );
    // Slot and lease rows gone, one queued job.
    assert_eq!(count(&d, "SELECT count(*) FROM unfinished_slot"), 0);
    assert_eq!(
        count(
            &d,
            "SELECT count(*) FROM analysis_job WHERE state = 'queued'"
        ),
        1
    );
    // The history rows carry the game and the end time.
    let hist = points(&s, 1, Track::Strict);
    let last = hist.last().unwrap();
    assert_eq!(
        (
            last.game_id,
            last.at,
            last.rating_before,
            last.rating_after,
            last.k
        ),
        (Some(gid(1)), WallMs(9_000), 1200, 1204, 40)
    );
}

#[test]
fn ut_st_051_rounding_vectors_black_win_and_draw() {
    let (_d, s) = world();
    // Black wins: white 40 * (0 - 0.9091) = -36.36 -> -36; black 40 * (1 - 0.0909) = 36.36 -> 36.
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(1, GameResult::BlackWins, 1))).unwrap();
    assert_eq!(elos(&s, 1).0, 1164);
    assert_eq!(elos(&s, 10).0, 836);
    // Draw, equal ratings, K 40: 40 * (0.5 - 0.5) = 0.
    start(&s, 2, pid(2), pid(3), Matchup::HumanHuman, Mode::Strict);
    block_on(s.finalize_game(req(2, GameResult::Draw, 2))).unwrap();
    assert_eq!((elos(&s, 2).0, elos(&s, 3).0), (1200, 1200));
    // Equal ratings, a win: 40 * 0.5 = 20 exactly.
    start(&s, 3, pid(2), pid(3), Matchup::HumanHuman, Mode::Strict);
    block_on(s.finalize_game(req(3, GameResult::WhiteWins, 3))).unwrap();
    assert_eq!((elos(&s, 2).0, elos(&s, 3).0), (1220, 1180));
    // Draw, unequal: 1220 v 1180 -> E = 0.5575; white 40*(0.5-0.5575) = -2.3 -> -2.
    start(&s, 4, pid(2), pid(3), Matchup::HumanHuman, Mode::Strict);
    block_on(s.finalize_game(req(4, GameResult::Draw, 4))).unwrap();
    assert_eq!((elos(&s, 2).0, elos(&s, 3).0), (1218, 1182));
}

#[test]
fn ut_st_052_training_track_is_separate_and_k_counters_are_per_track() {
    let (d, s) = world();
    // Seed 29 rated strict games for Ann; training stays at 0.
    raw(d.path())
        .execute(
            "UPDATE player SET strict_rated_games = 29 WHERE uuid = ?1",
            [pid(1).as_uuid().to_string()],
        )
        .unwrap();
    start(
        &s,
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    );
    let out = block_on(s.finalize_game(req(1, GameResult::WhiteWins, 1))).unwrap();
    let FinalizeOutcome::Finalized { white: Some(w), .. } = out else {
        panic!("{out:?}")
    };
    // Training: K from the training counter (0 games) = 40; the strict side is untouched.
    assert_eq!((w.track, w.k, w.after), (Track::Training, 40, 1204));
    assert_eq!(elos(&s, 1), (1200, 1204, 29, 1));
    // The strict game: 29 games -> K 40 -> counter 30; the next one uses K 32.
    start(&s, 2, pid(1), pid(11), Matchup::HumanComputer, Mode::Strict);
    let FinalizeOutcome::Finalized {
        white: Some(w2), ..
    } = block_on(s.finalize_game(req(2, GameResult::Draw, 2))).unwrap()
    else {
        panic!()
    };
    assert_eq!(w2.k, 40);
    assert_eq!(elos(&s, 1).2, 30);
    start(&s, 3, pid(1), pid(11), Matchup::HumanComputer, Mode::Strict);
    let FinalizeOutcome::Finalized {
        white: Some(w3), ..
    } = block_on(s.finalize_game(req(3, GameResult::Draw, 3))).unwrap()
    else {
        panic!()
    };
    assert_eq!(w3.k, 32, "30 rated games: K = 32");
    assert_eq!(elos(&s, 1).2, 31);
    assert_eq!(elos(&s, 1).3, 1, "the training counter did not move");
}

#[test]
fn ut_st_053_self_play_custom_start_and_unrated_games_change_no_rating() {
    let (d, s) = world();
    // Self-play, rated flag set: unrated (ELO-2).
    start(&s, 1, pid(1), pid(1), Matchup::HumanHuman, Mode::Strict);
    // Custom start (the schema forces rated = 0).
    let mut custom = new_game(2, pid(2), pid(10), Matchup::HumanComputer, Mode::Strict);
    custom.rated = false;
    custom.start_fen = Some("8/8/8/8/8/8/8/K1k5 w - - 0 1".into());
    block_on(s.start_game(custom)).unwrap();
    // Plain unrated.
    let mut unrated = new_game(3, pid(3), pid(11), Matchup::HumanComputer, Mode::Strict);
    unrated.rated = false;
    block_on(s.start_game(unrated)).unwrap();
    let before = (
        elos(&s, 1),
        elos(&s, 2),
        elos(&s, 3),
        elos(&s, 10),
        elos(&s, 11),
    );
    let rows_before = count(&d, "SELECT count(*) FROM rating_history");
    for n in 1..=3 {
        let out = block_on(s.finalize_game(req(n, GameResult::WhiteWins, 10 + at(n)))).unwrap();
        assert_eq!(
            out,
            FinalizeOutcome::Finalized {
                white: None,
                black: None
            }
        );
        let g = game(&s, n);
        assert_eq!(g.status, GameStatus::Completed);
        assert_eq!(
            (
                g.white_elo_before,
                g.white_elo_after,
                g.black_elo_before,
                g.black_elo_after
            ),
            (None, None, None, None),
            "game {n}"
        );
    }
    assert_eq!(
        before,
        (
            elos(&s, 1),
            elos(&s, 2),
            elos(&s, 3),
            elos(&s, 10),
            elos(&s, 11)
        )
    );
    assert_eq!(
        rows_before,
        count(&d, "SELECT count(*) FROM rating_history"),
        "no rating row"
    );
    assert!(game(&s, 1).self_play && game(&s, 2).custom_start);
    // Their slots are gone too.
    assert_eq!(count(&d, "SELECT count(*) FROM unfinished_slot"), 0);
}

#[test]
fn ut_st_054_finalize_twice_is_already_final_and_changes_nothing() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(1, GameResult::WhiteWins, 5))).unwrap();
    let snapshot = dump(&raw(d.path()));
    let mut again = req(1, GameResult::BlackWins, 99);
    again.content_hash = "sha256:v1:bbbb".into();
    again.total_active_ms = 1;
    assert_eq!(
        block_on(s.finalize_game(again)).unwrap(),
        FinalizeOutcome::AlreadyFinal
    );
    assert_eq!(snapshot, dump(&raw(d.path())));
}

#[test]
fn ut_st_055_history_sums_to_the_rating_and_the_invariant_holds() {
    let (d, s) = world();
    let results = [
        GameResult::WhiteWins,
        GameResult::Draw,
        GameResult::BlackWins,
        GameResult::WhiteWins,
        GameResult::BlackWins,
    ];
    for (i, r) in results.iter().enumerate() {
        let n = i as u128 + 1;
        start(&s, n, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
        block_on(s.finalize_game(req(n, *r, 10_000_000 + at(n)))).unwrap();
    }
    for p in [1, 10] {
        let hist = points(&s, p, Track::Strict);
        assert_eq!(hist.len(), 6, "initial point plus five games");
        let sum: i32 = hist.iter().map(|h| h.rating_after - h.rating_before).sum();
        assert_eq!(hist[0].rating_before + sum, elos(&s, p).0);
        check_history(&hist, Track::Strict).unwrap();
    }
    assert_eq!(
        block_on(s.check_rating_invariant(vec![pid(1), pid(10), pid(2)])).unwrap(),
        vec![]
    );
    assert_eq!(
        count(&d, "SELECT count(*) FROM game WHERE status='completed'"),
        5
    );
}

#[test]
fn ut_st_056_two_games_against_one_computer_give_the_in_order_result() {
    // Ann (1200) beats the computer in game 1; Bob (1000) loses to it in game 2.
    let play = |first: u128| -> (i32, i32, i32) {
        let (d, s) = world();
        set(
            &d,
            "UPDATE player SET strict_elo = 1000 WHERE name = 'Bob'; \
             UPDATE rating_history SET rating_before = 1000, rating_after = 1000 \
             WHERE track = 'strict' AND player_id = (SELECT id FROM player WHERE name = 'Bob')",
        );
        start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
        start(&s, 2, pid(2), pid(10), Matchup::HumanComputer, Mode::Strict);
        let result = |n: u128| {
            if n == 1 {
                GameResult::WhiteWins
            } else {
                GameResult::BlackWins
            }
        };
        let second = 3 - first;
        block_on(s.finalize_game(req(first, result(first), 10_000_000))).unwrap();
        block_on(s.finalize_game(req(second, result(second), 20_000_000))).unwrap();
        assert_eq!(
            block_on(s.check_rating_invariant(vec![pid(1), pid(2), pid(10)])).unwrap(),
            vec![]
        );
        check_history(&points(&s, 10, Track::Strict), Track::Strict).unwrap();
        (elos(&s, 10).0, elos(&s, 1).0, elos(&s, 2).0)
    };
    // The expected values come from the domain function applied in the stated order.
    let side = |rating, rated_games_before| RatedSide {
        rating,
        rated_games_before,
    };
    let (ann, bob, bot) = (1200, 1000, 800);
    let ann_game = |computer: RatedSide| {
        EloCalc::settle(side(ann, 0), computer, GameResult::WhiteWins).unwrap()
    };
    let bob_game = |computer: RatedSide| {
        EloCalc::settle(side(bob, 0), computer, GameResult::BlackWins).unwrap()
    };
    // Order 1 then 2.
    let g1 = ann_game(side(bot, 0));
    let g2 = bob_game(side(g1.black.after, 1));
    let in_order_12 = (g2.black.after, g1.white.after, g2.white.after);
    // Order 2 then 1.
    let h2 = bob_game(side(bot, 0));
    let h1 = ann_game(side(h2.black.after, 1));
    let in_order_21 = (h1.black.after, h1.white.after, h2.white.after);
    assert_eq!(play(1), in_order_12);
    assert_eq!(play(2), in_order_21);
    assert_ne!(
        in_order_12, in_order_21,
        "the order matters, so the test can tell"
    );
}

/// D2-02 (Brian, 2026-11-22): the store returns rating rows in `id` (insertion) order; the domain
/// orders them by `(at, id)`; the two agree only while finalizes are applied in `ended_at` order.
/// This pins both halves, so a change on either side fails here.
#[test]
fn d2_02_the_store_orders_rating_rows_by_id_and_the_domain_by_at_then_id() {
    let play = |first_ended_after: i64, second_ended_after: i64| {
        let (_d, s) = world();
        let initial = points(&s, 1, Track::Strict)[0].at.0;
        start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
        block_on(s.finalize_game(req(1, GameResult::WhiteWins, initial + first_ended_after)))
            .unwrap();
        start(&s, 2, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
        block_on(s.finalize_game(req(2, GameResult::BlackWins, initial + second_ended_after)))
            .unwrap();
        points(&s, 1, Track::Strict)
    };
    // In ended_at order: the id order is the (at, id) order and the domain check passes.
    let ordered = play(1_000, 2_000);
    let ids: Vec<u64> = ordered.iter().map(|p| p.id).collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "store: id order {ids:?}"
    );
    assert_eq!(check_history(&ordered, Track::Strict), Ok(()));
    // The clock steps back: game 2 is applied second but ended earlier. The store still returns
    // insertion order; the domain re-orders by (at, id) and reports a gap.
    let stepped = play(2_000, 1_000);
    let games: Vec<Option<_>> = stepped.iter().map(|p| p.game_id).collect();
    assert_eq!(games, [None, Some(gid(1)), Some(gid(2))], "store: id order");
    assert!(
        stepped[1].at.0 > stepped[2].at.0,
        "at goes backwards in id order"
    );
    let series = chess_domain::rating_series(&stepped, Track::Strict, None);
    let by_at: Vec<Option<_>> = series.iter().map(|p| p.game_id).collect();
    assert_eq!(
        by_at,
        [None, Some(gid(2)), Some(gid(1))],
        "domain: (at, id)"
    );
    assert!(matches!(
        check_history(&stepped, Track::Strict),
        Err(chess_domain::HistoryError::Gap { .. })
    ));
}

#[test]
fn finalize_refuses_an_empty_hash_an_unknown_game_and_an_imported_row() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    let mut bad = req(1, GameResult::Draw, 1);
    bad.content_hash = String::new();
    let before = dump(&raw(d.path()));
    assert!(matches!(
        block_on(s.finalize_game(bad)),
        Err(StoreError::InvariantViolated)
    ));
    assert_eq!(before, dump(&raw(d.path())));
    assert!(matches!(
        block_on(s.finalize_game(req(77, GameResult::Draw, 1))),
        Err(StoreError::NotFound {
            entity: Entity::Game,
            ..
        })
    ));
    raw(d.path())
        .execute(
            "INSERT INTO game (uuid, rated, self_play, status, content_hash) \
             VALUES (?1, 0, 0, 'imported', 'sha256:v1:cc')",
            [gid(5).as_uuid().to_string()],
        )
        .unwrap();
    assert!(matches!(
        block_on(s.finalize_game(req(5, GameResult::Draw, 1))),
        Err(StoreError::InvariantViolated)
    ));
}

#[test]
fn a_game_voided_before_its_game_end_arrives_stays_voided_d_67_v() {
    let (_d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.void_unreadable_game(gid(1), HASH.into())).unwrap();
    assert_eq!(
        block_on(s.finalize_game(req(1, GameResult::WhiteWins, 5))).unwrap(),
        FinalizeOutcome::AlreadyFinal
    );
    assert_eq!(game(&s, 1).status, GameStatus::Voided);
    assert_eq!(elos(&s, 1), (1200, 1200, 0, 0));
}

// ---------------------------------------------------------------- abort, void, reset

#[test]
fn abort_sets_the_hash_releases_slots_queues_a_job_and_rates_nothing() {
    let (d, s) = world();
    start(
        &s,
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    );
    block_on(s.lease_upsert(gid(1), s.owner())).unwrap();
    assert_eq!(count(&d, "SELECT count(*) FROM unfinished_slot"), 2);
    let abort = AbortRequest {
        game: gid(1),
        ended_at: WallMs(77),
        content_hash: HASH.into(),
        final_journal_seq: 9,
    };
    assert_eq!(
        block_on(s.abort_game(abort.clone())).unwrap(),
        EndOutcome::Applied
    );
    let g = game(&s, 1);
    assert_eq!(g.status, GameStatus::Aborted);
    assert_eq!(g.result, None);
    assert_eq!(g.termination.as_deref(), Some("aborted"));
    assert_eq!((g.ended_at, g.journal_seq), (Some(WallMs(77)), 9));
    assert_eq!(g.content_hash.as_deref(), Some(HASH));
    assert_eq!(count(&d, "SELECT count(*) FROM unfinished_slot"), 0);
    assert_eq!(count(&d, "SELECT count(*) FROM game_lease"), 0);
    assert_eq!(
        count(&d, "SELECT count(*) FROM analysis_job WHERE state='queued'"),
        1
    );
    assert_eq!(elos(&s, 10), (800, 800, 0, 0));
    assert_eq!(
        count(
            &d,
            "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL"
        ),
        0
    );
    // Repeat, and finalize after abort, change nothing.
    let snapshot = dump(&raw(d.path()));
    assert_eq!(
        block_on(s.abort_game(abort)).unwrap(),
        EndOutcome::AlreadyFinal
    );
    assert_eq!(
        block_on(s.finalize_game(req(1, GameResult::Draw, 1))).unwrap(),
        FinalizeOutcome::AlreadyFinal
    );
    assert_eq!(snapshot, dump(&raw(d.path())));
    let mut empty = AbortRequest {
        game: gid(1),
        ended_at: WallMs(1),
        content_hash: String::new(),
        final_journal_seq: 0,
    };
    start(
        &s,
        2,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    );
    empty.game = gid(2);
    assert!(matches!(
        block_on(s.abort_game(empty)),
        Err(StoreError::InvariantViolated)
    ));
}

#[test]
fn ut_st_086_void_half_zero_moves_hash_is_stored_and_lease_beat_reports_the_row() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.lease_upsert(gid(1), s.owner())).unwrap();
    assert_eq!(count(&d, "SELECT count(*) FROM move"), 0, "zero moves");
    assert_eq!(
        block_on(s.void_unreadable_game(gid(1), HASH.into())).unwrap(),
        EndOutcome::Applied
    );
    let g = game(&s, 1);
    assert_eq!(g.status, GameStatus::Voided);
    assert_eq!(g.termination.as_deref(), Some("voided: unreadable"));
    assert_eq!((g.voided_by, g.result), (None, None));
    assert_eq!(g.content_hash.as_deref(), Some(HASH));
    let ended = g.ended_at.expect("the void time");
    assert!(ended.0 >= 1_000, "the store clock: {ended:?}");
    assert_eq!(count(&d, "SELECT count(*) FROM unfinished_slot"), 0);
    assert_eq!(
        count(&d, "SELECT count(*) FROM analysis_job WHERE state='queued'"),
        1
    );
    assert_eq!(elos(&s, 1), (1200, 1200, 0, 0));
    // The lease row is gone: RowGone carries what the session needs for its `Voided` record.
    assert_eq!(
        block_on(s.lease_beat(gid(1), s.owner())).unwrap(),
        LeaseBeat::RowGone {
            status: GameStatus::Voided,
            voided_by: None,
            content_hash: Some(HASH.into()),
            ended_at: Some(ended),
        }
    );
    // A second void is a no-op.
    let snapshot = dump(&raw(d.path()));
    assert_eq!(
        block_on(s.void_unreadable_game(gid(1), "sha256:v1:zz".into())).unwrap(),
        EndOutcome::AlreadyFinal
    );
    assert_eq!(snapshot, dump(&raw(d.path())));
    // An empty hash is refused (SF-17) and an unknown game is NotFound.
    start(&s, 2, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    assert!(matches!(
        block_on(s.void_unreadable_game(gid(2), String::new())),
        Err(StoreError::InvariantViolated)
    ));
    assert!(matches!(
        block_on(s.void_unreadable_game(gid(9), HASH.into())),
        Err(StoreError::NotFound { .. })
    ));
}

#[test]
fn ut_st_035_the_hash_is_null_until_terminal_and_set_by_every_ending() {
    let (d, s) = world();
    for n in 1..=4 {
        let (w, b, m) = match n {
            3 => (pid(10), pid(11), Matchup::ComputerComputer),
            4 => (pid(3), pid(10), Matchup::HumanComputer),
            _ => (pid(n), pid(10), Matchup::HumanComputer),
        };
        start(&s, n, w, b, m, Mode::Strict);
    }
    let hash = |n: u128| game(&s, n).content_hash;
    assert!((1..=4).all(|n| hash(n).is_none()));
    // The CHECK refuses a hash on an in-progress row and a terminal row without one.
    let conn = raw(d.path());
    assert!(
        conn.execute(
            "UPDATE game SET content_hash = 'x' WHERE status = 'in_progress'",
            []
        )
        .is_err()
    );
    assert!(
        conn.execute(
            "UPDATE game SET status = 'aborted', content_hash = NULL WHERE uuid = ?1",
            [gid(1).as_uuid().to_string()]
        )
        .is_err()
    );
    block_on(s.finalize_game(req(1, GameResult::Draw, 1))).unwrap();
    block_on(s.void_unreadable_game(gid(2), "sha256:v1:v".into())).unwrap();
    block_on(s.abort_game(AbortRequest {
        game: gid(3),
        ended_at: WallMs(3),
        content_hash: "sha256:v1:a".into(),
        final_journal_seq: 1,
    }))
    .unwrap();
    block_on(s.reset_player(
        pid(3),
        vec![VoidSpec {
            game: gid(4),
            content_hash: "sha256:v1:r".into(),
        }],
        "{}".into(),
    ))
    .unwrap();
    assert_eq!(hash(1).as_deref(), Some(HASH));
    assert_eq!(hash(2).as_deref(), Some("sha256:v1:v"));
    assert_eq!(hash(3).as_deref(), Some("sha256:v1:a"));
    assert_eq!(hash(4).as_deref(), Some("sha256:v1:r"));
}

fn spec(n: u128, hash: &str) -> VoidSpec {
    VoidSpec {
        game: gid(n),
        content_hash: hash.into(),
    }
}

/// One reset scenario: Ann holds a strict h-h game with Bob (`lease`: none = suspended, this
/// process, another process) and a training game against a computer.
fn reset_scenario(lease: Option<&str>) {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(2), Matchup::HumanHuman, Mode::Strict);
    start(
        &s,
        2,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    );
    match lease {
        None => {}
        Some("self") => {
            block_on(s.lease_upsert(gid(1), s.owner())).unwrap();
            block_on(s.lease_upsert(gid(2), s.owner())).unwrap();
        }
        Some(other) => {
            block_on(s.lease_upsert(gid(1), other.into())).unwrap();
            block_on(s.lease_upsert(gid(2), other.into())).unwrap();
        }
    }
    // A finished rated game of others, so ratings and counters exist to be kept.
    start(&s, 3, pid(3), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(3, GameResult::WhiteWins, 5))).unwrap();
    let ratings_before = (elos(&s, 1), elos(&s, 2), elos(&s, 10));
    let counters_before = count(
        &d,
        "SELECT sum(strict_rated_games + training_rated_games) FROM player",
    );
    let hist_before = count(
        &d,
        "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL",
    );
    block_on(s.reset_player(
        pid(1),
        vec![spec(1, "sha256:v1:one"), spec(2, "sha256:v1:two")],
        "{\"v\":0}".into(),
    ))
    .unwrap();
    let reset_at = block_on(s.get_player(pid(1))).unwrap().reset_at.unwrap();
    for (n, hash) in [(1, "sha256:v1:one"), (2, "sha256:v1:two")] {
        let g = game(&s, n);
        assert_eq!(g.status, GameStatus::Voided, "game {n}");
        assert_eq!(g.termination.as_deref(), Some("voided by reset"));
        assert_eq!((g.result, g.voided_by), (None, Some(pid(1))));
        assert_eq!(g.ended_at, Some(reset_at), "the commit time (D-69)");
        assert_eq!(g.content_hash.as_deref(), Some(hash));
    }
    assert_eq!(
        count(
            &d,
            "SELECT count(*) FROM unfinished_slot WHERE game_id IN (SELECT id FROM game WHERE status='voided')"
        ),
        0,
        "Bob's slot too"
    );
    assert_eq!(count(&d, "SELECT count(*) FROM game_lease"), 0);
    assert_eq!(
        count(&d, "SELECT count(*) FROM analysis_job WHERE state='queued'"),
        3,
        "two voided games and the finished game 3"
    );
    assert_eq!(ratings_before, (elos(&s, 1), elos(&s, 2), elos(&s, 10)));
    assert_eq!(
        counters_before,
        count(
            &d,
            "SELECT sum(strict_rated_games + training_rated_games) FROM player"
        )
    );
    assert_eq!(
        hist_before,
        count(
            &d,
            "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL"
        ),
        "no rating row for any side"
    );
    // One reset point per track: no game, before = after = current, k = 0 (F-A04).
    for track in [Track::Strict, Track::Training] {
        let h = points(&s, 1, track);
        assert_eq!(h.len(), 2, "initial point and reset point");
        let last = h.last().unwrap();
        assert_eq!((last.game_id, last.at, last.k), (None, reset_at, 0));
        assert_eq!(last.rating_before, last.rating_after);
    }
    assert_eq!(
        block_on(s.get_preferences(pid(1))).unwrap().as_deref(),
        Some("{\"v\":0}")
    );
    assert!(
        block_on(s.check_rating_invariant(vec![pid(1)]))
            .unwrap()
            .is_empty()
    );
    // Bob (the opponent) still sees the voided game, with no change to his rating.
    assert!(
        block_on(s.list_games(chess_store::GameFilter {
            player: Some(pid(2)),
            ..Default::default()
        }))
        .unwrap()
        .iter()
        .any(|g| g.id == gid(1) && g.status == GameStatus::Voided)
    );
    // Bob's strict slot is free again.
    start(&s, 6, pid(2), pid(10), Matchup::HumanComputer, Mode::Strict);
}

#[test]
fn ct_st_028_reset_voids_a_suspended_game() {
    reset_scenario(None);
}

#[test]
fn ct_st_028_reset_voids_a_game_leased_by_this_process() {
    reset_scenario(Some("self"));
}

#[test]
fn ct_st_028_reset_voids_a_game_leased_by_another_process() {
    reset_scenario(Some("server:99999"));
}

#[test]
fn ct_st_028_an_unlisted_unfinished_game_rolls_the_whole_reset_back() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(2), Matchup::HumanHuman, Mode::Strict);
    start(
        &s,
        2,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    );
    let before = dump(&raw(d.path()));
    let err = block_on(s.reset_player(pid(1), vec![spec(1, HASH)], "{}".into())).unwrap_err();
    assert!(
        matches!(&err, StoreError::UnlistedUnfinishedGames { ids } if *ids == vec![gid(2)]),
        "{err:?}"
    );
    assert_eq!(
        before,
        dump(&raw(d.path())),
        "nothing changed, not even game 1"
    );
    assert_eq!(game(&s, 1).status, GameStatus::InProgress);
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().reset_at, None);
    // Listing nothing while a game is unfinished is the same error, with both ids in start order.
    let err = block_on(s.reset_player(pid(1), vec![], "{}".into())).unwrap_err();
    assert!(
        matches!(&err, StoreError::UnlistedUnfinishedGames { ids } if *ids == vec![gid(1), gid(2)]),
        "{err:?}"
    );
    // Archive reports the same games (F-A05 option A).
    let err = block_on(s.archive_player(pid(1))).unwrap_err();
    assert!(
        matches!(&err, StoreError::HasUnfinishedGames { ids } if ids.len() == 2),
        "{err:?}"
    );
}

#[test]
fn reset_with_no_unfinished_game_only_resets_and_a_second_reset_adds_a_second_point() {
    let (_d, s) = world();
    block_on(s.reset_player(pid(1), vec![], "{\"a\":1}".into())).unwrap();
    block_on(s.reset_player(pid(1), vec![], "{\"a\":2}".into())).unwrap();
    assert_eq!(points(&s, 1, Track::Strict).len(), 3);
    assert_eq!(
        block_on(s.get_preferences(pid(1))).unwrap().as_deref(),
        Some("{\"a\":2}")
    );
    assert!(
        block_on(s.check_rating_invariant(vec![pid(1)]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reset_edge_cases_computer_foreign_game_terminal_listed_game_and_duplicates() {
    let (d, s) = world();
    // A computer cannot be reset (the schema keeps reset_at human-only).
    assert!(matches!(
        block_on(s.reset_player(pid(10), vec![], "{}".into())),
        Err(StoreError::InvariantViolated)
    ));
    // A listed game of another Player that is in progress is a caller bug.
    start(&s, 1, pid(2), pid(10), Matchup::HumanComputer, Mode::Strict);
    let before = dump(&raw(d.path()));
    assert!(matches!(
        block_on(s.reset_player(pid(1), vec![spec(1, HASH)], "{}".into())),
        Err(StoreError::InvariantViolated)
    ));
    assert_eq!(before, dump(&raw(d.path())));
    // An unknown listed game is NotFound and rolls back.
    assert!(matches!(
        block_on(s.reset_player(pid(1), vec![spec(9, HASH)], "{}".into())),
        Err(StoreError::NotFound { .. })
    ));
    assert_eq!(before, dump(&raw(d.path())));
    // A listed game that finished in the meantime is skipped; a duplicate entry is one void.
    start(&s, 2, pid(1), pid(11), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(2, GameResult::Draw, 5))).unwrap();
    start(
        &s,
        3,
        pid(1),
        pid(11),
        Matchup::HumanComputer,
        Mode::Training,
    );
    block_on(s.reset_player(
        pid(1),
        vec![
            spec(2, "sha256:v1:other"),
            spec(3, "sha256:v1:t"),
            spec(3, "sha256:v1:t2"),
        ],
        "{}".into(),
    ))
    .unwrap();
    let done = game(&s, 2);
    assert_eq!(done.status, GameStatus::Completed);
    assert_eq!(
        done.content_hash.as_deref(),
        Some(HASH),
        "the finished game is untouched"
    );
    assert_eq!(
        game(&s, 3).content_hash.as_deref(),
        Some("sha256:v1:t"),
        "the first entry wins"
    );
    // Voiding an empty hash is refused.
    start(
        &s,
        4,
        pid(3),
        pid(11),
        Matchup::HumanComputer,
        Mode::Training,
    );
    assert!(matches!(
        block_on(s.reset_player(pid(3), vec![spec(4, "")], "{}".into())),
        Err(StoreError::InvariantViolated)
    ));
    assert_eq!(game(&s, 4).status, GameStatus::InProgress);
}

// ---------------------------------------------------------------- invariant

fn set(d: &TempDir, sql: &str) {
    raw(d.path()).execute_batch(sql).unwrap();
}

#[test]
fn invariant_names_each_kind_of_damage_and_treats_null_game_rows_as_points() {
    let (d, s) = world();
    let uuid = pid(1).as_uuid().to_string();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(1, GameResult::WhiteWins, 5))).unwrap();
    // A reset point in the middle, then another game: the sum restarts at the point.
    block_on(s.reset_player(pid(1), vec![], "{}".into())).unwrap();
    start(&s, 2, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(2, GameResult::BlackWins, 6))).unwrap();
    let ok = || block_on(s.check_rating_invariant(vec![pid(1)])).unwrap();
    assert_eq!(ok(), vec![]);
    // `k` has no meaning on a point (F-A04): a nonzero k on a NULL-game row is not an error.
    set(&d, "UPDATE rating_history SET k = 99 WHERE game_id IS NULL");
    assert_eq!(ok(), vec![]);
    // The stored rating differs from last point + later deltas.
    set(
        &d,
        &format!("UPDATE player SET strict_elo = strict_elo + 1 WHERE uuid = '{uuid}'"),
    );
    let v = ok();
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].player, v[0].track), (pid(1), Track::Strict));
    assert!(
        matches!(v[0].kind, RatingViolationKind::CurrentMismatch { expected, found } if found == expected + 1)
    );
    set(
        &d,
        &format!("UPDATE player SET strict_elo = strict_elo - 1 WHERE uuid = '{uuid}'"),
    );
    assert_eq!(ok(), vec![]);
    // A point that changes the rating.
    set(
        &d,
        "UPDATE rating_history SET rating_after = rating_after + 5 WHERE game_id IS NULL AND id = (SELECT max(id) FROM rating_history WHERE game_id IS NULL AND track='strict' AND player_id = (SELECT id FROM player WHERE name='Ann'))",
    );
    assert!(
        matches!(
            ok()[0].kind,
            RatingViolationKind::PointChangesRating { index: 2 }
        ),
        "{:?}",
        ok()
    );
    set(
        &d,
        "UPDATE rating_history SET rating_after = rating_before WHERE game_id IS NULL",
    );
    assert_eq!(ok(), vec![]);
    // A gap in the chain.
    set(
        &d,
        "UPDATE rating_history SET rating_before = rating_before + 3 WHERE game_id IS NOT NULL AND track='strict' AND player_id = (SELECT id FROM player WHERE name='Ann') AND id = (SELECT min(id) FROM rating_history WHERE game_id IS NOT NULL AND track='strict' AND player_id = (SELECT id FROM player WHERE name='Ann'))",
    );
    assert!(
        matches!(ok()[0].kind, RatingViolationKind::ChainBroken { index: 1 }),
        "{:?}",
        ok()
    );
    // No initial point.
    set(
        &d,
        "DELETE FROM rating_history WHERE player_id = (SELECT id FROM player WHERE name='Ann') AND track = 'training'",
    );
    let v = ok();
    assert!(
        v.iter()
            .any(|x| x.track == Track::Training && x.kind == RatingViolationKind::NoInitialPoint),
        "{v:?}"
    );
    // An unknown Player is NotFound.
    assert!(matches!(
        block_on(s.check_rating_invariant(vec![pid(404)])),
        Err(StoreError::NotFound {
            entity: Entity::Player,
            ..
        })
    ));
}

#[test]
fn invariant_first_row_must_be_an_initial_point() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(1, GameResult::WhiteWins, 5))).unwrap();
    // The initial point is removed: the first row is now a game row.
    set(
        &d,
        "DELETE FROM rating_history WHERE game_id IS NULL AND track = 'strict' AND player_id = (SELECT id FROM player WHERE name='Ann')",
    );
    let v = block_on(s.check_rating_invariant(vec![pid(1)])).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].kind, RatingViolationKind::NoInitialPoint);
    // A first game row that happens to leave the rating unchanged (a draw of equals) is still
    // not an initial point.
    start(&s, 2, pid(2), pid(3), Matchup::HumanHuman, Mode::Strict);
    block_on(s.finalize_game(req(2, GameResult::Draw, 6))).unwrap();
    set(
        &d,
        "DELETE FROM rating_history WHERE game_id IS NULL AND track = 'strict' AND player_id = (SELECT id FROM player WHERE name='Bob')",
    );
    let v = block_on(s.check_rating_invariant(vec![pid(2)])).unwrap();
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].kind, RatingViolationKind::NoInitialPoint);
    // An initial point that moves the rating is also not initial.
    set(
        &d,
        "UPDATE rating_history SET rating_after = rating_before + 1 WHERE game_id IS NULL AND track='training' AND player_id = (SELECT id FROM player WHERE name='Ann')",
    );
    let v = block_on(s.check_rating_invariant(vec![pid(1)])).unwrap();
    assert!(
        v.iter()
            .any(|x| x.track == Track::Training && x.kind == RatingViolationKind::NoInitialPoint),
        "{v:?}"
    );
}

/// FIND-D2R-01: the invariant check reads both statements in one snapshot, so a finalize that
/// commits between them cannot make a healthy store look corrupt. One thread finalizes 300
/// games while another loops the check; every answer must be empty.
#[test]
fn find_d2r_01_the_invariant_check_never_reports_a_false_violation_beside_a_writer() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let (_d, s) = world();
    let done = AtomicBool::new(false);
    let (checks, bad) = std::thread::scope(|scope| {
        let (s, done) = (&s, &done);
        let writer = scope.spawn(move || {
            for n in 1..=300u128 {
                start(s, n, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
                let result = if n % 3 == 0 {
                    GameResult::BlackWins
                } else {
                    GameResult::WhiteWins
                };
                block_on(s.finalize_game(req(n, result, 10_000_000 + at(n)))).unwrap();
            }
            done.store(true, Ordering::SeqCst);
        });
        let checker = scope.spawn(move || {
            let (mut checks, mut bad) = (0u32, Vec::new());
            while !done.load(Ordering::SeqCst) {
                let found = block_on(s.check_rating_invariant(vec![pid(1), pid(10)])).unwrap();
                checks += 1;
                if !found.is_empty() {
                    bad.push(found);
                }
            }
            (checks, bad)
        });
        writer.join().unwrap();
        checker.join().unwrap()
    });
    assert!(checks > 0);
    assert!(
        bad.is_empty(),
        "{} false violations in {checks} checks, first: {:?}",
        bad.len(),
        bad.first()
    );
    assert_eq!(
        block_on(s.check_rating_invariant(vec![pid(1), pid(10)])).unwrap(),
        vec![]
    );
}

fn abort_request(n: u128) -> AbortRequest {
    AbortRequest {
        game: gid(n),
        ended_at: WallMs(77),
        content_hash: HASH.into(),
        final_journal_seq: 9,
    }
}

/// M14 (FIND-D2R-04, architecture 7.4): aborting a game that is already `voided` answers
/// `AlreadyFinal` and changes nothing; so does aborting a completed one.
#[test]
fn m14_abort_of_a_voided_or_completed_game_is_already_final_and_changes_nothing() {
    let (d, s) = world();
    start(
        &s,
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    );
    assert_eq!(
        block_on(s.void_unreadable_game(gid(1), HASH.into())).unwrap(),
        EndOutcome::Applied
    );
    start(&s, 2, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(2, GameResult::Draw, 5))).unwrap();
    let snapshot = dump(&raw(d.path()));
    for n in [1, 2] {
        assert_eq!(
            block_on(s.abort_game(abort_request(n))).unwrap(),
            EndOutcome::AlreadyFinal,
            "game {n}"
        );
    }
    assert_eq!(snapshot, dump(&raw(d.path())));
    assert_eq!(game(&s, 1).status, GameStatus::Voided);
}

/// M25 (FIND-D2R-04): `void_unreadable_game` of a completed, an aborted and a voided game
/// answers `AlreadyFinal` and changes nothing; an unknown game is `NotFound`.
#[test]
fn m25_void_unreadable_of_a_finished_game_is_already_final_and_changes_nothing() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.finalize_game(req(1, GameResult::WhiteWins, 5))).unwrap();
    start(
        &s,
        2,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    );
    block_on(s.abort_game(abort_request(2))).unwrap();
    start(&s, 3, pid(2), pid(10), Matchup::HumanComputer, Mode::Strict);
    block_on(s.void_unreadable_game(gid(3), HASH.into())).unwrap();
    let snapshot = dump(&raw(d.path()));
    for n in [1, 2, 3] {
        assert_eq!(
            block_on(s.void_unreadable_game(gid(n), "sha256:v1:bbbb".into())).unwrap(),
            EndOutcome::AlreadyFinal,
            "game {n}"
        );
    }
    assert_eq!(snapshot, dump(&raw(d.path())));
    assert_eq!(game(&s, 1).status, GameStatus::Completed);
    assert_eq!(game(&s, 2).status, GameStatus::Aborted);
    assert!(matches!(
        block_on(s.void_unreadable_game(gid(99), HASH.into())),
        Err(StoreError::NotFound { .. })
    ));
}

// ---------------------------------------------------------------- rulings of 2026-11-23

fn analysis_flag(s: &Store, n: u128) -> bool {
    game(s, n).analysis_used
}

fn finalize_with_flag(s: &Store, n: u128, flag: bool) {
    let mut r = req(n, GameResult::Draw, 9_000);
    r.analysis_used = flag;
    assert!(matches!(
        block_on(s.finalize_game(r)).unwrap(),
        FinalizeOutcome::Finalized { .. }
    ));
}

/// FIND-D2R-02 (ruling 2026-11-23): `analysis_used` is a latch. The gateway set it when it granted
/// analysis; a `GameEnd` that says `false` (rebuilt after a crash from a journal that carries no
/// flag) must not clear it.
#[test]
fn d2r_02_set_analysis_used_then_finalize_with_false_keeps_true() {
    let (_d, s) = world();
    start(
        &s,
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    );
    block_on(s.set_analysis_used(gid(1))).unwrap();
    assert!(analysis_flag(&s, 1));
    finalize_with_flag(&s, 1, false);
    assert!(analysis_flag(&s, 1), "the latch is not cleared");
}

/// The other three cells of the truth table of `stored OR request`.
#[test]
fn d2r_02_finalize_stores_the_or_of_the_stored_flag_and_the_request_flag() {
    let (_d, s) = world();
    for (n, set, request, expected) in [
        (1u128, false, false, false),
        (2, false, true, true),
        (3, true, true, true),
        (4, true, false, true),
    ] {
        start(
            &s,
            n,
            pid(1),
            pid(10),
            Matchup::HumanComputer,
            Mode::Training,
        );
        if set {
            block_on(s.set_analysis_used(gid(n))).unwrap();
        }
        finalize_with_flag(&s, n, request);
        assert_eq!(
            analysis_flag(&s, n),
            expected,
            "stored {set} or request {request}"
        );
    }
}

/// Crash case: the grant was committed, the process died, the next process finalizes from a
/// replayed `GameEnd` with `false`. The flag survives the restart.
#[test]
fn d2r_02_the_latch_survives_a_restart_before_the_finalize() {
    let (d, s) = world();
    start(
        &s,
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    );
    block_on(s.set_analysis_used(gid(1))).unwrap();
    drop(s);
    let s = open_in(d.path()).store;
    assert!(analysis_flag(&s, 1), "stored before the restart");
    finalize_with_flag(&s, 1, false);
    assert!(analysis_flag(&s, 1));
}

/// FIND-D2R-03 (ruling 2026-11-23): the store checks the shape of terminal data only.
#[test]
fn d2r_03_finalize_refuses_a_non_positive_ended_at_and_an_empty_termination_and_writes_nothing() {
    let (d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    let before = dump(&raw(d.path()));
    for ended_at in [0, -5, i64::MIN] {
        assert!(
            matches!(
                block_on(s.finalize_game(req(1, GameResult::WhiteWins, ended_at))),
                Err(StoreError::InvariantViolated)
            ),
            "ended_at {ended_at}"
        );
    }
    let mut empty = req(1, GameResult::WhiteWins, 9_000);
    empty.termination = String::new();
    assert!(matches!(
        block_on(s.finalize_game(empty)),
        Err(StoreError::InvariantViolated)
    ));
    assert_eq!(before, dump(&raw(d.path())), "nothing written");
    assert_eq!(game(&s, 1).status, GameStatus::InProgress);
    assert_eq!(elos(&s, 1), (1200, 1200, 0, 0));
    // The boundary: 1 is a valid time.
    assert!(matches!(
        block_on(s.finalize_game(req(1, GameResult::WhiteWins, 1))).unwrap(),
        FinalizeOutcome::Finalized { .. }
    ));
}

#[test]
fn d2r_03_abort_refuses_a_non_positive_ended_at_and_writes_nothing() {
    let (d, s) = world();
    start(
        &s,
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    );
    let before = dump(&raw(d.path()));
    for ended_at in [0, -1] {
        assert!(
            matches!(
                block_on(s.abort_game(AbortRequest {
                    game: gid(1),
                    ended_at: WallMs(ended_at),
                    content_hash: HASH.into(),
                    final_journal_seq: 1,
                })),
                Err(StoreError::InvariantViolated)
            ),
            "ended_at {ended_at}"
        );
    }
    assert_eq!(before, dump(&raw(d.path())));
    assert_eq!(game(&s, 1).status, GameStatus::InProgress);
}

/// What the store must NOT check (ruling 2026-11-23; the store applies what a journal says):
/// an end time before the start, a journal sequence that is lower than the stored one, and an
/// abort of a game that is not computer against computer are all stored as given.
#[test]
fn d2r_03_the_store_does_not_check_time_order_journal_sequence_or_who_aborts() {
    let (_d, s) = world();
    start(&s, 1, pid(1), pid(10), Matchup::HumanComputer, Mode::Strict);
    let started = game(&s, 1).started_at.expect("a start time");
    // Ends at 5 ms, long before it started; final_journal_seq 0 is below nothing stored but is
    // not checked against anything either.
    let mut r = req(1, GameResult::Draw, 5);
    r.final_journal_seq = 0;
    assert!(started.0 > 5);
    assert!(matches!(
        block_on(s.finalize_game(r)).unwrap(),
        FinalizeOutcome::Finalized { .. }
    ));
    assert_eq!(game(&s, 1).ended_at, Some(WallMs(5)));
    // A human-against-human game is aborted without complaint.
    start(&s, 2, pid(1), pid(2), Matchup::HumanHuman, Mode::Strict);
    assert_eq!(
        block_on(s.abort_game(AbortRequest {
            game: gid(2),
            ended_at: WallMs(3),
            content_hash: HASH.into(),
            final_journal_seq: 0,
        }))
        .unwrap(),
        EndOutcome::Applied
    );
    assert_eq!(game(&s, 2).status, GameStatus::Aborted);
}

/// D3 ruling 2026-11-24 (FIND-D3R-07): the live void paths use the same `ended_at > 0` check as
/// the journal. A store clock at or below zero refuses `void_unreadable_game` and the voids of
/// `reset_player`; nothing is written and the games stay in progress.
#[test]
fn d3_live_void_paths_refuse_a_clock_at_or_below_zero() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};
    for bad in [0, -5] {
        let dir = TempDir::new().unwrap();
        let now = Arc::new(AtomicI64::new(1_000));
        let clock = Arc::clone(&now);
        let options = chess_store::StoreOptions::new(chess_store::DataDir::new(dir.path()))
            .with_clock(Arc::new(move || clock.load(Ordering::SeqCst)));
        let s = block_on(Store::open(options)).unwrap().store;
        for (n, name) in [(1, "Ann"), (2, "Bob")] {
            block_on(s.register_player(human(n, name))).unwrap();
        }
        start(&s, 1, pid(1), pid(2), Matchup::HumanHuman, Mode::Strict);
        now.store(bad, Ordering::SeqCst);
        let before = dump(&raw(dir.path()));
        let unreadable = block_on(s.void_unreadable_game(gid(1), HASH.into()));
        assert!(
            matches!(unreadable, Err(StoreError::InvariantViolated)),
            "{bad}: {unreadable:?}"
        );
        let reset = block_on(s.reset_player(pid(1), vec![spec(1, HASH)], "{}".into()));
        assert!(
            matches!(reset, Err(StoreError::InvariantViolated)),
            "{bad}: {reset:?}"
        );
        assert_eq!(before, dump(&raw(dir.path())), "{bad}: nothing written");
        assert_eq!(game(&s, 1).status, GameStatus::InProgress);
        now.store(2_000, Ordering::SeqCst);
        assert_eq!(
            block_on(s.void_unreadable_game(gid(1), HASH.into())).unwrap(),
            EndOutcome::Applied
        );
    }
}

/// FIND-F2-05 (ruling 2026-11-24): `set_analysis_used` on a game that is already final is
/// refused with a typed error and changes nothing, unless the flag is already set (then it is
/// the same idempotent `Ok`). An in-progress game accepts it.
#[test]
fn f2_05_set_analysis_used_refuses_a_final_game_unless_the_flag_is_already_set() {
    let (d, s) = world();
    for n in 1..=4u128 {
        start(
            &s,
            n,
            pid(1),
            pid(10),
            Matchup::HumanComputer,
            Mode::Training,
        );
        // Training allows one unfinished game per Player and mode: finish each before the next.
        match n {
            1 => {
                let mut r = req(1, GameResult::WhiteWins, 5);
                r.analysis_used = false;
                block_on(s.finalize_game(r)).unwrap();
            }
            2 => {
                let mut r = req(2, GameResult::Draw, 6);
                r.analysis_used = true;
                block_on(s.finalize_game(r)).unwrap();
            }
            3 => {
                block_on(s.abort_game(AbortRequest {
                    game: gid(3),
                    ended_at: WallMs(7),
                    content_hash: HASH.into(),
                    final_journal_seq: 1,
                }))
                .unwrap();
            }
            _ => {
                block_on(s.void_unreadable_game(gid(4), HASH.into())).unwrap();
            }
        }
    }
    let flag = |n: u128| -> i64 {
        count(
            &d,
            &format!("SELECT analysis_used FROM game WHERE id = {n}"),
        )
    };
    let before = dump(&raw(d.path()));
    for n in [1u128, 3, 4] {
        let out = block_on(s.set_analysis_used(gid(n)));
        assert!(
            matches!(out, Err(StoreError::InvariantViolated)),
            "{n}: {out:?}"
        );
        assert_eq!(flag(n), 0, "{n}");
    }
    block_on(s.set_analysis_used(gid(2))).unwrap();
    assert_eq!(flag(2), 1, "already set: idempotent Ok");
    assert_eq!(before, dump(&raw(d.path())), "nothing changed");
    // An in-progress game accepts it, twice.
    start(
        &s,
        5,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    );
    block_on(s.set_analysis_used(gid(5))).unwrap();
    block_on(s.set_analysis_used(gid(5))).unwrap();
    assert_eq!(flag(5), 1);
}
