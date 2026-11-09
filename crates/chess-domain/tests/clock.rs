//! UT-DOM-010 to 014, 103, 122 (clock part): the clock with fake time, ClockSpec, TimeTotals.
//! Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    Accrual, Clock, ClockSpec, ClockSpecError, ClockValues, GameState, MonoMs, TimeSource,
    TimeTotals, time_accrual,
};
use chess_rules::Side;
use proptest::prelude::*;

fn timed(minutes: u32, seconds: u32) -> Clock {
    Clock::new(Some(common::spec(minutes, seconds)))
}

#[test]
fn ut_dom_010_rule_4_start_stop_switch_and_increment_after_the_move() {
    let t = common::FakeTime::new(0, 5_000);
    let mut c = timed(5, 2); // 300 s + 2 s
    c.start(Side::White, t.mono());
    t.advance(10_000);
    assert_eq!(c.remaining(Side::White, t.mono()), Some(290_000));
    assert_eq!(c.remaining(Side::Black, t.mono()), Some(300_000));
    let timing = c.after_move(Side::White, t.mono());
    assert_eq!(timing.think_ms, 10_000);
    assert_eq!(
        timing.remaining_after,
        Some(292_000),
        "290 s left plus the 2 s increment"
    );
    assert_eq!(c.running_side(), Some(Side::Black));
    t.advance(3_000);
    assert_eq!(c.remaining(Side::Black, t.mono()), Some(297_000));
    assert_eq!(
        c.remaining(Side::White, t.mono()),
        Some(292_000),
        "the idle side loses nothing"
    );
    let timing = c.after_move(Side::Black, t.mono());
    assert_eq!(
        (timing.think_ms, timing.remaining_after),
        (3_000, Some(299_000))
    );
}

#[test]
fn ut_dom_010_rule_4_pause_and_resume_do_not_charge_time() {
    let t = common::FakeTime::new(0, 0);
    let mut c = timed(1, 0);
    c.start(Side::White, t.mono());
    t.advance(4_000);
    c.stop(t.mono());
    t.advance(500_000); // paused for a long time
    assert_eq!(c.remaining(Side::White, t.mono()), Some(56_000));
    c.start(Side::White, t.mono());
    t.advance(1_000);
    let timing = c.after_move(Side::White, t.mono());
    assert_eq!(
        timing.think_ms, 5_000,
        "only running time: 4 s before the pause, 1 s after"
    );
    assert_eq!(timing.remaining_after, Some(55_000));
}

#[test]
fn ut_dom_011_rule_4_flag_deadline_and_zero_floor() {
    let t = common::FakeTime::new(100, 0);
    let mut c = timed(1, 0);
    assert_eq!(c.flag_deadline(t.mono()), None, "nothing runs yet");
    c.start(Side::White, t.mono());
    assert_eq!(
        c.flag_deadline(t.mono()),
        Some((Side::White, MonoMs(60_100)))
    );
    t.advance(20_000);
    assert_eq!(
        c.flag_deadline(t.mono()),
        Some((Side::White, MonoMs(60_100))),
        "same instant"
    );
    c.after_move(Side::White, t.mono());
    assert_eq!(
        c.flag_deadline(t.mono()),
        Some((Side::Black, MonoMs(80_100))),
        "re-armed for the other side"
    );
    t.advance(1_000_000);
    assert_eq!(
        c.remaining(Side::Black, t.mono()),
        Some(0),
        "never negative"
    );
    c.stop(t.mono());
    assert_eq!(c.remaining(Side::Black, t.mono()), Some(0));
}

#[test]
fn ut_dom_011_untimed_clock_has_no_deadline_and_no_remaining() {
    let t = common::FakeTime::new(0, 0);
    let mut c = Clock::new(None);
    c.start(Side::White, t.mono());
    assert_eq!(c.flag_deadline(t.mono()), None);
    assert_eq!(c.remaining(Side::White, t.mono()), None);
    assert_eq!(c.checkpoint(t.mono()), None);
}

#[test]
fn ut_dom_011_ife_2_a_backwards_reading_is_zero_elapsed() {
    let mut c = timed(1, 0);
    c.start(Side::White, MonoMs(1_000));
    assert_eq!(c.remaining(Side::White, MonoMs(500)), Some(60_000));
    c.stop(MonoMs(500));
    assert_eq!(c.remaining(Side::White, MonoMs(500)), Some(60_000));
}

proptest! {
    #[test]
    fn ut_dom_012_rule_4_remaining_at_turn_start_minus_elapsed_is_remaining_now(
        segments in proptest::collection::vec((0u64..5_000, 0u64..5_000), 1..8)
    ) {
        // Within one turn: run for a, pause for b, run again ... then stop.
        let mut now = 0u64;
        let mut c = timed(5, 0);
        let start_remaining = c.remaining(Side::White, MonoMs(now)).unwrap();
        for (run, pause) in &segments {
            c.start(Side::White, MonoMs(now));
            now += run;
            c.stop(MonoMs(now));
            now += pause;
        }
        let ran: u64 = segments.iter().map(|(r, _)| r).sum();
        prop_assert_eq!(c.turn_elapsed_ms(), ran);
        prop_assert_eq!(c.remaining(Side::White, MonoMs(now)).unwrap(), start_remaining - ran.min(start_remaining));
    }
}

#[test]
fn ut_dom_014_dat_3_untimed_game_still_reports_think_ms() {
    let mut c = Clock::new(None);
    c.start(Side::White, MonoMs(10));
    let timing = c.after_move(Side::White, MonoMs(7_010));
    assert_eq!(timing.think_ms, 7_000);
    assert_eq!(timing.remaining_after, None);
}

#[test]
fn ut_dom_014_rule_5_restore_from_a_checkpoint_rebuilds_the_turn() {
    let t = common::FakeTime::new(0, 0);
    let mut c = timed(1, 0);
    c.start(Side::White, t.mono());
    t.advance(6_000);
    let cp = c.checkpoint(t.mono()).unwrap();
    assert_eq!(
        cp,
        ClockValues {
            white_ms: 54_000,
            black_ms: 60_000
        }
    );

    let mut r = timed(1, 0);
    r.restore(cp);
    assert_eq!(r.running_side(), None, "replay leaves the clock stopped");
    assert_eq!(r.remaining(Side::White, MonoMs(0)), Some(54_000));
    assert_eq!(
        r.turn_elapsed_ms(),
        6_000,
        "turn_elapsed is the start-of-turn value minus the checkpoint"
    );
    r.start(Side::White, MonoMs(1_000));
    let timing = r.after_move(Side::White, MonoMs(3_000));
    assert_eq!(timing.think_ms, 8_000);
}

#[test]
fn ut_dom_103_xc_23_clock_spec_presets_and_conversions() {
    for (m, s, text) in [
        (1, 0, "60+0"),
        (3, 2, "180+2"),
        (5, 0, "300+0"),
        (10, 5, "600+5"),
        (15, 10, "900+10"),
        (30, 0, "1800+0"),
    ] {
        let spec = ClockSpec::from_minutes_plus_seconds(m, s).unwrap();
        assert_eq!(spec.to_spec_text(), text);
        assert_eq!(ClockSpec::from_spec_text(text).unwrap(), spec);
    }
    let five = ClockSpec::from_minutes_plus_seconds(5, 0).unwrap();
    assert_eq!((five.base_ms(), five.increment_ms()), (300_000, 0));
    assert_eq!(
        ClockSpec::from_seconds(300, 0).unwrap(),
        five,
        "PGN TimeControl seconds map directly"
    );
}

#[test]
fn ut_dom_103_tbd_34_limits_edges() {
    assert!(ClockSpec::from_seconds(1, 0).is_ok());
    assert!(ClockSpec::from_seconds(86_400, 3_600).is_ok());
    assert_eq!(
        ClockSpec::from_seconds(0, 0),
        Err(ClockSpecError::BaseOutOfRange)
    );
    assert_eq!(
        ClockSpec::from_seconds(86_401, 0),
        Err(ClockSpecError::BaseOutOfRange)
    );
    assert_eq!(
        ClockSpec::from_seconds(60, 3_601),
        Err(ClockSpecError::IncrementOutOfRange)
    );
    assert_eq!(
        ClockSpec::from_minutes_plus_seconds(1_441, 0),
        Err(ClockSpecError::BaseOutOfRange)
    );
    assert_eq!(
        ClockSpec::from_minutes_plus_seconds(u32::MAX, 0),
        Err(ClockSpecError::BaseOutOfRange),
        "no overflow"
    );
}

#[test]
fn ut_dom_121_sf_11_clock_spec_text_form() {
    assert_eq!(
        ClockSpec::from_seconds(300, 0).unwrap().to_spec_text(),
        "300+0"
    );
    for bad in [
        "05+0",
        "5 + 0",
        "+5",
        "5+",
        "5",
        "",
        "-5+0",
        "5+-1",
        "5+0+1",
        "0x5+0",
        "+5+0",
        "5.0+0",
        " 5+0",
        "5+0 ",
        "0+0",
        "99999999999+0",
    ] {
        assert!(
            ClockSpec::from_spec_text(bad).is_err(),
            "{bad:?} must be rejected"
        );
    }
    assert!(ClockSpec::from_spec_text("1+0").is_ok());
}

#[test]
fn ut_dom_122_d_63_time_accrual_table() {
    use GameState::*;
    let w = Side::White;
    let charged = Accrual {
        side: Some(w),
        total: true,
    };
    for s in [AwaitingMove, EngineThinking, EngineError] {
        assert_eq!(time_accrual(&s, w), charged, "{s:?}");
    }
    assert_eq!(
        time_accrual(&AwaitingMove, Side::Black).side,
        Some(Side::Black)
    );
    assert_eq!(
        time_accrual(&DrawOffered, w),
        Accrual {
            side: None,
            total: true
        }
    );
    for s in [Paused, Suspended, Completed, Finalized, Aborted, Voided] {
        assert_eq!(
            time_accrual(&s, w),
            Accrual {
                side: None,
                total: false
            },
            "{s:?}"
        );
    }
}

#[test]
fn ut_dom_122_d_63_time_totals_add_saturates_and_charges() {
    let mut t = TimeTotals::default();
    t.add(
        Accrual {
            side: Some(Side::White),
            total: true,
        },
        1_000,
    );
    t.add(
        Accrual {
            side: Some(Side::Black),
            total: true,
        },
        250,
    );
    t.add(
        Accrual {
            side: None,
            total: true,
        },
        40,
    );
    t.add(
        Accrual {
            side: None,
            total: false,
        },
        9_999,
    );
    assert_eq!(
        (t.white_used_ms, t.black_used_ms, t.total_active_ms),
        (1_000, 250, 1_290)
    );
    assert!(t.total_active_ms >= t.white_used_ms + t.black_used_ms);
    t.add(
        Accrual {
            side: Some(Side::White),
            total: true,
        },
        u64::MAX,
    );
    assert_eq!((t.white_used_ms, t.total_active_ms), (u64::MAX, u64::MAX));
}

proptest! {
    #[test]
    fn ut_dom_122_d_63_total_active_never_below_the_sides_over_random_state_sequences(
        steps in proptest::collection::vec((0usize..10, any::<bool>(), 0u64..100_000), 0..60)
    ) {
        use GameState::*;
        let states = [AwaitingMove, EngineThinking, EngineError, DrawOffered, Paused, Suspended, Completed, Finalized, Aborted, Voided];
        let mut t = TimeTotals::default();
        for (i, white, ms) in steps {
            let side = if white { Side::White } else { Side::Black };
            t.add(time_accrual(&states[i], side), ms);
            prop_assert!(t.total_active_ms >= t.white_used_ms.saturating_add(t.black_used_ms));
        }
    }
}
