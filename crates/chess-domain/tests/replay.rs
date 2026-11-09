//! UT-DOM-035, 042, 044, 108, 117 to 119, 122 to 124, FZ-DOM-001: facts, replay, terminal
//! re-check, SAN policy, time totals. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    Actor, DropCause, EndReason, Fact, Game, GameCommand as C, GameError, GameResult, GameSetup,
    GameState as S, MoveCheck, Outcome, ReplayError, ReplayItem, ReplayWarning, SuspendCause,
    TimeTotals, VoidReason, WallMs, time_accrual,
};
use chess_rules::Side;
use common::*;
use proptest::prelude::*;

/// Property-test case count; `PROPTEST_CASES` raises it for a soak run.
fn cases(default: u32) -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn untimed_hh() -> GameSetup {
    let mut s = strict_hh();
    s.clock = None;
    s
}

fn timed_hh() -> GameSetup {
    let mut s = strict_hh();
    s.clock = Some(spec(1, 2));
    s
}

fn setups() -> Vec<GameSetup> {
    vec![
        training(),
        timed_hh(),
        strict_hc(),
        strict_cc(),
        untimed_hh(),
    ]
}

/// One scripted step of the random driver: a command chosen by `choice`, parameterised by `idx`.
fn candidate(game: &Game, choice: u8, idx: u16) -> Option<C> {
    let legal = game.position().legal_moves();
    let to_move = game.turn();
    let computer_to_move = game.context().kind_of(to_move) == chess_domain::PlayerKind::Computer;
    Some(match choice % 26 {
        0..=7 => {
            if computer_to_move && game.state() == S::AwaitingMove {
                C::EngineStarted
            } else {
                let mv = *legal.get(usize::from(idx) % legal.len().max(1))?;
                C::Move {
                    mv,
                    expected_ply: game.ply(),
                    actor: if computer_to_move {
                        Actor::Engine
                    } else {
                        Actor::Human
                    },
                }
            }
        }
        8 => C::EngineStarted,
        9 => C::EngineFailed,
        10 => C::EngineRetry,
        11 => C::Takeback,
        12 => C::OfferDraw { by: to_move },
        13 => C::RespondDraw {
            accept: idx.is_multiple_of(2),
        },
        14 => C::DropDrawOffer,
        15 => C::Resign {
            side: if idx.is_multiple_of(2) {
                to_move
            } else {
                to_move.other()
            },
        },
        16 => C::Flag,
        17 => C::Pause,
        18 => C::PlayResume,
        19 => C::Step,
        20 => C::Suspend {
            cause: [
                SuspendCause::Detach,
                SuspendCause::Shutdown,
                SuspendCause::Recovered,
            ][usize::from(idx) % 3],
        },
        21 | 22 => C::SuspendResume,
        23 => C::Checkpoint,
        24 => C::Abort,
        _ => match idx % 4 {
            0 => C::Void {
                reason: VoidReason::Reset,
                reset_player: Some(pid(1)),
            },
            1 => C::MarkFinalized,
            2 => C::ResolveTerminal,
            _ => C::Void {
                reason: VoidReason::Unreadable,
                reset_player: None,
            },
        },
    })
}

fn drive(setup: GameSetup, script: &[(u8, u16, u32)]) -> Run {
    let mut r = Run::new(setup);
    for (choice, idx, dt) in script {
        r.tick(u64::from(*dt % 4000));
        if let Some(c) = candidate(&r.game, *choice, *idx) {
            let _ = r.try_cmd(c);
        }
    }
    r
}

fn script_strategy(max: usize) -> impl Strategy<Value = Vec<(u8, u16, u32)>> {
    proptest::collection::vec((any::<u8>(), any::<u16>(), any::<u32>()), 0..max)
}

fn ended_count(facts: &[Fact]) -> usize {
    facts
        .iter()
        .filter(|f| matches!(f, Fact::Ended { .. }))
        .count()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(96)))]

    #[test]
    fn ut_dom_042_rule_5_replay_equals_the_live_game_and_is_idempotent(
        which in 0usize..5, script in script_strategy(120)
    ) {
        let run = drive(setups()[which].clone(), &script);
        prop_assert_eq!(run.game.check_invariants(), Ok(()));
        let out = run.replay();
        prop_assert_eq!(out.error, None);
        prop_assert_eq!(out.applied, run.facts.len());
        let replayed = out.game.unwrap();
        prop_assert_eq!(replayed.check_invariants(), Ok(()));
        prop_assert!(replayed.journal_eq(&run.game), "live {:?} replayed {:?}", run.game, replayed);
        let again = run.replay();
        prop_assert_eq!((again.applied, again.interrupted, again.terminal), (out.applied, out.interrupted, out.terminal));
        prop_assert!(again.game.unwrap().journal_eq(&replayed));
    }

    #[test]
    fn ut_dom_035_nfr_3_random_playouts_never_reach_an_illegal_state(
        script in proptest::collection::vec(any::<u16>(), 0..220)
    ) {
        let mut r = Run::new(untimed_hh());
        for idx in script {
            let legal = r.game.position().legal_moves();
            if r.game.state() != S::AwaitingMove || legal.is_empty() { break; }
            let mv = legal[usize::from(idx) % legal.len()];
            let c = C::Move { mv, expected_ply: r.game.ply(), actor: Actor::Human };
            r.cmd(c);
            prop_assert_eq!(r.game.check_invariants(), Ok(()));
        }
        let n = ended_count(&r.facts);
        prop_assert!(n <= 1, "at most one Ended fact");
        prop_assert_eq!(n == 1, r.game.state() == S::Completed);
        if let Some(o) = r.game.outcome() {
            match o.reason {
                EndReason::Checkmate => prop_assert!(r.game.position().is_checkmate()),
                EndReason::Stalemate => prop_assert!(r.game.position().is_stalemate()),
                _ => {}
            }
        }
        if r.game.state() == S::Completed {
            let resigned = r.try_cmd(C::Resign { side: Side::White });
            prop_assert!(resigned.is_err());
        }
    }
}

// ------------------------------------------------------------------ UT-DOM-044

fn base_facts() -> Vec<Fact> {
    let mut r = Run::new(untimed_hh());
    r.mv("e2e4");
    r.mv("e7e5");
    r.mv("g1f3");
    r.facts
}

#[test]
fn ut_dom_044_dat_7_replay_of_a_good_journal_applies_everything() {
    let facts = base_facts();
    let out = Game::replay(items(&facts));
    assert_eq!(
        (out.applied, out.error, out.interrupted, out.terminal),
        (4, None, true, None)
    );
    assert!(out.warnings.is_empty());
    let g = out.game.unwrap();
    assert_eq!((g.ply(), g.state()), (3, S::AwaitingMove));
}

#[test]
fn ut_dom_044_replay_stops_at_a_bad_fact_and_reports_applied_and_the_error() {
    let good = base_facts();
    let at_index = |i: usize, f: Fact| {
        let mut v = good.clone();
        v[i] = f;
        v
    };
    let mv_fact = |ply, uci: &str| Fact::Move {
        ply,
        mv: mvu(uci),
        think_ms: Some(0),
        clocks_after: None,
        played_at: WallMs(0),
    };

    // wrong ply
    let out = Game::replay(items(&at_index(2, mv_fact(5, "e7e5"))));
    assert_eq!(
        (out.applied, out.error),
        (
            2,
            Some(ReplayError::WrongPly {
                expected: 1,
                found: 5
            })
        )
    );
    assert_eq!(out.game.unwrap().ply(), 1, "the game built so far");
    // illegal move
    let out = Game::replay(items(&at_index(2, mv_fact(1, "e1e5"))));
    assert_eq!(out.applied, 2);
    assert!(
        matches!(out.error, Some(ReplayError::Illegal(_))),
        "{:?}",
        out.error
    );
    // a second Start
    let mut v = good.clone();
    v.push(good[0].clone());
    let out = Game::replay(items(&v));
    assert_eq!(
        (out.applied, out.error),
        (4, Some(ReplayError::SecondStart))
    );
    // the first fact is not a Start
    let out = Game::replay(items(&good[1..]));
    assert_eq!((out.applied, out.error), (0, Some(ReplayError::NotStarted)));
    assert!(out.game.is_none());
    let out = Game::replay(Vec::<ReplayItem>::new());
    assert_eq!(
        (out.applied, out.error, out.game.is_none()),
        (0, Some(ReplayError::NotStarted), true)
    );
    // a takeback beyond the moves played
    let mut v = good.clone();
    v.push(Fact::Takeback { to_ply: 9 });
    let out = Game::replay(items(&v));
    assert_eq!(
        (out.applied, out.error),
        (4, Some(ReplayError::BadTakeback))
    );
    // a fact that does not fit the state: a draw answer without an offer
    let mut v = good.clone();
    v.push(Fact::DrawDeclined { by: Side::White });
    assert!(matches!(
        Game::replay(items(&v)).error,
        Some(ReplayError::WrongState { .. })
    ));
    // an invalid recorded setup
    let mut bad = good.clone();
    if let Fact::Start { setup, .. } = &mut bad[0] {
        setup.mode = chess_domain::Mode::Training;
    }
    let out = Game::replay(items(&bad));
    assert!(
        matches!(out.error, Some(ReplayError::BadSetup(_))),
        "{:?}",
        out.error
    );
    assert_eq!(out.applied, 0);
}

#[test]
fn ut_dom_105_d_41_after_suspend_only_resume_checkpoint_and_terminals_are_accepted() {
    let mut r = Run::new(untimed_hh());
    r.mv("e2e4");
    r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    let m = Fact::Move {
        ply: 1,
        mv: mvu("e7e5"),
        think_ms: Some(0),
        clocks_after: None,
        played_at: WallMs(0),
    };
    for bad in [
        m.clone(),
        Fact::Pause,
        Fact::DrawOffer { by: Side::Black },
        Fact::Takeback { to_ply: 0 },
        Fact::Suspend {
            cause: SuspendCause::Detach,
        },
    ] {
        let mut v = r.facts.clone();
        v.push(bad.clone());
        let out = Game::replay(items(&v));
        assert_eq!(out.error, Some(ReplayError::NotResumed), "{bad:?}");
        assert_eq!(out.applied, r.facts.len());
    }
    for ok in [
        Fact::Clock {
            clocks: None,
            running: None,
        },
        Fact::SuspendResume,
        Fact::Ended {
            outcome: Outcome {
                result: GameResult::BlackWins,
                reason: EndReason::Resignation,
            },
        },
        Fact::Voided {
            reason: VoidReason::Unreadable,
            reset_player: None,
        },
    ] {
        let mut v = r.facts.clone();
        v.push(ok.clone());
        assert_eq!(Game::replay(items(&v)).error, None, "{ok:?}");
    }
    // an interrupted journal reports `interrupted`; a suspended one does not
    assert!(!Game::replay(items(&r.facts)).interrupted);
    assert!(Game::replay(items(&base_facts())).interrupted);
    // Ended makes it non-interrupted too
    let mut f = base_facts();
    f.push(Fact::Ended {
        outcome: Outcome {
            result: GameResult::Draw,
            reason: EndReason::DrawAgreed,
        },
    });
    let out = Game::replay(items(&f));
    assert_eq!((out.interrupted, out.error), (false, None));
    assert_eq!(out.game.unwrap().state(), S::Completed);
}

#[test]
fn ut_dom_105_d_58_draw_dropped_is_emitted_only_where_the_design_says() {
    // Over the random driver, DrawDropped appears only in steps of Suspend, EngineFailed or
    // DropDrawOffer.
    let mut seen = 0;
    for first in [
        C::Suspend {
            cause: SuspendCause::Detach,
        },
        C::EngineFailed,
        C::DropDrawOffer,
    ] {
        let mut r = Run::new(training());
        r.cmd(C::OfferDraw { by: Side::White });
        let step = r.cmd(first);
        assert_eq!(
            step.facts
                .iter()
                .filter(|f| matches!(f, Fact::DrawDropped { .. }))
                .count(),
            1
        );
        seen += 1;
    }
    for seed in 0u8..40 {
        let mut r = Run::new(training());
        for i in 0u16..80 {
            let choice = seed
                .wrapping_mul(7)
                .wrapping_add(u8::try_from(i % 251).unwrap())
                .wrapping_mul(13);
            let Some(c) = candidate(&r.game, choice, i) else {
                continue;
            };
            let allowed = matches!(c, C::Suspend { .. } | C::EngineFailed | C::DropDrawOffer);
            if let Ok(step) = r.try_cmd(c.clone()) {
                let dropped = step
                    .facts
                    .iter()
                    .any(|f| matches!(f, Fact::DrawDropped { .. }));
                assert!(!dropped || allowed, "{c:?} emitted {:?}", step.facts);
                seen += usize::from(dropped);
            }
        }
    }
    assert!(seen > 0, "the driver does reach a dropped offer");
}

// ------------------------------------------------------------------ UT-DOM-117 terminal re-check

fn without_ended(mut r: Run) -> (Run, Outcome) {
    let o = match r.facts.pop() {
        Some(Fact::Ended { outcome }) => outcome,
        other => panic!("expected the Ended fact last, got {other:?}"),
    };
    (r, o)
}

#[test]
fn ut_dom_117_d_65_a_terminal_position_without_ended_is_reported_and_resolved() {
    let cases: Vec<(Run, Vec<&str>)> = vec![
        (Run::new(untimed_hh()), vec!["f2f3", "e7e5", "g2g4", "d8h4"]),
        (
            Run::new(with_start(untimed_hh(), "k7/8/1K6/8/8/8/8/2Q5 w - - 0 1")),
            vec!["c1c7"],
        ),
        (
            Run::new(with_start(untimed_hh(), "7k/8/8/8/8/1p6/8/K2B4 w - - 0 1")),
            vec!["d1b3"],
        ),
        (
            Run::new(untimed_hh()),
            vec![
                "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8",
            ],
        ),
        (
            Run::new(with_start(untimed_hh(), "4k3/8/8/8/8/8/8/R3K3 w - - 99 80")),
            vec!["a1a2"],
        ),
    ];
    let reasons = [
        EndReason::Checkmate,
        EndReason::Stalemate,
        EndReason::InsufficientMaterial,
        EndReason::ThreefoldRepetition,
        EndReason::FiftyMoveRule,
    ];
    for ((mut run, moves), reason) in cases.into_iter().zip(reasons) {
        for m in moves {
            run.mv(m);
        }
        let (run, live) = without_ended(run);
        assert_eq!(live.reason, reason);
        let out = Game::replay(items(&run.facts));
        assert_eq!(out.error, None);
        assert_eq!(out.terminal, Some(live), "{reason:?}");
        assert!(
            !out.interrupted,
            "a terminal position is not an interrupted game"
        );
        let game = out.game.unwrap();
        assert_eq!(game.pending_termination(), Some(live));
        let step = game.decide(C::ResolveTerminal, at(99)).unwrap();
        assert_eq!(
            step.facts,
            vec![Fact::Ended { outcome: live }],
            "no Suspend, SuspendResume or clock restart"
        );
        assert_eq!(step.ended, Some(live));
        assert_eq!(step.game.state(), S::Completed);
    }
}

#[test]
fn ut_dom_117_resolve_terminal_on_a_live_position_is_state_not_allowed() {
    let mut r = Run::new(untimed_hh());
    r.mv("e2e4");
    assert_eq!(r.game.pending_termination(), None);
    let e = r.try_cmd(C::ResolveTerminal).unwrap_err();
    assert!(
        matches!(e, GameError::Denied(d) if d.reason == chess_domain::DeniedReason::StateNotAllowed)
    );
    // a start position that is playable but dead is not "terminal" before any move
    let g = Game::new(
        with_start(untimed_hh(), "7k/8/8/8/8/8/8/K1B5 w - - 0 1"),
        at(0),
    )
    .unwrap()
    .game;
    assert_eq!(g.pending_termination(), None);
}

#[test]
fn ut_dom_117_a_flag_fall_is_not_rederived_from_the_journal() {
    let mut r = Run::new(timed_hh());
    r.mv("e2e4");
    r.cmd(C::Checkpoint);
    let out = r.replay();
    assert_eq!((out.terminal, out.interrupted), (None, true));
}

// ------------------------------------------------------------------ UT-DOM-118 SAN policy

fn checked_items(facts: &[Fact], fen_after: &str, san: &str) -> Vec<ReplayItem> {
    let last_move = facts
        .iter()
        .rposition(|f| matches!(f, Fact::Move { .. }))
        .unwrap();
    facts
        .iter()
        .enumerate()
        .map(|(i, f)| ReplayItem {
            fact: f.clone(),
            check: (i == last_move).then(|| MoveCheck {
                san: san.to_string(),
                fen_after: fen_after.to_string(),
            }),
        })
        .collect()
}

#[test]
fn ut_dom_118_d_72_d_79_san_policy() {
    let mut r = Run::new(untimed_hh());
    r.mv("e2e4");
    r.mv("e7e5");
    let fen = r.game.plies()[1].fen_after.clone();
    let san = r.game.plies()[1].san.clone();
    assert_eq!(san, "e5");

    // exact match: no warning
    let out = Game::replay(checked_items(&r.facts, &fen, &san));
    assert_eq!((out.error, out.warnings.len()), (None, 0));
    // SAN-only difference with equal positions: a warning, the computed values are kept
    let out = Game::replay(checked_items(&r.facts, &fen, "e5!?"));
    assert_eq!(out.error, None);
    assert_eq!(
        out.warnings,
        vec![ReplayWarning::SanMismatch {
            ply: 1,
            recorded: "e5!?".into(),
            computed: "e5".into()
        }]
    );
    assert_eq!(out.game.unwrap().plies()[1].san, "e5");
    // a different fen_after with identical SAN: FenMismatch, whatever the SAN says
    let other = fen.replace(" 0 2", " 0 3");
    let out = Game::replay(checked_items(&r.facts, &other, &san));
    assert_eq!(out.error, Some(ReplayError::FenMismatch { ply: 1 }));
    assert_eq!(out.applied, 2, "the mismatching move is not applied");
    // both differ: FenMismatch (D-79 (3))
    let out = Game::replay(checked_items(&r.facts, &other, "Nf6"));
    assert_eq!(out.error, Some(ReplayError::FenMismatch { ply: 1 }));
    // an unparseable recorded fen_after counts as a mismatch
    for junk in ["", "not a fen", "8/8/8/8/8/8/8/8 w - - 0 1"] {
        let out = Game::replay(checked_items(&r.facts, junk, &san));
        assert_eq!(
            out.error,
            Some(ReplayError::FenMismatch { ply: 1 }),
            "{junk:?}"
        );
    }
    // any other field differing still mismatches (c)
    for variant in [
        fen.replace(" KQkq", " Kkq"),
        fen.replace(" 0 2", " 5 2"),
        fen.replace("4p3/4P3", "4p3/4N3"),
    ] {
        assert_ne!(variant, fen);
        let out = Game::replay(checked_items(&r.facts, &variant, &san));
        assert_eq!(
            out.error,
            Some(ReplayError::FenMismatch { ply: 1 }),
            "{variant}"
        );
    }
    // an illegal uci is an Illegal error even with a check attached
    let mut bad = r.facts.clone();
    if let Some(Fact::Move { mv, .. }) = bad.last_mut() {
        *mv = mvu("e1e5");
    }
    let out = Game::replay(checked_items(&bad, &fen, &san));
    assert!(matches!(out.error, Some(ReplayError::Illegal(_))));
    // no MoveCheck: nothing is compared
    assert_eq!(Game::replay(items(&r.facts)).error, None);
}

#[test]
fn ut_dom_118_xc_dom_a8_5_fixture_a_no_capture_possible_en_passant_spellings() {
    let mut r = Run::new(untimed_hh());
    r.mv("e2e4");
    let with_ep = r.game.plies()[0].fen_after.clone();
    assert!(
        with_ep.contains(" e3 "),
        "the rules writer emits the square after every double push"
    );
    let without_ep = with_ep.replace(" e3 ", " - ");
    for fen in [&with_ep, &without_ep] {
        let out = Game::replay(checked_items(&r.facts, fen, "e4"));
        assert_eq!(
            out.error, None,
            "{fen}: no capture exists, both spellings describe one position"
        );
    }
}

#[test]
fn ut_dom_118_xc_dom_a8_5_fixture_b_capture_possible_dash_is_a_mismatch() {
    let mut r = Run::new(untimed_hh());
    for m in ["a2a3", "d7d5", "a3a4", "d5d4", "e2e4"] {
        r.mv(m);
    }
    let with_ep = r.game.plies()[4].fen_after.clone();
    assert!(with_ep.contains(" e3 "));
    let out = Game::replay(checked_items(&r.facts, &with_ep, "e4"));
    assert_eq!(out.error, None);
    let dash = with_ep.replace(" e3 ", " - ");
    let out = Game::replay(checked_items(&r.facts, &dash, "e4"));
    assert_eq!(
        out.error,
        Some(ReplayError::FenMismatch { ply: 4 }),
        "'-' says no capture exists, which is false"
    );
}

// ------------------------------------------------------------------ UT-DOM-119

#[test]
fn ut_dom_119_sf_22_draw_dropped_by_mismatch_is_a_warning() {
    let mut r = Run::new(training());
    r.cmd(C::OfferDraw { by: Side::White });
    let mut facts = r.facts.clone();
    facts.push(Fact::DrawDropped {
        by: Side::Black,
        cause: DropCause::EngineFailure,
    });
    let out = Game::replay(items(&facts));
    assert_eq!(out.error, None);
    assert_eq!(
        out.warnings,
        vec![ReplayWarning::DrawDroppedBy {
            recorded: Side::Black,
            pending: Side::White
        }]
    );
    let g = out.game.unwrap();
    assert_eq!(
        (g.state(), g.pending_offer()),
        (S::AwaitingMove, None),
        "continues with the pending offer's side"
    );
    // no pending offer: the fact does not fit
    let mut facts = Run::new(training()).facts;
    facts.push(Fact::DrawDropped {
        by: Side::White,
        cause: DropCause::Suspend,
    });
    assert!(matches!(
        Game::replay(items(&facts)).error,
        Some(ReplayError::WrongState { .. })
    ));
}

#[test]
fn ut_dom_119_d_67_ended_then_voided_replays_to_voided_and_finalized_cannot_be_voided() {
    let mut r = Run::new(untimed_hh());
    r.cmd(C::Resign { side: Side::White });
    let mut facts = r.facts.clone();
    facts.push(Fact::Voided {
        reason: VoidReason::Reset,
        reset_player: Some(pid(1)),
    });
    let out = Game::replay(items(&facts));
    assert_eq!(out.error, None);
    assert_eq!(out.game.unwrap().state(), S::Voided);
    r.cmd(C::MarkFinalized);
    let e = r
        .game
        .evolve(Fact::Voided {
            reason: VoidReason::Reset,
            reset_player: None,
        })
        .unwrap_err();
    assert!(
        matches!(
            e,
            ReplayError::WrongState {
                state: S::Finalized
            }
        ),
        "{e:?}"
    );
    // Aborted then Voided is damage
    let mut r = Run::new(strict_cc());
    r.cmd(C::Abort);
    assert!(
        r.game
            .evolve(Fact::Voided {
                reason: VoidReason::Unreadable,
                reset_player: None
            })
            .is_err()
    );
}

// ------------------------------------------------------------------ UT-DOM-108

fn kind(f: &Fact) -> &'static str {
    match f {
        Fact::Start { .. } => "Start",
        Fact::Move { .. } => "Move",
        Fact::Takeback { .. } => "Takeback",
        Fact::Clock { .. } => "Clock",
        Fact::DrawOffer { .. } => "DrawOffer",
        Fact::DrawDeclined { .. } => "DrawDeclined",
        Fact::DrawDropped { .. } => "DrawDropped",
        Fact::Suspend { .. } => "Suspend",
        Fact::Pause => "Pause",
        Fact::PlayResume => "Resume",
        Fact::SuspendResume => "SuspendResume",
        Fact::Ended { .. } => "GameEnd",
        Fact::Aborted => "Abort",
        Fact::Voided { .. } => "Voided",
    }
}

#[test]
fn ut_dom_108_d_41_every_fact_variant_maps_to_exactly_one_journal_kind() {
    let o = Outcome {
        result: GameResult::Draw,
        reason: EndReason::DrawAgreed,
    };
    let one_of_each = vec![
        Fact::Start {
            setup: training(),
            started_at: WallMs(1),
        },
        Fact::Move {
            ply: 0,
            mv: mvu("e2e4"),
            think_ms: None,
            clocks_after: None,
            played_at: WallMs(1),
        },
        Fact::Takeback { to_ply: 0 },
        Fact::Clock {
            clocks: None,
            running: None,
        },
        Fact::DrawOffer { by: Side::White },
        Fact::DrawDeclined { by: Side::White },
        Fact::DrawDropped {
            by: Side::White,
            cause: DropCause::Suspend,
        },
        Fact::Suspend {
            cause: SuspendCause::Detach,
        },
        Fact::Pause,
        Fact::PlayResume,
        Fact::SuspendResume,
        Fact::Ended { outcome: o },
        Fact::Aborted,
        Fact::Voided {
            reason: VoidReason::Reset,
            reset_player: None,
        },
    ];
    let mut names: Vec<&str> = one_of_each.iter().map(kind).collect();
    assert_eq!(names.len(), 14);
    for (f, name) in one_of_each.iter().zip(names.clone()) {
        assert_eq!(f.journal_kind(), name);
    }
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        14,
        "the 12 kinds of the architecture plus DrawDropped and SuspendResume"
    );
}

// ------------------------------------------------------------------ UT-DOM-122..124 totals

fn mfact(ply: u32, think: Option<u64>) -> Fact {
    Fact::Move {
        ply,
        mv: mvu("e2e4"),
        think_ms: think,
        clocks_after: None,
        played_at: WallMs(0),
    }
}

#[test]
fn ut_dom_122_d_78_seed_sums_think_ms_over_every_move_including_taken_back_plies() {
    let facts = vec![
        Fact::Start {
            setup: untimed_hh(),
            started_at: WallMs(0),
        },
        mfact(0, Some(100)),
        mfact(1, Some(200)),
        Fact::Takeback { to_ply: 0 },
        mfact(0, Some(50)),
        Fact::Clock {
            clocks: None,
            running: Some(Side::Black),
        },
        mfact(1, None),
        Fact::DrawOffer { by: Side::White },
        Fact::DrawDeclined { by: Side::White },
    ];
    let t = TimeTotals::seed_from_facts(&facts);
    assert_eq!(
        (t.white_used_ms, t.black_used_ms),
        (150, 200),
        "a takeback does not refund time; a move without think_ms adds 0"
    );
    assert_eq!(t.total_active_ms, 350, "white + black");
    // the mover comes from the start position's side to move
    let black_first = vec![
        Fact::Start {
            setup: with_start(
                untimed_hh(),
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
            ),
            started_at: WallMs(0),
        },
        mfact(0, Some(7)),
        mfact(1, Some(9)),
    ];
    let t = TimeTotals::seed_from_facts(&black_first);
    assert_eq!((t.white_used_ms, t.black_used_ms), (9, 7));
    assert_eq!(TimeTotals::seed_from_facts(&[]), TimeTotals::default());
}

fn accrue(live: &mut TimeTotals, never: &mut TimeTotals, r: &Run, mark: &mut u64) {
    let a = time_accrual(&r.game.state(), r.game.turn());
    let dt = r.now - *mark;
    live.add(a, dt);
    never.add(a, dt);
    *mark = r.now;
}

fn sum_think(facts: &[Fact]) -> (u64, u64) {
    let (mut w, mut b) = (0, 0);
    for f in facts {
        if let Fact::Move { ply, think_ms, .. } = f {
            if ply % 2 == 0 {
                w += think_ms.unwrap_or(0)
            } else {
                b += think_ms.unwrap_or(0)
            }
        }
    }
    (w, b)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(128)))]

    #[test]
    fn ut_dom_123_d_78_seeded_totals_lie_between_the_journal_sum_and_the_never_stopped_run(
        script in proptest::collection::vec((0u8..6, 0u32..5000, 0u32..4000), 1..40)
    ) {
        let mut r = Run::new(untimed_hh());
        let (mut live, mut never, mut mark) = (TimeTotals::default(), TimeTotals::default(), 0u64);
        let mut stops = 0u32;
        for (op, ms, extra) in script {
            if r.game.state().is_finished() { break; }
            r.tick(u64::from(ms));
            accrue(&mut live, &mut never, &r, &mut mark);
            match op {
                0..=2 => {
                    let legal = r.game.position().legal_moves();
                    let mv = legal[usize::try_from(extra).unwrap() % legal.len()];
                    let c = C::Move { mv, expected_ply: r.game.ply(), actor: Actor::Human };
                    r.cmd(c);
                }
                3 => {
                    if r.try_cmd(C::OfferDraw { by: r.game.turn() }).is_ok() {
                        r.tick(u64::from(extra));
                        accrue(&mut live, &mut never, &r, &mut mark);
                        r.cmd(C::RespondDraw { accept: false });
                    }
                }
                4 => {
                    r.cmd(C::Suspend { cause: SuspendCause::Detach });
                    // The process stops: the next one rebuilds the game from the journal.
                    r.game = Game::replay(items(&r.facts)).game.unwrap();
                    r.tick(u64::from(extra)); // the app is closed: nothing accrues
                    accrue(&mut live, &mut never, &r, &mut mark);
                    r.cmd(C::SuspendResume);
                    live = TimeTotals::seed_from_facts(&r.facts);
                    stops += 1;
                }
                _ => { r.cmd(C::Checkpoint); }
            }
        }
        let (lw, lb) = sum_think(&r.facts);
        prop_assert!(lw <= live.white_used_ms && live.white_used_ms <= never.white_used_ms, "white {lw} {} {}", live.white_used_ms, never.white_used_ms);
        prop_assert!(lb <= live.black_used_ms && live.black_used_ms <= never.black_used_ms);
        prop_assert!(lw + lb <= live.total_active_ms && live.total_active_ms <= never.total_active_ms);
        prop_assert!(live.total_active_ms >= live.white_used_ms + live.black_used_ms);
        if stops == 0 { prop_assert_eq!(live, never, "no stop: exact"); }
    }
}

#[test]
fn ut_dom_124_xc_dom_a8_4_draw_offer_time_counts_in_the_total_only() {
    let mut r = Run::new(untimed_hh());
    r.tick(1_000);
    r.mv("e2e4");
    r.tick(500);
    r.cmd(C::OfferDraw { by: Side::Black });
    r.tick(9_000);
    r.cmd(C::RespondDraw { accept: false }); // a closed interval
    r.tick(300);
    r.mv("e7e5");
    r.cmd(C::OfferDraw { by: Side::White }); // open at the stop
    r.tick(4_000);
    r.cmd(C::Suspend {
        cause: SuspendCause::Shutdown,
    });
    let seed = TimeTotals::seed_from_facts(&r.facts);
    assert_eq!(seed.white_used_ms, 1_000);
    assert_eq!(
        seed.black_used_ms,
        500 + 300,
        "Black's turn: 500 before the offer, 300 after it"
    );
    assert_eq!(
        seed.total_active_ms,
        seed.white_used_ms + seed.black_used_ms,
        "the seed counts no draw-offer time"
    );
    let mut live = seed;
    live.add(time_accrual(&S::DrawOffered, Side::White), 2_000);
    assert!(live.total_active_ms >= live.white_used_ms + live.black_used_ms);
    assert_eq!(live.total_active_ms, seed.total_active_ms + 2_000);
}

// ------------------------------------------------------------------ FZ-DOM-001

fn arb_setup() -> impl Strategy<Value = GameSetup> {
    (0usize..5).prop_map(|i| setups()[i].clone())
}

fn arb_uci() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "e2e4", "e7e5", "g1f3", "b8c6", "e1e5", "a2a3", "d7d5", "e1g1", "a7a8q", "h1h8", "e2e2",
    ])
}

fn arb_fact() -> impl Strategy<Value = Fact> {
    let side = prop::sample::select(vec![Side::White, Side::Black]);
    let outcome = (0usize..3, 0usize..12).prop_map(|(r, e)| Outcome {
        result: [
            GameResult::WhiteWins,
            GameResult::BlackWins,
            GameResult::Draw,
        ][r],
        reason: chess_domain::EndReason::ALL[e],
    });
    prop_oneof![
        arb_setup().prop_map(|setup| Fact::Start {
            setup,
            started_at: WallMs(0)
        }),
        (0u32..8, arb_uci(), any::<Option<u64>>(), any::<bool>()).prop_map(
            |(ply, m, think_ms, with_clock)| Fact::Move {
                ply,
                mv: mvu(m),
                think_ms,
                clocks_after: with_clock.then_some(chess_domain::ClockValues {
                    white_ms: 5,
                    black_ms: u64::MAX
                }),
                played_at: WallMs(-1),
            }
        ),
        (0u32..10).prop_map(|to_ply| Fact::Takeback { to_ply }),
        Just(Fact::Clock {
            clocks: None,
            running: None
        }),
        side.clone().prop_map(|by| Fact::DrawOffer { by }),
        side.clone().prop_map(|by| Fact::DrawDeclined { by }),
        side.clone().prop_map(|by| Fact::DrawDropped {
            by,
            cause: DropCause::Suspend
        }),
        Just(Fact::Suspend {
            cause: SuspendCause::Detach
        }),
        Just(Fact::Pause),
        Just(Fact::PlayResume),
        Just(Fact::SuspendResume),
        outcome.prop_map(|outcome| Fact::Ended { outcome }),
        Just(Fact::Aborted),
        Just(Fact::Voided {
            reason: VoidReason::Reset,
            reset_player: None
        }),
    ]
}

fn arb_check() -> impl Strategy<Value = Option<MoveCheck>> {
    prop::option::of(
        ("[ -~]{0,12}", "[ -~]{0,60}").prop_map(|(san, fen_after)| MoveCheck { san, fen_after }),
    )
}

fn assert_outcome_sane(items: Vec<ReplayItem>) -> Result<(), TestCaseError> {
    let n = items.len();
    let out = Game::replay(items);
    prop_assert!(out.applied <= n);
    prop_assert!(!(out.terminal.is_some() && out.error.is_some()));
    if let Some(g) = &out.game {
        prop_assert_eq!(g.check_invariants(), Ok(()));
    } else {
        prop_assert_eq!(out.applied, 0);
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(200)))]

    #[test]
    fn fz_dom_001_nfr_3_replay_on_arbitrary_fact_sequences_never_panics(
        first_is_start in any::<bool>(), setup in arb_setup(),
        rest in proptest::collection::vec((arb_fact(), arb_check()), 0..30)
    ) {
        let mut items = Vec::new();
        if first_is_start {
            items.push(ReplayItem { fact: Fact::Start { setup, started_at: WallMs(0) }, check: None });
        }
        items.extend(rest.into_iter().map(|(fact, check)| ReplayItem { fact, check }));
        assert_outcome_sane(items)?;
    }

    #[test]
    fn fz_dom_001_nfr_3_replay_of_mutated_random_legal_journals_never_panics(
        which in 0usize..5, script in script_strategy(80),
        cuts in proptest::collection::vec((any::<u8>(), any::<u8>()), 0..6),
        checks in proptest::collection::vec(arb_check(), 0..8)
    ) {
        let run = drive(setups()[which].clone(), &script);
        let mut facts = run.facts.clone();
        for (op, at) in cuts {
            if facts.is_empty() { break; }
            let i = usize::from(at) % facts.len();
            match op % 3 {
                0 => { facts.remove(i); }
                1 => { let f = facts[i].clone(); facts.insert(i, f); }
                _ => { let j = usize::from(op) % facts.len(); facts.swap(i, j); }
            }
        }
        let mut checks = checks.into_iter();
        let items = facts.into_iter().map(|fact| {
            let check = if matches!(fact, Fact::Move { .. }) { checks.next().flatten() } else { None };
            ReplayItem { fact, check }
        }).collect();
        assert_outcome_sane(items)?;
    }
}

#[test]
fn ut_dom_042_rule_5_pause_step_pause_replays_although_step_has_no_fact() {
    // Found by the UT-DOM-042 property test (shrunk to Pause, Step, Pause in a c-c game).
    let mut r = Run::new(strict_cc());
    for c in [C::Pause, C::Step, C::Pause] {
        r.cmd(c);
    }
    let out = r.replay();
    assert_eq!(out.error, None);
    assert!(out.game.unwrap().journal_eq(&r.game));
}

#[test]
fn ut_dom_044_a_wrong_ply_is_reported_before_any_move_check() {
    let mut r = Run::new(untimed_hh());
    r.mv("e2e4");
    let mut facts = r.facts.clone();
    if let Some(Fact::Move { ply, .. }) = facts.last_mut() {
        *ply = 4;
    }
    let out = Game::replay(checked_items(&facts, "junk", "e4"));
    assert_eq!(
        out.error,
        Some(ReplayError::WrongPly {
            expected: 0,
            found: 4
        })
    );
}

#[test]
fn ut_dom_044_abort_replays_from_suspended_and_from_a_live_state() {
    let mut r = Run::new(strict_cc());
    r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    r.cmd(C::Abort);
    let out = r.replay();
    assert_eq!(out.error, None);
    assert_eq!(out.game.unwrap().state(), S::Aborted);
}

// ------------------------------------------------------------------ B5 / D1(b): final clocks
//
// Ruling B5 (Brian Prascher, 2026-11-09): the final clocks of a finished game are NOT
// journaled by the writer (it writes no `Clock` fact before `Ended`). `Game::journal_eq`
// therefore does not compare the remaining times of a finished game, and (RC-02) it does not
// compare them even when a journal does hold a `Clock` checkpoint before `Ended`. These tests
// assert the documented difference instead of hiding it (DF-01), show that think time,
// outcome, plies and the clock spec are still compared, and pin exactly what is not compared.

fn remaining_ms(game: &Game, side: Side, now: u64) -> Option<u64> {
    game.clock().remaining(side, at(now).mono)
}

#[test]
fn ut_dom_042_b5_flag_final_clock_live_is_zero_replay_is_the_full_base_and_journal_eq_is_true() {
    // DF-01 reproduction 1: strict h-h, 10 s base, Flag decided at 11 s.
    let mut setup = strict_hh();
    setup.clock = Some(chess_domain::ClockSpec::from_seconds(10, 0).unwrap());
    let mut r = Run::new(setup);
    r.tick(11_000);
    r.cmd(C::Flag);
    assert_eq!(r.game.state(), S::Completed);
    let out = r.replay();
    assert_eq!(out.error, None);
    let replayed = out.game.unwrap();
    assert_eq!(remaining_ms(&r.game, Side::White, 11_000), Some(0), "live");
    assert_eq!(
        remaining_ms(&replayed, Side::White, 11_000),
        Some(10_000),
        "replay: the final clock is not journaled"
    );
    assert!(
        replayed.journal_eq(&r.game),
        "journal_eq documents that finished-game clocks are not compared"
    );
}

#[test]
fn ut_dom_042_b5_resign_final_clock_live_100000_replay_300000_and_journal_eq_is_true() {
    // DF-01 reproduction 2: 5+0, White moves at 1 s, Black resigns at 201 s.
    let mut r = Run::new(strict_hh());
    r.tick(1_000);
    r.mv("e2e4");
    r.tick(200_000);
    r.cmd(C::Resign { side: Side::Black });
    assert_eq!(r.game.state(), S::Completed);
    let replayed = r.replay().game.unwrap();
    assert_eq!(
        remaining_ms(&r.game, Side::Black, 201_000),
        Some(100_000),
        "live"
    );
    assert_eq!(
        remaining_ms(&replayed, Side::Black, 201_000),
        Some(300_000),
        "replay"
    );
    assert!(replayed.journal_eq(&r.game));
}

#[test]
fn ut_dom_042_b5_no_clock_fact_is_written_before_ended() {
    let mut r = Run::new(strict_hh());
    r.tick(1_000);
    r.mv("e2e4");
    r.tick(200_000);
    let step = r.cmd(C::Resign { side: Side::Black });
    assert_eq!(
        step.facts
            .iter()
            .map(chess_domain::Fact::journal_kind)
            .collect::<Vec<_>>(),
        vec!["GameEnd"],
        "the ending step carries only the Ended fact (no journal contract change)"
    );
}

#[test]
fn ut_dom_042_b5_journal_eq_still_compares_think_time_outcome_and_plies_of_a_finished_game() {
    // Only these three are mutated here. What is NOT compared for a finished game (remaining
    // times, including those restored from a journaled `Clock` fact) is pinned by
    // `ut_dom_042_rc_02_*` below.
    let mut r = Run::new(strict_hh());
    r.tick(1_000);
    r.mv("e2e4");
    r.tick(2_000);
    r.mv("e7e5");
    r.tick(3_000);
    r.cmd(C::Resign { side: Side::White });
    let good = r.replay().game.unwrap();
    assert!(good.journal_eq(&r.game), "control: equal");

    // a different recorded think time
    let mut facts = r.facts.clone();
    for f in &mut facts {
        if let Fact::Move {
            think_ms, ply: 0, ..
        } = f
        {
            *think_ms = Some(1);
        }
    }
    let g = Game::replay(items(&facts)).game.unwrap();
    assert!(!g.journal_eq(&r.game), "think_ms differs");

    // a different outcome
    let mut facts = r.facts.clone();
    for f in &mut facts {
        if let Fact::Ended { outcome } = f {
            outcome.result = GameResult::Draw;
        }
    }
    let g = Game::replay(items(&facts)).game.unwrap();
    assert!(!g.journal_eq(&r.game), "outcome differs");

    // one move fewer, ended the same way
    let mut facts = r.facts.clone();
    let end = facts.pop().unwrap();
    facts.pop();
    facts.push(end);
    let g = Game::replay(items(&facts)).game.unwrap();
    assert!(!g.journal_eq(&r.game), "plies differ");
}

#[test]
fn ut_dom_042_rc_02_a_finished_game_compares_the_clock_spec_but_no_remaining_time() {
    // RC-02 (re-check): documents exactly what `journal_eq` does and does not compare once a
    // game is finished (ruling B5 stands, behavior unchanged). A journaled `Clock` fact before
    // `Ended` is applied on replay, then its content is not compared.
    let mut r = Run::new(strict_hh());
    r.tick(1_000);
    r.mv("e2e4");
    r.tick(2_000);
    r.mv("e7e5");
    r.tick(3_000);
    r.cmd(C::Resign { side: Side::White });
    let ended_at = r.facts.len() - 1;
    let checkpoint = |white_ms: u64, running: Option<Side>| Fact::Clock {
        clocks: Some(chess_domain::ClockValues {
            white_ms,
            black_ms: 260_000,
        }),
        running,
    };
    let with = |fact: Option<Fact>| {
        let mut facts = r.facts.clone();
        if let Some(f) = fact {
            facts.insert(ended_at, f);
        }
        let out = Game::replay(items(&facts));
        assert_eq!(out.error, None);
        out.game.unwrap()
    };

    // NOT compared: the content of a journaled Clock fact, its running side, or its absence.
    let base = with(Some(checkpoint(250_000, Some(Side::White))));
    assert!(base.journal_eq(&r.game), "control");
    let tiny = with(Some(checkpoint(1, Some(Side::White))));
    assert_eq!(remaining_ms(&tiny, Side::White, 0), Some(1), "applied");
    assert!(tiny.journal_eq(&r.game), "remaining time not compared");
    assert!(tiny.journal_eq(&base));
    assert!(with(Some(checkpoint(250_000, Some(Side::Black)))).journal_eq(&r.game));
    assert!(with(None).journal_eq(&r.game), "a dropped Clock fact");
    assert_ne!(
        remaining_ms(&tiny, Side::White, 0),
        remaining_ms(&r.game, Side::White, 0),
        "the games do differ in remaining time; journal_eq says equal by ruling B5"
    );

    // Compared: the clock spec (same moves, a 6+0 clock instead of 5+0). Think time, outcome
    // and ply count are compared too, see the test above.
    let mut setup = strict_hh();
    setup.clock = Some(spec(6, 0));
    let mut o = Run::new(setup);
    o.tick(1_000);
    o.mv("e2e4");
    o.tick(2_000);
    o.mv("e7e5");
    o.tick(3_000);
    o.cmd(C::Resign { side: Side::White });
    assert!(!o.game.journal_eq(&r.game), "clock spec differs");
}

#[test]
fn ut_dom_042_b5_journal_eq_still_compares_the_clock_while_the_game_is_running() {
    // The clock is skipped only once the game is finished.
    let mut r = Run::new(strict_hh());
    r.tick(1_000);
    r.mv("e2e4");
    let mut facts = r.facts.clone();
    for f in &mut facts {
        if let Fact::Move { clocks_after, .. } = f {
            *clocks_after = Some(chess_domain::ClockValues {
                white_ms: 1,
                black_ms: 300_000,
            });
        }
    }
    let g = Game::replay(items(&facts)).game.unwrap();
    assert!(!g.journal_eq(&r.game), "journaled clock differs while live");
}

// ------------------------------------------------------------------ D2 / DF-05

fn late_clock() -> Fact {
    Fact::Clock {
        clocks: Some(chess_domain::ClockValues {
            white_ms: 7,
            black_ms: 8,
        }),
        running: Some(Side::White),
    }
}

#[test]
fn ut_dom_044_d2_a_clock_fact_after_a_finished_state_is_ignored_not_refused() {
    // Desk review D2(ii): refusing a harmless record would turn it into JournalUnreadable.
    let finished: Vec<(&str, Run)> = {
        let mut done = Run::new(strict_hh());
        done.tick(1_000);
        done.cmd(C::Resign { side: Side::White });
        let mut aborted = Run::new(strict_cc());
        aborted.cmd(C::Abort);
        let mut voided = Run::new(strict_hh());
        voided.cmd(C::Void {
            reason: VoidReason::Reset,
            reset_player: Some(pid(1)),
        });
        let mut finalized = Run::new(strict_hh());
        finalized.cmd(C::Resign { side: Side::White });
        finalized.cmd(C::MarkFinalized);
        vec![
            ("Completed", done),
            ("Aborted", aborted),
            ("Voided", voided),
            ("Finalized", finalized),
        ]
    };
    for (label, run) in finished {
        assert!(run.game.state().is_finished(), "{label}");
        let after = run
            .game
            .evolve(late_clock())
            .unwrap_or_else(|e| panic!("{label}: refused with {e:?}"));
        assert_eq!(after.state(), run.game.state(), "{label}");
        assert_eq!(after.outcome(), run.game.outcome(), "{label}");
        assert!(after.journal_eq(&run.game), "{label}: no field changed");
        assert_eq!(
            remaining_ms(&after, Side::White, 0),
            remaining_ms(&run.game, Side::White, 0),
            "{label}: the clock was not restored from the late fact"
        );
        assert_eq!(
            remaining_ms(&after, Side::Black, 0),
            remaining_ms(&run.game, Side::Black, 0),
            "{label}"
        );
        // through replay: counted as applied, no error
        let mut facts = run.facts.clone();
        facts.push(late_clock());
        let out = Game::replay(items(&facts));
        assert_eq!(out.error, None, "{label}");
        assert_eq!(out.applied, facts.len(), "{label}");
    }
}

#[test]
fn ut_dom_044_d2_a_clock_fact_in_a_live_state_still_restores_the_checkpoint() {
    // The no-op is only for finished states.
    let mut r = Run::new(strict_hh());
    r.tick(1_000);
    r.mv("e2e4");
    let after = r.game.evolve(late_clock()).unwrap();
    assert_eq!(remaining_ms(&after, Side::White, 0), Some(7));
    assert_eq!(remaining_ms(&after, Side::Black, 0), Some(8));
}

#[test]
fn ut_dom_044_d2_an_ended_outcome_is_trusted_as_recorded() {
    // Desk review D2(i): `Ended` is not re-derived from the position. The journal's CRC and the
    // store reconciliation are the integrity check. Pinned: a checkmate outcome on the start
    // position replays as recorded.
    let mut facts = Run::new(untimed_hh()).facts;
    let recorded = Outcome {
        result: GameResult::WhiteWins,
        reason: EndReason::Checkmate,
    };
    facts.push(Fact::Ended { outcome: recorded });
    let out = Game::replay(items(&facts));
    assert_eq!(out.error, None);
    let g = out.game.unwrap();
    assert_eq!(g.state(), S::Completed);
    assert_eq!(g.outcome(), Some(recorded));
    assert!(!g.position().is_checkmate(), "the position contradicts it");
}
