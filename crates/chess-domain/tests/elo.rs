//! UT-DOM-050 to 059: Elo (ELO-1, ELO-2), category bands, rating eligibility. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
use chess_domain::{
    Category, EloCalc, EloError, GameFacts, GameResult, GameStatus, Mode, RatedSide, RatingTrack,
    Track, UnratedReason, round_half_away,
};
use proptest::prelude::*;

fn side(rating: i32, games: u32) -> RatedSide {
    RatedSide {
        rating,
        rated_games_before: games,
    }
}

fn deltas(w: RatedSide, b: RatedSide, r: GameResult) -> (i32, i32) {
    let s = EloCalc::settle(w, b, r).unwrap();
    (s.white.delta, s.black.delta)
}

#[test]
fn ut_dom_050_elo_1_expected_score_hand_values() {
    let eq = EloCalc::expected_score(1200, 1200);
    assert!((eq - 0.5).abs() < 1e-12);
    assert!((EloCalc::expected_score(1400, 1200) - 0.759_746_926_6).abs() < 1e-9);
    assert!((EloCalc::expected_score(1600, 1200) - 0.909_090_909_1).abs() < 1e-9);
    assert!(
        (EloCalc::expected_score(1200, 1400) - 0.240_253_073_4).abs() < 1e-9,
        "the weaker side expects the complement"
    );
    for (a, b) in [(1200, 1200), (1500, 900), (400, 2500), (-300, 2800)] {
        let sum = EloCalc::expected_score(a, b) + EloCalc::expected_score(b, a);
        assert!((sum - 1.0).abs() < 1e-9, "{a} v {b}: sum {sum}");
    }
}

#[test]
fn ut_dom_051_elo_1_exact_integer_deltas_equal_ratings() {
    let (w, b) = (side(1200, 40), side(1200, 40));
    assert_eq!(deltas(w, b, GameResult::WhiteWins), (16, -16));
    assert_eq!(deltas(w, b, GameResult::Draw), (0, 0));
    assert_eq!(deltas(w, b, GameResult::BlackWins), (-16, 16));
}

#[test]
fn ut_dom_051_elo_1_exact_integer_deltas_mixed_k() {
    // A (white) has fewer than 30 rated games: K 40. B has 30 or more: K 32.
    let (w, b) = (side(1200, 5), side(1200, 30));
    assert_eq!(deltas(w, b, GameResult::WhiteWins), (20, -16));
}

#[test]
fn ut_dom_051_elo_1_exact_integer_deltas_1400_v_1200() {
    let (w, b) = (side(1400, 100), side(1200, 100));
    assert_eq!(deltas(w, b, GameResult::WhiteWins), (8, -8));
    assert_eq!(deltas(w, b, GameResult::Draw), (-8, 8));
    assert_eq!(deltas(w, b, GameResult::BlackWins), (-24, 24));
}

#[test]
fn ut_dom_051_elo_1_settlement_reports_before_after_k_counter() {
    let s = EloCalc::settle(side(1400, 29), side(1200, 31), GameResult::BlackWins).unwrap();
    assert_eq!(
        (s.white.before, s.white.k, s.white.counter_after),
        (1400, 40, 30)
    );
    assert_eq!(
        (s.black.before, s.black.k, s.black.counter_after),
        (1200, 32, 32)
    );
    assert_eq!(s.white.after, s.white.before + s.white.delta);
    assert_eq!(s.black.after, s.black.before + s.black.delta);
    assert_eq!(s.white.delta, -30, "40 * (0 - 0.7597) = -30.39");
    assert_eq!(s.black.delta, 24, "32 * (1 - 0.2403) = 24.31");
}

#[test]
fn ut_dom_052_elo_1_round_half_away_from_zero() {
    for (x, want) in [
        (0.5, 1.0),
        (-0.5, -1.0),
        (1.5, 2.0),
        (-1.5, -2.0),
        (2.4, 2.0),
        (-2.6, -3.0),
        (0.0, 0.0),
        (2.5, 3.0),
        (-2.5, -3.0),
    ] {
        assert_eq!(round_half_away(x), want, "round({x})");
    }
}

#[test]
fn ut_dom_053_elo_1_k_switches_after_exactly_30_rated_games() {
    assert_eq!(EloCalc::k_factor(0), 40);
    assert_eq!(EloCalc::k_factor(29), 40, "game 30 has 29 before it");
    assert_eq!(EloCalc::k_factor(30), 32, "game 31 has 30 before it");
    assert_eq!(EloCalc::k_factor(5000), 32);
}

#[test]
fn ut_dom_053_elo_2_the_counter_is_per_side_and_saturates() {
    let s = EloCalc::settle(side(1000, u32::MAX), side(1000, 0), GameResult::Draw).unwrap();
    assert_eq!(s.white.counter_after, u32::MAX);
    assert_eq!(s.black.counter_after, 1);
}

#[test]
fn ut_dom_056_elo_1_no_clamp_above_2500_and_below_start() {
    let s = EloCalc::settle(side(2600, 50), side(100, 50), GameResult::BlackWins).unwrap();
    assert!(s.black.after > 100 && s.white.after < 2600);
    let s = EloCalc::settle(side(3000, 50), side(3000, 50), GameResult::WhiteWins).unwrap();
    assert_eq!(s.white.after, 3016);
    let s = EloCalc::settle(side(-50, 50), side(-50, 50), GameResult::BlackWins).unwrap();
    assert_eq!(s.white.after, -66, "negative ratings are not clamped");
}

#[test]
fn ut_dom_054_elo_1_overflow_is_an_error_not_a_wrap() {
    let r = EloCalc::settle(
        side(i32::MAX, 50),
        side(i32::MAX, 50),
        GameResult::WhiteWins,
    );
    assert_eq!(r, Err(EloError::RatingOverflow));
    let r = EloCalc::settle(
        side(i32::MIN, 50),
        side(i32::MIN, 50),
        GameResult::BlackWins,
    );
    assert_eq!(r, Err(EloError::RatingOverflow));
}

proptest! {
    #[test]
    fn ut_dom_054_elo_1_delta_bounds_sign_and_symmetry(
        wr in -500i32..3500, br in -500i32..3500, wg in 0u32..80, bg in 0u32..80, res in 0usize..3
    ) {
        let result = [GameResult::WhiteWins, GameResult::Draw, GameResult::BlackWins][res];
        let s = EloCalc::settle(side(wr, wg), side(br, bg), result).unwrap();
        let sw = [1.0, 0.5, 0.0][res];
        let ew = EloCalc::expected_score(wr, br);
        let (kw, kb) = (EloCalc::k_factor(wg), EloCalc::k_factor(bg));
        prop_assert!(s.white.delta.abs() <= kw && s.black.delta.abs() <= kb);
        prop_assert_eq!((s.white.k, s.black.k), (kw, kb));
        if sw - ew > 0.0 { prop_assert!(s.white.delta >= 0); }
        if sw - ew < 0.0 { prop_assert!(s.white.delta <= 0); }
        if kw == kb { prop_assert_eq!(s.white.delta, -s.black.delta); }
        prop_assert_eq!(s.white.after, wr + s.white.delta);
        prop_assert_eq!(s.black.after, br + s.black.delta);
    }
}

#[test]
fn ut_dom_055_elo_2_rating_track_rated_cases() {
    let f = |status, mode, self_play, custom_start| GameFacts {
        status,
        mode,
        self_play,
        custom_start,
    };
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Completed, Mode::Training, false, false)),
        RatingTrack::Rated(Track::Training)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Completed, Mode::Strict, false, false)),
        RatingTrack::Rated(Track::Strict)
    );
}

#[test]
fn ut_dom_055_rule_8_rating_track_unrated_cases() {
    let f = |status, self_play, custom_start| GameFacts {
        status,
        mode: Mode::Strict,
        self_play,
        custom_start,
    };
    let un = |r| RatingTrack::Unrated(r);
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Completed, true, false)),
        un(UnratedReason::SelfPlay)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Completed, false, true)),
        un(UnratedReason::CustomStart)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Imported, false, false)),
        un(UnratedReason::Imported)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Aborted, false, false)),
        un(UnratedReason::Aborted)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Voided, false, false)),
        un(UnratedReason::Voided)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::InProgress, false, false)),
        un(UnratedReason::NotCompleted)
    );
    // Precedence: self-play wins over custom start wins over status.
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Voided, true, true)),
        un(UnratedReason::SelfPlay)
    );
    assert_eq!(
        EloCalc::rating_track(&f(GameStatus::Voided, false, true)),
        un(UnratedReason::CustomStart)
    );
}

#[test]
fn ut_dom_057_elo_2_a_training_game_settles_on_training_inputs_only() {
    // settle() takes only the two RatedSide values the caller chose. Feeding the Training
    // rating and counter must give a result that depends on nothing else: same inputs, same
    // output whatever the "other track" holds (the function cannot see it).
    let a = EloCalc::settle(side(900, 3), side(1300, 3), GameResult::WhiteWins).unwrap();
    let b = EloCalc::settle(side(900, 3), side(1300, 3), GameResult::WhiteWins).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.white.k, 40);
}

#[test]
fn ut_dom_059_elo_1_category_bands_at_the_edges() {
    for (rating, want) in [
        (-100, Category::Beginner),
        (0, Category::Beginner),
        (599, Category::Beginner),
        (600, Category::Novice),
        (1199, Category::Novice),
        (1200, Category::Intermediate),
        (2199, Category::Intermediate),
        (2200, Category::Master),
        (2499, Category::Master),
        (2500, Category::GrandMaster),
        (3500, Category::GrandMaster),
    ] {
        assert_eq!(Category::from_rating(rating), want, "rating {rating}");
    }
}

#[test]
fn ut_dom_059_ply_4_baselines_strictly_increase_and_sit_in_their_band() {
    let base: Vec<i32> = Category::ALL.iter().map(|c| c.baseline()).collect();
    assert_eq!(base, [400, 600, 1200, 2200, 2500]);
    assert!(base.windows(2).all(|w| w[0] < w[1]));
    for c in Category::ALL {
        assert_eq!(Category::from_rating(c.baseline()), *c);
    }
    assert_eq!(Category::GrandMaster.as_str(), "Grand Master");
}
