//! UT-DOM-090 to 096 (DD-DOM-018): statistics aggregation, the counting predicate, the reset
//! window, rating series and the history check. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    Category, GameId, GameStatus, HistoryError, Mode, OpponentRef, PlayerId, PlayerKind,
    PlayerStats, RatingPoint, ResultForViewed as R, StatsRow, Tally, Track, WallMs, aggregate,
    check_history, counts_in_aggregates, rating_series,
};
use chess_rules::Side;
use common::{gid, pid};

fn opp(id: u128, kind: PlayerKind, cat: Option<Category>) -> OpponentRef {
    OpponentRef {
        player: pid(id),
        kind,
        category_now: cat,
    }
}

fn row(
    mode: Mode,
    color: Side,
    result: R,
    started: i64,
    plies: u32,
    active: u64,
    o: OpponentRef,
) -> StatsRow {
    StatsRow {
        status: GameStatus::Completed,
        self_play: false,
        custom_start: false,
        mode,
        color,
        result_for_viewed: Some(result),
        started_at: WallMs(started),
        plies,
        active_ms: active,
        opponent: o,
    }
}

fn t(w: u32, l: u32, d: u32) -> Tally {
    Tally {
        wins: w,
        losses: l,
        draws: d,
    }
}

/// Seeded rows with hand-computed totals (test-strategy "Reference data").
fn seeded() -> Vec<StatsRow> {
    let human = opp(30, PlayerKind::Human, None);
    let nov = opp(20, PlayerKind::Computer, Some(Category::Novice));
    let mas = opp(10, PlayerKind::Computer, Some(Category::Master));
    vec![
        row(Mode::Training, Side::White, R::Win, 100, 41, 60_000, nov),
        row(Mode::Training, Side::Black, R::Loss, 110, 30, 30_000, mas),
        row(Mode::Strict, Side::White, R::Draw, 120, 60, 90_000, mas),
        row(Mode::Strict, Side::Black, R::Win, 130, 1, 1_000, human),
        row(Mode::Strict, Side::White, R::Win, 140, 2, 2_000, nov),
        row(Mode::Strict, Side::Black, R::Loss, 150, 3, 3_000, mas),
    ]
}

#[test]
fn ut_dom_090_aggregates_equal_hand_computed_totals() {
    let s = aggregate(&seeded(), None);
    // 3 wins (rows 1, 4, 5), 2 losses (2, 6), 1 draw (3)
    assert_eq!(s.overall, t(3, 2, 1));
    assert_eq!(s.games(), 6);
    // by colour: White rows 1,3,5 = W,D,W; Black rows 2,4,6 = L,W,L
    assert_eq!(s.white, t(2, 0, 1));
    assert_eq!(s.black, t(1, 2, 0));
    // by mode
    assert_eq!(s.training, t(1, 1, 0));
    assert_eq!(s.strict, t(2, 1, 1));
    // by opponent, ordered by id: 10 (Master) rows 2,3,6; 20 (Novice) rows 1,5; 30 (human) row 4
    assert_eq!(
        s.by_opponent,
        vec![
            (pid(10), t(0, 2, 1)),
            (pid(20), t(2, 0, 0)),
            (pid(30), t(1, 0, 0))
        ]
    );
    // by opponent category (the computers' current labels; the human has none), class order
    assert_eq!(
        s.by_category,
        vec![
            (Category::Novice, t(2, 0, 0)),
            (Category::Master, t(0, 2, 1))
        ]
    );
    // average length: (plies + 1) / 2 per game = 21, 15, 30, 1, 1, 2 -> 70 / 6
    assert_eq!(s.avg_moves, Some(70.0 / 6.0));
    // average time = (60000 + 30000 + 90000 + 1000 + 2000 + 3000) / 6
    assert_eq!(s.avg_time_ms, Some(186_000.0 / 6.0));
}

#[test]
fn ut_dom_091_d_44_uncounted_games_are_dropped_by_aggregate_itself() {
    let o = opp(10, PlayerKind::Computer, Some(Category::Master));
    let good = row(Mode::Strict, Side::White, R::Win, 100, 10, 1000, o);
    assert!(counts_in_aggregates(&good));
    let variants: Vec<(&str, StatsRow)> = vec![
        (
            "self play",
            StatsRow {
                self_play: true,
                ..good.clone()
            },
        ),
        (
            "custom start",
            StatsRow {
                custom_start: true,
                ..good.clone()
            },
        ),
        (
            "aborted",
            StatsRow {
                status: GameStatus::Aborted,
                result_for_viewed: None,
                ..good.clone()
            },
        ),
        (
            "voided",
            StatsRow {
                status: GameStatus::Voided,
                result_for_viewed: None,
                ..good.clone()
            },
        ),
        (
            "unfinished",
            StatsRow {
                status: GameStatus::InProgress,
                result_for_viewed: None,
                ..good.clone()
            },
        ),
        (
            "imported",
            StatsRow {
                status: GameStatus::Imported,
                ..good.clone()
            },
        ),
        (
            "completed without a result",
            StatsRow {
                result_for_viewed: None,
                ..good.clone()
            },
        ),
    ];
    for (name, bad) in &variants {
        assert!(!counts_in_aggregates(bad), "{name}");
        let s = aggregate(std::slice::from_ref(bad), None);
        assert_eq!(s, aggregate(&[], None), "{name}");
        // mixed with a good row, only the good row is counted
        let s = aggregate(&[bad.clone(), good.clone()], None);
        assert_eq!(s.overall, t(1, 0, 0), "{name}");
        assert_eq!(s.games(), 1);
    }
    // a draw and a loss count as themselves
    let d = StatsRow {
        result_for_viewed: Some(R::Draw),
        ..good.clone()
    };
    let l = StatsRow {
        result_for_viewed: Some(R::Loss),
        ..good
    };
    assert_eq!(aggregate(&[d, l], None).overall, t(0, 1, 1));
}

#[test]
fn ut_dom_092_cr_4_15_empty_state_counts_zero_and_means_none() {
    let s = aggregate(&[], None);
    assert_eq!(s, PlayerStats::default());
    assert_eq!(s.overall, t(0, 0, 0));
    assert_eq!(s.games(), 0);
    assert_eq!(s.avg_moves, None);
    assert_eq!(s.avg_time_ms, None);
    assert!(s.by_opponent.is_empty() && s.by_category.is_empty());
    // all rows filtered out is the same empty state, not NaN or 0
    let mut r = seeded();
    for x in &mut r {
        x.self_play = true;
    }
    assert_eq!(aggregate(&r, None).avg_moves, None);
    // the series of an empty player is its baseline point
    let base = point(1, 5, Track::Strict, None, 1200, 1200);
    assert_eq!(rating_series(&[base], Track::Strict, None), vec![base]);
    assert_eq!(rating_series(&[], Track::Strict, None), vec![]);
}

#[test]
fn ut_dom_093_q_76_reset_window_for_the_own_view_only() {
    let rows = seeded(); // started_at 100, 110, 120, 130, 140, 150
    // own view: reset at 130 keeps rows 4, 5, 6 (a game started exactly at the reset counts)
    let own = aggregate(&rows, Some(WallMs(130)));
    assert_eq!(own.games(), 3);
    assert_eq!(own.overall, t(2, 1, 0));
    // an opponent's view passes None and keeps every game against the reset Player
    assert_eq!(aggregate(&rows, None).games(), 6);
    // boundaries
    assert_eq!(aggregate(&rows, Some(WallMs(100))).games(), 6);
    assert_eq!(aggregate(&rows, Some(WallMs(101))).games(), 5);
    assert_eq!(aggregate(&rows, Some(WallMs(151))).games(), 0);
    assert_eq!(aggregate(&rows, Some(WallMs(i64::MIN))).games(), 6);
}

#[test]
fn ut_dom_095_ui_2_average_length_is_in_full_moves() {
    let o = opp(10, PlayerKind::Computer, Some(Category::Master));
    for (plies, moves) in [
        (0u32, 0.0),
        (1, 1.0),
        (2, 1.0),
        (3, 2.0),
        (4, 2.0),
        (41, 21.0),
    ] {
        let r = row(Mode::Strict, Side::White, R::Win, 1, plies, 0, o);
        assert_eq!(aggregate(&[r], None).avg_moves, Some(moves), "{plies}");
    }
    // the mean is over counted games only
    let a = row(Mode::Strict, Side::White, R::Win, 1, 3, 10, o);
    let b = row(Mode::Strict, Side::White, R::Win, 1, 7, 20, o);
    let s = aggregate(&[a, b], None);
    assert_eq!(s.avg_moves, Some(3.0));
    assert_eq!(s.avg_time_ms, Some(15.0));
    // the largest ply count does not overflow
    let big = row(Mode::Strict, Side::White, R::Win, 1, u32::MAX, u64::MAX, o);
    assert!(aggregate(&[big.clone(), big], None).avg_moves.is_some());
}

#[test]
fn ut_dom_096_output_order_is_deterministic() {
    let rows = seeded();
    let mut reversed = rows.clone();
    reversed.reverse();
    assert_eq!(aggregate(&rows, None), aggregate(&reversed, None));
    for _ in 0..5 {
        assert_eq!(aggregate(&rows, None), aggregate(&rows, None));
    }
    let ids: Vec<PlayerId> = aggregate(&reversed, None)
        .by_opponent
        .iter()
        .map(|x| x.0)
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

// ------------------------------------------------------------------ series, history

fn point(
    id: u64,
    at: i64,
    track: Track,
    game: Option<u128>,
    before: i32,
    after: i32,
) -> RatingPoint {
    RatingPoint {
        id,
        at: WallMs(at),
        track,
        game_id: game.map(gid).map(|g: GameId| g),
        rating_before: before,
        rating_after: after,
        k: 32,
    }
}

#[test]
fn ut_dom_092_rating_series_filters_the_track_orders_and_applies_the_window() {
    let pts = vec![
        point(4, 300, Track::Strict, Some(2), 1216, 1210),
        point(1, 100, Track::Strict, None, 1200, 1200),
        point(2, 200, Track::Training, None, 1200, 1200),
        point(3, 200, Track::Strict, Some(1), 1200, 1216),
        point(5, 200, Track::Strict, Some(9), 1216, 1216), // same time as id 3: ordered by id
    ];
    let s = rating_series(&pts, Track::Strict, None);
    assert_eq!(s.iter().map(|p| p.id).collect::<Vec<_>>(), vec![1, 3, 5, 4]);
    let t = rating_series(&pts, Track::Training, None);
    assert_eq!(t.iter().map(|p| p.id).collect::<Vec<_>>(), vec![2]);
    // from the reset point on: at >= reset_at
    let w = rating_series(&pts, Track::Strict, Some(WallMs(200)));
    assert_eq!(w.iter().map(|p| p.id).collect::<Vec<_>>(), vec![3, 5, 4]);
    let w = rating_series(&pts, Track::Strict, Some(WallMs(201)));
    assert_eq!(w.iter().map(|p| p.id).collect::<Vec<_>>(), vec![4]);
}

#[test]
fn ut_dom_094_elo_1_check_history_accepts_a_valid_series_with_a_reset_point() {
    let ok = vec![
        point(1, 100, Track::Strict, None, 1200, 1200),
        point(2, 200, Track::Strict, Some(1), 1200, 1216),
        point(3, 300, Track::Strict, Some(2), 1216, 1190),
        // reset point: no game, before == after
        point(4, 400, Track::Strict, None, 1190, 1190),
        point(5, 500, Track::Strict, Some(3), 1190, 1200),
        // another track interleaved does not disturb this one
        point(6, 250, Track::Training, None, 400, 400),
        point(7, 260, Track::Training, Some(1), 400, 420),
    ];
    assert_eq!(check_history(&ok, Track::Strict), Ok(()));
    assert_eq!(check_history(&ok, Track::Training), Ok(()));
    assert_eq!(check_history(&[], Track::Strict), Ok(()));
}

#[test]
fn ut_dom_094_check_history_rejects_a_gap_a_bad_first_row_and_a_bad_null_game_row() {
    let base = vec![
        point(1, 100, Track::Strict, None, 1200, 1200),
        point(2, 200, Track::Strict, Some(1), 1200, 1216),
        point(3, 300, Track::Strict, Some(2), 1216, 1190),
    ];
    // a gap: the second row starts where the first did not end
    let mut gap = base.clone();
    gap[2].rating_before = 1217;
    assert_eq!(
        check_history(&gap, Track::Strict),
        Err(HistoryError::Gap { id: 3 })
    );
    // the first row has a game
    let mut first = base.clone();
    first[0].game_id = Some(gid(7));
    assert_eq!(
        check_history(&first, Track::Strict),
        Err(HistoryError::FirstRowNotInitial { id: 1 })
    );
    // the first row has before != after
    let mut first = base.clone();
    first[0].rating_after = 1210;
    first[1].rating_before = 1210;
    assert_eq!(
        check_history(&first, Track::Strict),
        Err(HistoryError::FirstRowNotInitial { id: 1 })
    );
    // a later row without a game must not change the rating
    let mut bad = base;
    bad.push(point(4, 400, Track::Strict, None, 1190, 1195));
    assert_eq!(
        check_history(&bad, Track::Strict),
        Err(HistoryError::NullGameChangesRating { id: 4 })
    );
    // a row of the other track is ignored
    bad.pop();
    bad.push(point(4, 400, Track::Training, None, 1, 2));
    assert_eq!(check_history(&bad, Track::Strict), Ok(()));
    assert!(check_history(&bad, Track::Training).is_err());
    for e in [
        HistoryError::Gap { id: 1 },
        HistoryError::FirstRowNotInitial { id: 1 },
        HistoryError::NullGameChangesRating { id: 1 },
    ] {
        assert!(!e.to_string().is_empty());
    }
}

#[test]
fn ut_dom_094_check_history_orders_by_time_then_id_before_checking() {
    let shuffled = vec![
        point(3, 300, Track::Strict, Some(2), 1216, 1190),
        point(1, 100, Track::Strict, None, 1200, 1200),
        point(2, 100, Track::Strict, Some(1), 1200, 1216),
    ];
    assert_eq!(check_history(&shuffled, Track::Strict), Ok(()));
}

#[test]
fn df_02_tally_games_saturates_instead_of_overflowing() {
    // GATE-103 rework DF-02: `games()` must not panic (debug) or wrap (release).
    let t = Tally {
        wins: u32::MAX,
        losses: 1,
        draws: 1,
    };
    assert_eq!(t.games(), u32::MAX);
    let all_max = Tally {
        wins: u32::MAX,
        losses: u32::MAX,
        draws: u32::MAX,
    };
    assert_eq!(all_max.games(), u32::MAX);
    // below the limit the sum is exact
    let small = Tally {
        wins: 2,
        losses: 3,
        draws: 4,
    };
    assert_eq!(small.games(), 9);
    let stats = PlayerStats {
        overall: t,
        ..PlayerStats::default()
    };
    assert_eq!(stats.games(), u32::MAX);
}
