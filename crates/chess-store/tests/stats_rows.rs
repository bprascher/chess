//! Part D5, statistics rows through the public `Store` API (UT-ST-060 stats half; D-44,
//! TR-7): rows filtered only by structural keys, voided, self-play and custom-start games
//! included, no SQL view.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d5_support;

use chess_domain::{Category, GameStatus, Mode, PlayerKind, ResultForViewed, WallMs};
use chess_store::{Entity, StatsSide, StoreError};
use common::{block_on, gid, pid, raw};
use d5_support::{G, seed, world};
use tempfile::TempDir;

/// Ann = 1, Bob = 2, Bot A = 10. Eight games that name Ann, one that does not.
fn eight_games(dir: &std::path::Path) {
    seed(dir, &G::done(1, 1, 2, "1-0")); // Ann white wins
    let mut voided = G::done(2, 1, 2, "1-0");
    voided.status = "voided";
    voided.result = None;
    voided.active_ms = None;
    seed(dir, &voided);
    seed(dir, &G::done(3, 1, 1, "1/2-1/2")); // self-play
    let mut custom = G::done(4, 1, 2, "0-1");
    custom.custom = true;
    seed(dir, &custom);
    let mut aborted = G::done(5, 1, 2, "1-0");
    aborted.status = "aborted";
    aborted.result = None;
    aborted.active_ms = None;
    seed(dir, &aborted);
    let mut open = G::done(6, 1, 2, "1-0");
    open.status = "in_progress";
    open.result = None;
    open.active_ms = None;
    open.plies = 0;
    seed(dir, &open);
    seed(dir, &G::done(7, 2, 1, "0-1")); // Bob white, Ann black wins
    let mut hc = G::done(8, 1, 10, "1/2-1/2");
    hc.matchup = "hc";
    seed(dir, &hc);
    seed(dir, &G::done(9, 2, 3, "1-0")); // Bob v Cy: not Ann's
}

#[test]
fn ut_st_060_stats_rows_apply_no_counting_predicate() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    eight_games(dir.path());
    let rows = block_on(s.stats_rows(pid(1), None, None, None)).unwrap();
    assert_eq!(rows.len(), 8, "every game that names Ann, no predicate");
    let status: Vec<GameStatus> = rows.iter().map(|r| r.status).collect();
    assert_eq!(
        status,
        [
            GameStatus::Completed,
            GameStatus::Voided,
            GameStatus::Completed,
            GameStatus::Completed,
            GameStatus::Aborted,
            GameStatus::InProgress,
            GameStatus::Completed,
            GameStatus::Completed
        ],
        "game.id order; voided, aborted and unfinished games are rows"
    );
    assert!(rows[2].self_play && !rows[0].self_play, "self-play row");
    assert!(
        rows[3].custom_start && !rows[0].custom_start,
        "custom start row"
    );
    let results: Vec<Option<ResultForViewed>> = rows.iter().map(|r| r.result_for_viewed).collect();
    assert_eq!(
        results,
        [
            Some(ResultForViewed::Win),
            None,
            Some(ResultForViewed::Draw),
            Some(ResultForViewed::Loss),
            None,
            None,
            Some(ResultForViewed::Win), // Ann was Black and Black won
            Some(ResultForViewed::Draw)
        ]
    );
    assert_eq!(rows[6].color, StatsSide::Black, "Ann was Black in game 7");
    assert_eq!(rows[0].color, StatsSide::White);
    assert_eq!(rows[0].mode, Mode::Strict);
    assert_eq!((rows[0].plies, rows[0].active_ms), (10, 60_000));
    assert_eq!((rows[5].plies, rows[5].active_ms), (0, 0), "NULL time is 0");
    assert_eq!(rows[0].opponent.player, pid(2));
    assert_eq!(rows[0].opponent.kind, PlayerKind::Human);
    assert_eq!(rows[0].opponent.category_now, None, "a human has no label");
    assert_eq!(
        rows[2].opponent.player,
        pid(1),
        "self-play: the opponent is the viewed Player"
    );
    assert_eq!(rows[7].opponent.kind, PlayerKind::Computer);
    assert_eq!(rows[7].opponent.category_now, Some(Category::Novice));
}

#[test]
fn a_computers_label_is_its_current_strict_band() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let mut hc = G::done(1, 1, 10, "1-0");
    hc.matchup = "hc";
    seed(dir.path(), &hc);
    let label = |s: &chess_store::Store| {
        block_on(s.stats_rows(pid(1), None, None, None)).unwrap()[0]
            .opponent
            .category_now
    };
    assert_eq!(label(&s), Some(Category::Novice), "800 is Novice");
    raw(dir.path())
        .execute(
            "UPDATE player SET strict_elo = 2500 WHERE uuid = ?1",
            [common::pid(10).as_uuid().hyphenated().to_string()],
        )
        .unwrap();
    assert_eq!(label(&s), Some(Category::GrandMaster), "read at query time");
}

#[test]
fn the_side_key_selects_the_seat_and_sets_the_colour() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    eight_games(dir.path());
    let white = block_on(s.stats_rows(pid(1), None, Some(StatsSide::White), None)).unwrap();
    assert_eq!(white.len(), 7, "games 1 to 6 and 8");
    assert!(white.iter().all(|r| r.color == StatsSide::White));
    let black = block_on(s.stats_rows(pid(1), None, Some(StatsSide::Black), None)).unwrap();
    // Game 3 (self-play: Ann holds Black too) and game 7.
    assert_eq!(black.len(), 2);
    assert!(black.iter().all(|r| r.color == StatsSide::Black));
    assert_eq!(black[1].result_for_viewed, Some(ResultForViewed::Win));
    assert_eq!(
        black[0].result_for_viewed,
        Some(ResultForViewed::Draw),
        "the self-play draw seen from Black"
    );
}

#[test]
fn the_reset_window_is_started_at_greater_or_equal() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let mut a = G::done(1, 1, 2, "1-0");
    a.started_at = 999;
    let mut b = G::done(2, 1, 2, "1-0");
    b.started_at = 1_000;
    let mut c = G::done(3, 1, 2, "1-0");
    c.started_at = 1_001;
    for g in [&a, &b, &c] {
        seed(dir.path(), g);
    }
    let rows = block_on(s.stats_rows(pid(1), Some(WallMs(1_000)), None, None)).unwrap();
    let starts: Vec<i64> = rows.iter().map(|r| r.started_at.0).collect();
    assert_eq!(starts, [1_000, 1_001], "the boundary is inside, 999 is out");
    let all = block_on(s.stats_rows(pid(1), None, None, None)).unwrap();
    assert_eq!(all.len(), 3, "no window: no filter");
}

#[test]
fn limit_keeps_the_first_n_in_start_order() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    for n in 1..=5 {
        seed(dir.path(), &G::done(n, 1, 2, "1-0"));
    }
    let rows = block_on(s.stats_rows(pid(1), None, None, Some(3))).unwrap();
    let starts: Vec<i64> = rows.iter().map(|r| r.started_at.0).collect();
    assert_eq!(starts, [10_001, 10_002, 10_003]);
    assert_eq!(
        block_on(s.stats_rows(pid(1), None, None, Some(0)))
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn head_to_head_takes_both_seats_and_only_that_opponent() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    eight_games(dir.path());
    let rows = block_on(s.head_to_head_rows(pid(1), pid(2), None, None)).unwrap();
    // Games 1, 2, 4, 5, 6 (Ann white) and 7 (Ann black); not 3 (self-play), 8 (computer), 9.
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| r.opponent.player == pid(2)));
    assert_eq!(rows[5].color, StatsSide::Black);
    // The same games seen from Bob.
    let bob = block_on(s.head_to_head_rows(pid(2), pid(1), None, None)).unwrap();
    assert_eq!(bob.len(), 6);
    assert_eq!(bob[5].result_for_viewed, Some(ResultForViewed::Loss));
    // The window and the limit apply.
    let limited =
        block_on(s.head_to_head_rows(pid(1), pid(2), Some(WallMs(10_004)), Some(2))).unwrap();
    assert_eq!(limited.len(), 2);
    assert_eq!(limited[0].started_at, WallMs(10_004));
    // A Player against itself: the self-play game only.
    let own = block_on(s.head_to_head_rows(pid(1), pid(1), None, None)).unwrap();
    assert_eq!(own.len(), 1);
    assert!(own[0].self_play);
}

#[test]
fn unknown_players_are_not_found() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let missing = pid(99);
    for r in [
        block_on(s.stats_rows(missing, None, None, None)).map(drop),
        block_on(s.head_to_head_rows(missing, pid(1), None, None)).map(drop),
        block_on(s.head_to_head_rows(pid(1), missing, None, None)).map(drop),
        block_on(s.games_for_history(missing, None, None)).map(drop),
    ] {
        assert!(
            matches!(r, Err(StoreError::NotFound { entity: Entity::Player, id }) if id == missing.as_uuid()),
            "{r:?}"
        );
    }
}

#[test]
fn rating_points_without_a_game_are_not_games() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let conn = raw(dir.path());
    let key = d5_support::player_key(&conn, 3);
    // Registration wrote an initial point per track (NULL game_id); add a reset point too.
    conn.execute(
        "INSERT INTO rating_history (player_id, track, game_id, rating_before, rating_after, k, at) \
         VALUES (?1, 'strict', NULL, 1200, 1200, 0, 5)",
        [key],
    )
    .unwrap();
    let points: i64 = conn
        .query_row(
            "SELECT count(*) FROM rating_history WHERE player_id = ?1 AND game_id IS NULL",
            [key],
            |r| r.get(0),
        )
        .unwrap();
    assert!(points >= 3, "initial points exist: {points}");
    assert!(
        block_on(s.stats_rows(pid(3), None, None, None))
            .unwrap()
            .is_empty()
    );
    assert!(
        block_on(s.games_for_history(pid(3), None, None))
            .unwrap()
            .is_empty()
    );
    assert!(
        block_on(s.head_to_head_rows(pid(3), pid(1), None, None))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn imported_games_name_no_player_and_are_not_rows() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let mut imported = G::done(1, 1, 2, "1-0");
    imported.white = None;
    imported.black = None;
    imported.status = "imported";
    seed(dir.path(), &imported);
    seed(dir.path(), &G::done(2, 1, 2, "1-0"));
    let rows = block_on(s.stats_rows(pid(1), None, None, None)).unwrap();
    assert_eq!(rows.len(), 1);
    let history = block_on(s.games_for_history(pid(1), None, None)).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, gid(2));
}

#[test]
fn games_for_history_returns_whole_rows_of_every_kind_with_window_and_limit() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    eight_games(dir.path());
    let all = block_on(s.games_for_history(pid(1), None, None)).unwrap();
    assert_eq!(
        all.len(),
        8,
        "voided, aborted, unfinished, self-play and custom start included"
    );
    assert!(all.iter().any(|g| g.status == GameStatus::Voided));
    assert!(all.iter().any(|g| g.custom_start && g.start_fen.is_some()));
    assert!(all.iter().any(|g| g.self_play));
    let windowed = block_on(s.games_for_history(pid(1), Some(WallMs(10_007)), Some(1))).unwrap();
    assert_eq!(windowed.len(), 1);
    assert_eq!(windowed[0].id, gid(7));
}

#[test]
fn no_sql_view_exists() {
    let dir = TempDir::new().unwrap();
    let _s = world(dir.path());
    let views: i64 = raw(dir.path())
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'view'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(views, 0, "TR-7, D-44: no player_stats or head_to_head view");
}

/// Games 1 and 4 were inserted first and started last; games 2 and 3 were inserted later (as a
/// restore from a journal does) and started earlier, at the same instant. Ann plays all four.
fn restored_order(dir: &std::path::Path) {
    let mut g1 = G::done(1, 1, 2, "1-0");
    g1.started_at = 40_000;
    g1.active_ms = Some(1);
    let mut g2 = G::done(2, 1, 2, "1-0");
    g2.started_at = 10_000;
    g2.active_ms = Some(2);
    let mut g3 = G::done(3, 2, 1, "0-1");
    g3.started_at = 10_000;
    g3.active_ms = Some(3);
    let mut g4 = G::done(4, 1, 2, "1-0");
    g4.started_at = 30_000;
    g4.active_ms = Some(4);
    // Insertion order: 1, 4, 2, 3 (ids ascend in that order).
    for g in [&g1, &g4, &g2, &g3] {
        seed(dir, g);
    }
}

/// FIND-D5R-03 (ruling 2026-11-23): rows come in `(started_at, id)` order, and `limit` is the
/// oldest N by start, not the first N inserted.
#[test]
fn d5r_03_stats_rows_are_ordered_by_start_then_id_and_limit_is_the_oldest_n() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    restored_order(dir.path());
    let ms = |rows: Vec<chess_store::StatsGameRow>| -> Vec<u64> {
        rows.iter().map(|r| r.active_ms).collect()
    };
    // Game 2 and 3 tie on `started_at`: the lower `id` (game 2, inserted first of the two) goes
    // first; then game 4 (30 000), then game 1 (40 000).
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), None, None, None)).unwrap()),
        [2, 3, 4, 1]
    );
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), None, None, Some(1))).unwrap()),
        [2],
        "the oldest one"
    );
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), None, None, Some(3))).unwrap()),
        [2, 3, 4]
    );
    // Window and limit together: from 20 000 the oldest two are games 4 and 1.
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), Some(WallMs(20_000)), None, Some(1))).unwrap()),
        [4]
    );
    // Head to head uses the same key.
    assert_eq!(
        ms(block_on(s.head_to_head_rows(pid(1), pid(2), None, Some(2))).unwrap()),
        [2, 3]
    );
}

#[test]
fn d5r_03_games_for_history_and_list_games_use_the_same_key() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    restored_order(dir.path());
    let ids = |rows: Vec<chess_store::GameRecord>| -> Vec<chess_domain::GameId> {
        rows.iter().map(|g| g.id).collect()
    };
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, None)).unwrap()),
        [gid(2), gid(3), gid(4), gid(1)]
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, Some(1))).unwrap()),
        [gid(2)]
    );
    let filter = chess_store::GameFilter {
        player: None,
        status: None,
        from: None,
        limit: Some(2),
    };
    assert_eq!(
        ids(block_on(s.list_games(filter)).unwrap()),
        [gid(2), gid(3)]
    );
}

/// FIND-F2-01 (ruling 2026-11-24, RULE-7): an imported game that names both seats is not a
/// statistics row. Before the filter it made every statistics query of both Players fail with
/// `Corrupt` (an imported row has no mode). The history list still includes it.
#[test]
fn f2_01_an_imported_game_with_both_seats_is_not_a_statistics_row() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let mut imported = G::done(1, 1, 2, "1-0");
    imported.status = "imported"; // both seats stay filled
    seed(dir.path(), &imported);
    seed(dir.path(), &G::done(2, 1, 2, "1-0"));
    let rows = block_on(s.stats_rows(pid(1), None, None, None)).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, GameStatus::Completed);
    let h2h = block_on(s.head_to_head_rows(pid(1), pid(2), None, None)).unwrap();
    assert_eq!(h2h.len(), 1);
    for side in [StatsSide::White, StatsSide::Black] {
        let by_side = block_on(s.stats_rows(pid(2), None, Some(side), None)).unwrap();
        assert!(by_side.len() <= 1, "{side:?}");
    }
    let history = block_on(s.games_for_history(pid(1), None, None)).unwrap();
    assert_eq!(history.len(), 2, "history lists include everything");
}

/// Makes SQLite scan equal start times in descending `id` order, so that a query without its
/// `id` tie-break returns ties in the wrong order (FIND-F2-03: the plain scan is in `id` order
/// already and hides a missing tie-break). The expression indexes are `(started_at, 0 - id)` (for the order of the statistics queries) and
/// `(started_at IS NULL, started_at, 0 - id)` (for the order of `list_games`).
fn scan_ties_in_descending_id_order(dir: &std::path::Path) {
    raw(dir)
        .execute_batch(
            "CREATE INDEX tie_probe ON game(started_at, (0 - id)); \
             CREATE INDEX tie_probe_null_last ON game((started_at IS NULL), started_at, (0 - id))",
        )
        .unwrap();
}

/// Two games with the same `started_at`: the lower `id` comes first, and `limit 1` keeps it, for
/// every query that orders by start (mutants S02 and S05 of review fixes-2).
#[test]
fn f2_03_equal_start_times_are_ordered_by_id_and_a_limit_keeps_the_lower_id() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let mut a = G::done(1, 1, 2, "1-0");
    a.started_at = 5_000;
    a.active_ms = Some(1);
    let mut b = G::done(2, 2, 1, "0-1");
    b.started_at = 5_000;
    b.active_ms = Some(2);
    let mut c = G::done(3, 1, 2, "1-0");
    c.started_at = 5_000;
    c.active_ms = Some(3);
    for g in [&a, &b, &c] {
        seed(dir.path(), g);
    }
    scan_ties_in_descending_id_order(dir.path());
    let ms = |rows: Vec<chess_store::StatsGameRow>| -> Vec<u64> {
        rows.iter().map(|r| r.active_ms).collect()
    };
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), None, None, None)).unwrap()),
        [1, 2, 3]
    );
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), None, None, Some(1))).unwrap()),
        [1]
    );
    assert_eq!(
        ms(block_on(s.head_to_head_rows(pid(1), pid(2), None, Some(1))).unwrap()),
        [1]
    );
    let ids = |rows: Vec<chess_store::GameRecord>| -> Vec<chess_domain::GameId> {
        rows.iter().map(|g| g.id).collect()
    };
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, None)).unwrap()),
        [gid(1), gid(2), gid(3)]
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, Some(1))).unwrap()),
        [gid(1)]
    );
    let filter = |limit| chess_store::GameFilter {
        player: None,
        status: None,
        from: None,
        limit,
    };
    assert_eq!(
        ids(block_on(s.list_games(filter(Some(1)))).unwrap()),
        [gid(1)]
    );
    assert_eq!(
        ids(block_on(s.list_games(filter(None))).unwrap()),
        [gid(1), gid(2), gid(3)]
    );
}

/// FIND-F2-02 (ruling 2026-11-24): a game with no `started_at` comes last in `list_games` and
/// `games_for_history`, so "the oldest N" are played games; a window excludes it as before.
#[test]
fn f2_02_a_game_without_a_start_time_is_listed_last() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    let mut imported = G::done(1, 1, 2, "1-0");
    imported.status = "imported"; // the lowest id, both seats
    seed(dir.path(), &imported);
    raw(dir.path())
        .execute("UPDATE game SET started_at = NULL WHERE id = 1", [])
        .unwrap();
    let mut late = G::done(2, 1, 2, "1-0");
    late.started_at = 20_000;
    let mut early = G::done(3, 1, 2, "1-0");
    early.started_at = 10_000;
    seed(dir.path(), &late);
    seed(dir.path(), &early);
    let ids = |rows: Vec<chess_store::GameRecord>| -> Vec<chess_domain::GameId> {
        rows.iter().map(|g| g.id).collect()
    };
    let filter = |limit, from| chess_store::GameFilter {
        player: None,
        status: None,
        from,
        limit,
    };
    assert_eq!(
        ids(block_on(s.list_games(filter(None, None))).unwrap()),
        [gid(3), gid(2), gid(1)]
    );
    assert_eq!(
        ids(block_on(s.list_games(filter(Some(2), None))).unwrap()),
        [gid(3), gid(2)],
        "the oldest two are the played ones"
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, Some(2))).unwrap()),
        [gid(3), gid(2)]
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, None)).unwrap()),
        [gid(3), gid(2), gid(1)]
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), Some(WallMs(1)), None)).unwrap()),
        [gid(3), gid(2)],
        "a window excludes a game with no start"
    );
}
