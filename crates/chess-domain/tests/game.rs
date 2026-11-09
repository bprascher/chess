//! UT-DOM-030 to 041, 043, 045, 046, 013, 105, 116, 120: the `Game` state machine, termination,
//! in-play rules and clocks inside a game. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    ActionKind, Actor, ClockValues, Denied, DeniedReason as R, DropCause, EndReason, Fact, Game,
    GameCommand as C, GameError, GameResult, GameSetup, GameState as S, Outcome, SetupReason,
    SuspendCause, VoidReason, WallMs,
};
use chess_rules::{Position, Side};
use common::*;

fn denied(e: GameError) -> R {
    match e {
        GameError::Denied(Denied { reason }) => reason,
        other => panic!("expected Denied, got {other:?}"),
    }
}

fn timed_hh() -> GameSetup {
    let mut s = strict_hh();
    s.clock = Some(spec(1, 2)); // 60 s + 2 s
    s
}

fn untimed_hh() -> GameSetup {
    let mut s = strict_hh();
    s.clock = None;
    s
}

fn play(run: &mut Run, ucis: &[&str]) {
    for u in ucis {
        run.mv(u);
    }
}

// ------------------------------------------------------------------ T1 and the move cycle

#[test]
fn ut_dom_030_t1_new_game_awaits_a_move_and_emits_start() {
    let step = Game::new(training(), at(0)).unwrap();
    assert_eq!(step.game.state(), S::AwaitingMove);
    assert_eq!(step.game.ply(), 0);
    assert_eq!(step.game.turn(), Side::White);
    assert_eq!(step.ended, None);
    assert_eq!(step.facts.len(), 1);
    match &step.facts[0] {
        Fact::Start { setup, started_at } => {
            assert_eq!(*setup, training());
            assert_eq!(
                *started_at,
                at(0).wall,
                "the start stamp is the one wall time"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn ut_dom_030_t1_a_checkmate_or_stalemate_start_is_refused() {
    let e = Game::new(
        with_start(untimed_hh(), "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1"),
        at(0),
    )
    .unwrap_err();
    assert_eq!(e.reason, SetupReason::StartPositionTerminal);
}

#[test]
fn ut_dom_030_t1_t6_t3_the_move_cycle_and_facts() {
    let mut r = Run::new(training());
    r.tick(1_500);
    let step = r.mv("e2e4");
    assert_eq!(r.game.state(), S::AwaitingMove);
    assert_eq!(r.game.ply(), 1);
    assert_eq!(r.game.turn(), Side::Black);
    match &step.facts[..] {
        [
            Fact::Move {
                ply: 0,
                mv,
                think_ms: Some(1_500),
                clocks_after: None,
                played_at,
            },
        ] => {
            assert_eq!(mv.to_uci(), "e2e4");
            assert_eq!(*played_at, WallMs(1_000_000 + 1_500));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(r.game.plies()[0].san, "e4");
    assert_eq!(r.game.plies()[0].think_ms, 1_500);
    assert_eq!(
        r.game.plies()[0].fen_after,
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
    );

    // T2 (EngineStarted has no fact) and T3
    let s = r.cmd(C::EngineStarted);
    assert!(s.facts.is_empty());
    assert_eq!(r.game.state(), S::EngineThinking);
    r.tick(700);
    let step = r.cmd(r.move_cmd("e7e5", Actor::Engine));
    assert_eq!(r.game.state(), S::AwaitingMove);
    assert_eq!(r.game.plies()[1].think_ms, 700);
    assert!(matches!(
        step.facts[..],
        [Fact::Move {
            ply: 1,
            think_ms: Some(700),
            ..
        }]
    ));
}

#[test]
fn ut_dom_030_t4_t5_engine_failure_and_retry() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    r.cmd(C::EngineStarted);
    let s = r.cmd(C::EngineFailed);
    assert!(s.facts.is_empty());
    assert_eq!(r.game.state(), S::EngineError);
    r.cmd(C::EngineRetry);
    assert_eq!(r.game.state(), S::EngineThinking);
    r.cmd(r.move_cmd("c7c5", Actor::Engine));
    assert_eq!(r.game.ply(), 2);
}

#[test]
fn ut_dom_030_every_disallowed_pair_returns_denied_or_a_typed_error() {
    // From AwaitingMove (human to move), Training h-c.
    let mut r = Run::new(training());
    let e = |r: &mut Run, c| r.try_cmd(c).unwrap_err();
    assert_eq!(denied(e(&mut r, C::EngineFailed)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::EngineRetry)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::Pause)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::PlayResume)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::Step)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::SuspendResume)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::Abort)), R::StateNotAllowed);
    assert_eq!(
        denied(e(&mut r, C::RespondDraw { accept: true })),
        R::StateNotAllowed
    );
    assert_eq!(denied(e(&mut r, C::DropDrawOffer)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::MarkFinalized)), R::StateNotAllowed);
    assert_eq!(denied(e(&mut r, C::ResolveTerminal)), R::StateNotAllowed);
    assert_eq!(
        denied(e(&mut r, C::EngineStarted)),
        R::StateNotAllowed,
        "the side to move is a human"
    );
    assert_eq!(
        denied(e(&mut r, C::Resign { side: Side::Black })),
        R::StateNotAllowed,
        "not a human side"
    );
    let wrong_actor = r.move_cmd("e2e4", Actor::Engine);
    assert_eq!(denied(e(&mut r, wrong_actor)), R::StateNotAllowed);
    assert_eq!(r.game.ply(), 0, "nothing changed");
}

// ------------------------------------------------------------------ UT-DOM-034 rejections

#[test]
fn ut_dom_034_game_3_stale_expected_ply_and_illegal_move_change_nothing() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    r.cmd(C::EngineStarted);
    let before = r.game.clone();
    let err = r
        .try_cmd(C::Move {
            mv: mvu("e7e5"),
            expected_ply: 0,
            actor: Actor::Engine,
        })
        .unwrap_err();
    assert_eq!(err, GameError::StalePosition { live_ply: 1 });
    assert_eq!((r.game.ply(), r.game.state()), (1, S::EngineThinking));
    assert!(r.game.journal_eq(&before));
    let err = r
        .try_cmd(C::Move {
            mv: mvu("e7e2"),
            expected_ply: 1,
            actor: Actor::Engine,
        })
        .unwrap_err();
    assert!(matches!(err, GameError::IllegalMove(_)), "{err:?}");
    assert_eq!((r.game.ply(), r.game.state()), (1, S::EngineThinking));
    assert!(r.game.journal_eq(&before));
}

#[test]
fn ut_dom_034_a_human_cannot_move_for_the_computer() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    let err = r.try_cmd(r.move_cmd("e7e5", Actor::Human)).unwrap_err();
    assert_eq!(denied(err), R::NotHumanTurn);
}

// ------------------------------------------------------------------ UT-DOM-031..033 termination

fn ended_of(step: &chess_domain::Step) -> Outcome {
    step.ended.expect("the step ended the game")
}

#[test]
fn ut_dom_031_rule_1_checkmate() {
    let mut r = Run::new(untimed_hh());
    play(&mut r, &["f2f3", "e7e5", "g2g4"]);
    let step = r.mv("d8h4");
    let o = ended_of(&step);
    assert_eq!(
        (o.result, o.reason),
        (GameResult::BlackWins, EndReason::Checkmate)
    );
    assert!(matches!(step.facts[..], [Fact::Move { .. }, Fact::Ended { outcome }] if outcome == o));
    assert_eq!(r.game.state(), S::Completed);
    assert_eq!(r.game.outcome(), Some(o));
}

#[test]
fn ut_dom_031_rule_1_stalemate_dead_position_fifty_move() {
    let cases = [
        (
            "k7/8/1K6/8/8/8/8/2Q5 w - - 0 1",
            "c1c7",
            GameResult::Draw,
            EndReason::Stalemate,
        ),
        (
            "7k/8/8/8/8/1p6/8/K2B4 w - - 0 1",
            "d1b3",
            GameResult::Draw,
            EndReason::InsufficientMaterial,
        ),
        (
            "4k3/8/8/8/8/8/8/R3K3 w - - 99 80",
            "a1a2",
            GameResult::Draw,
            EndReason::FiftyMoveRule,
        ),
        (
            "4k3/8/8/8/8/8/8/R3K3 w - - 98 80",
            "a1a2",
            GameResult::Draw,
            EndReason::FiftyMoveRule,
        ),
    ];
    for (i, (fen, uci, result, reason)) in cases.iter().enumerate() {
        let mut r = Run::new(with_start(untimed_hh(), fen));
        let step = r.mv(uci);
        if i == 3 {
            assert_eq!(step.ended, None, "99 halfmoves only: one short of the rule");
            continue;
        }
        let o = ended_of(&step);
        assert_eq!((o.result, o.reason), (*result, *reason), "{fen}");
    }
}

#[test]
fn ut_dom_031_rule_2_threefold_repetition_counts_the_start_position() {
    let mut r = Run::new(untimed_hh());
    play(
        &mut r,
        &["g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1"],
    );
    assert_eq!(r.game.state(), S::AwaitingMove);
    let step = r.mv("f6g8");
    let o = ended_of(&step);
    assert_eq!(
        (o.result, o.reason),
        (GameResult::Draw, EndReason::ThreefoldRepetition)
    );
}

#[test]
fn ut_dom_031_rule_2_threefold_by_transposition() {
    // The same position by two different move orders, three times.
    let mut r = Run::new(untimed_hh());
    play(
        &mut r,
        &[
            "g1f3", "g8f6", "b1c3", "b8c6", "f3g1", "f6g8", "c3b1", "c6b8",
        ],
    );
    assert_eq!(r.game.state(), S::AwaitingMove, "start position seen twice");
    play(
        &mut r,
        &["b1c3", "g8f6", "g1f3", "b8c6", "c3b1", "f6g8", "f3g1"],
    );
    let step = r.mv("c6b8");
    assert_eq!(ended_of(&step).reason, EndReason::ThreefoldRepetition);
}

#[test]
fn ut_dom_032_rule_1_checkmate_beats_the_fifty_move_count() {
    let mut r = Run::new(with_start(untimed_hh(), "7k/5Q2/6K1/8/8/8/8/8 w - - 99 60"));
    let o = ended_of(&r.mv("f7g7"));
    assert_eq!(
        (o.result, o.reason),
        (GameResult::WhiteWins, EndReason::Checkmate)
    );
}

#[test]
fn ut_dom_033_find_dom_006_stalemate_beats_dead_position_and_threefold_order() {
    // K+Q v K driven to stalemate on the 100th halfmove: stalemate (draw) is reported before the
    // fifty-move rule. Black is left with a bare king, so the position is not dead by material.
    let mut r = Run::new(with_start(untimed_hh(), "k7/8/1K6/8/8/8/8/2Q5 w - - 99 50"));
    let o = ended_of(&r.mv("c1c7"));
    assert_eq!(o.reason, EndReason::Stalemate);
    // Dead position beats the fifty-move count: B captures the last pawn on the 100th halfmove.
    let mut r = Run::new(with_start(
        untimed_hh(),
        "7k/8/8/8/8/1p6/8/K2B4 w - - 99 50",
    ));
    let o = ended_of(&r.mv("d1b3"));
    assert_eq!(o.reason, EndReason::InsufficientMaterial);
}

// ------------------------------------------------------------------ clocks in a game

#[test]
fn ut_dom_010_rule_4_game_clock_runs_from_start_and_adds_the_increment() {
    let mut r = Run::new(timed_hh());
    r.tick(10_000);
    let step = r.mv("e2e4");
    match &step.facts[..] {
        [
            Fact::Move {
                think_ms: Some(10_000),
                clocks_after: Some(c),
                ..
            },
        ] => {
            assert_eq!(
                *c,
                ClockValues {
                    white_ms: 52_000,
                    black_ms: 60_000
                },
                "60 - 10 + 2"
            );
        }
        other => panic!("{other:?}"),
    }
    r.tick(5_000);
    let view = r.game.view(at(r.now));
    assert_eq!(view.clocks.white_ms, Some(52_000));
    assert_eq!(view.clocks.black_ms, Some(55_000));
    assert_eq!(view.clocks.running_side, Some(Side::Black));
}

#[test]
fn ut_dom_013_rule_4_a_move_after_expiry_is_the_flag_result_and_is_not_applied() {
    let mut r = Run::new(timed_hh());
    r.tick(61_000);
    let before = r.game.clone();
    let step = r.cmd(r.move_cmd("e2e4", Actor::Human));
    let o = ended_of(&step);
    assert_eq!(
        (o.result, o.reason),
        (GameResult::BlackWins, EndReason::TimeForfeit)
    );
    assert!(
        matches!(step.facts[..], [Fact::Ended { .. }]),
        "no Move fact"
    );
    assert_eq!(r.game.ply(), before.ply());
    assert_eq!(r.game.state(), S::Completed);
}

#[test]
fn ut_dom_013_rule_4_flag_only_when_the_running_clock_is_at_zero() {
    let mut r = Run::new(timed_hh());
    r.tick(59_999);
    assert_eq!(r.try_cmd(C::Flag).unwrap_err(), GameError::FlagNotFallen);
    r.tick(1);
    let step = r.cmd(C::Flag);
    assert_eq!(ended_of(&step).reason, EndReason::TimeForfeit);
    assert_eq!(ended_of(&step).result, GameResult::BlackWins);
    // untimed games have no flag
    let mut u = Run::new(untimed_hh());
    u.tick(10_000_000);
    assert_eq!(u.try_cmd(C::Flag).unwrap_err(), GameError::FlagNotFallen);
}

#[test]
fn ut_dom_013_rule_4_black_flags_and_a_bare_king_opponent_draws() {
    let mut r = Run::new(timed_hh());
    r.mv("e2e4");
    r.tick(61_000);
    let o = ended_of(&r.cmd(C::Flag));
    assert_eq!(
        (o.result, o.reason),
        (GameResult::WhiteWins, EndReason::TimeForfeit)
    );

    // White has K+Q, Black a bare king: if White flags, Black cannot mate, so a draw.
    let mut r = Run::new(with_start(timed_hh(), "7k/8/8/8/8/8/8/KQ6 w - - 0 1"));
    r.tick(61_000);
    let o = ended_of(&r.cmd(C::Flag));
    assert_eq!(
        (o.result, o.reason),
        (GameResult::Draw, EndReason::TimeoutOpponentCannotMate)
    );
    // The same position, Black flags: White can mate, White wins on time.
    let mut r = Run::new(with_start(timed_hh(), "7k/8/8/8/8/8/8/KQ6 b - - 0 1"));
    r.tick(61_000);
    let o = ended_of(&r.cmd(C::Flag));
    assert_eq!(
        (o.result, o.reason),
        (GameResult::WhiteWins, EndReason::TimeForfeit)
    );
}

#[test]
fn ut_dom_045_eng_4_engine_error_keeps_the_side_to_moves_clock_running() {
    let mut s = strict_hc();
    s.clock = Some(spec(1, 0));
    let mut r = Run::new(s);
    r.mv("e2e4");
    r.cmd(C::EngineStarted);
    r.cmd(C::EngineFailed);
    r.tick(8_000);
    let v = r.game.view(at(r.now));
    assert_eq!(v.state, S::EngineError);
    assert_eq!(v.clocks.running_side, Some(Side::Black));
    assert_eq!(v.clocks.black_ms, Some(52_000));
}

// ------------------------------------------------------------------ takeback

#[test]
fn ut_dom_036_rule_3_takeback_bounds_human_white() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    r.engine("e7e5");
    assert_eq!(r.game.ply(), 2);
    let step = r.cmd(C::Takeback);
    assert!(matches!(step.facts[..], [Fact::Takeback { to_ply: 0 }]));
    assert_eq!((r.game.ply(), r.game.state()), (0, S::AwaitingMove));
    assert_eq!(
        r.try_cmd(C::Takeback).unwrap_err(),
        GameError::NothingToTakeBack
    );
    // during a search: one move
    r.mv("d2d4");
    r.cmd(C::EngineStarted);
    assert_eq!(r.game.state(), S::EngineThinking);
    r.cmd(C::Takeback);
    assert_eq!((r.game.ply(), r.game.state()), (0, S::AwaitingMove));
    // after the reply: two moves
    r.mv("d2d4");
    r.engine("d7d5");
    r.mv("c2c4");
    r.engine("e7e6");
    r.cmd(C::Takeback);
    assert_eq!(r.game.ply(), 2);
}

#[test]
fn ut_dom_036_rule_3_takeback_bounds_human_black_and_custom_start() {
    let mut s = training();
    s.white = computer(2);
    s.black = human(1);
    let mut r = Run::new(s);
    r.engine("e2e4");
    assert_eq!(
        r.try_cmd(C::Takeback).unwrap_err(),
        GameError::NothingToTakeBack,
        "the earliest is after the computer's first move"
    );
    r.mv("e7e5");
    r.engine("g1f3");
    r.cmd(C::Takeback);
    assert_eq!(r.game.ply(), 1);
    assert_eq!(
        r.try_cmd(C::Takeback).unwrap_err(),
        GameError::NothingToTakeBack
    );

    // custom start with Black to move and the human playing Black: the human moves first
    let mut s = training();
    s.white = computer(2);
    s.black = human(1);
    let s = with_start(
        s,
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
    );
    let mut r = Run::new(s);
    r.mv("e7e5");
    r.engine("e2e4");
    r.cmd(C::Takeback);
    assert_eq!(r.game.ply(), 0, "the human's first turn is at ply 0");
}

#[test]
fn ut_dom_038_fm_dom_007_takeback_restores_keys_and_eco_of_a_game_that_never_played_the_moves() {
    let mut a = Run::new(training());
    for (h, e) in [("e2e4", "e7e5"), ("g1f3", "b8c6"), ("f1b5", "a7a6")] {
        a.mv(h);
        a.engine(e);
    }
    assert!(
        a.game.view(at(0)).eco.is_some(),
        "the Ruy Lopez is a named position"
    );
    a.cmd(C::Takeback);
    a.cmd(C::Takeback);
    let mut b = Run::new(training());
    b.mv("e2e4");
    b.engine("e7e5");
    assert_eq!(a.game.ply(), 2);
    assert_eq!(a.game.repetition_keys(), b.game.repetition_keys());
    assert_eq!(
        a.game.repetition_keys().len(),
        3,
        "the start position and two plies"
    );
    assert_eq!(a.game.view(at(0)).eco, b.game.view(at(0)).eco);
    assert!(a.game.journal_eq(&b.game));
}

#[test]
fn ut_dom_037_rule_3_a_takeback_replays_to_the_same_state() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    r.engine("e7e5");
    r.mv("g1f3");
    r.engine("b8c6");
    r.cmd(C::Takeback);
    let out = r.replay();
    assert_eq!(out.error, None);
    assert!(out.game.unwrap().journal_eq(&r.game));
}

// ------------------------------------------------------------------ draw offers

#[test]
fn ut_dom_039_rule_2_offer_decline_block_and_accept() {
    let mut r = Run::new(timed_hh());
    r.tick(4_000);
    let step = r.cmd(C::OfferDraw { by: Side::White });
    assert!(
        matches!(
            step.facts[..],
            [
                Fact::DrawOffer { by: Side::White },
                Fact::Clock {
                    clocks: Some(ClockValues {
                        white_ms: 56_000,
                        black_ms: 60_000
                    }),
                    running: None
                }
            ]
        ),
        "{:?}",
        step.facts
    );
    assert_eq!(r.game.state(), S::DrawOffered);
    r.tick(30_000);
    assert_eq!(
        r.game.view(at(r.now)).clocks.white_ms,
        Some(56_000),
        "both clocks are stopped"
    );
    assert_eq!(r.game.view(at(r.now)).clocks.running_side, None);
    // a second offer while pending, and an answer by decline
    assert_eq!(
        denied(r.try_cmd(C::OfferDraw { by: Side::Black }).unwrap_err()),
        R::DrawOfferPending
    );
    let step = r.cmd(C::RespondDraw { accept: false });
    assert!(matches!(
        step.facts[..],
        [Fact::DrawDeclined { by: Side::White }]
    ));
    assert_eq!(r.game.state(), S::AwaitingMove);
    assert_eq!(
        r.game.view(at(r.now)).clocks.running_side,
        Some(Side::White),
        "the same side to move restarts"
    );
    // repeat offer blocked until the offerer has moved
    assert_eq!(
        denied(r.try_cmd(C::OfferDraw { by: Side::White }).unwrap_err()),
        R::DrawRepeatBlocked
    );
    assert_eq!(r.game.offer_blocked_for(), Some(Side::White));
    r.mv("e2e4");
    assert_eq!(r.game.offer_blocked_for(), None);
    assert_eq!(
        denied(r.try_cmd(C::OfferDraw { by: Side::White }).unwrap_err()),
        R::DrawNotOwnTurn
    );
    r.mv("e7e5");
    r.cmd(C::OfferDraw { by: Side::White });
    let step = r.cmd(C::RespondDraw { accept: true });
    let o = ended_of(&step);
    assert_eq!(
        (o.result, o.reason),
        (GameResult::Draw, EndReason::DrawAgreed)
    );
    assert_eq!(r.game.state(), S::Completed);
}

#[test]
fn ut_dom_039_rule_2_offer_to_the_computer_and_an_engine_failure_drops_it() {
    let mut r = Run::new(training());
    r.cmd(C::OfferDraw { by: Side::White });
    assert_eq!(r.game.pending_offer(), Some(Side::White));
    let step = r.cmd(C::EngineFailed);
    assert!(
        matches!(
            step.facts[..],
            [Fact::DrawDropped {
                by: Side::White,
                cause: DropCause::EngineFailure
            }]
        ),
        "{:?}",
        step.facts
    );
    assert_eq!(
        (r.game.state(), r.game.pending_offer()),
        (S::AwaitingMove, None)
    );
    // the explicit DropDrawOffer command does the same
    r.cmd(C::OfferDraw { by: Side::White });
    let step = r.cmd(C::DropDrawOffer);
    assert!(matches!(
        step.facts[..],
        [Fact::DrawDropped {
            by: Side::White,
            cause: DropCause::EngineFailure
        }]
    ));
    // an offer during a search is refused
    r.mv("e2e4");
    r.cmd(C::EngineStarted);
    assert_eq!(
        denied(r.try_cmd(C::OfferDraw { by: Side::White }).unwrap_err()),
        R::DrawEngineBusy
    );
}

// ------------------------------------------------------------------ pause, step, abort

#[test]
fn ut_dom_040_game_2_pause_step_resume_abort_in_cc() {
    let mut r = Run::new(strict_cc());
    r.cmd(C::EngineStarted);
    let step = r.cmd(C::Pause);
    assert!(matches!(
        step.facts[..],
        [
            Fact::Pause,
            Fact::Clock {
                clocks: None,
                running: None
            }
        ]
    ));
    assert_eq!(r.game.state(), S::Paused);
    r.cmd(C::Step);
    assert_eq!(r.game.state(), S::EngineThinking);
    r.cmd(r.move_cmd("e2e4", Actor::Engine));
    assert_eq!(
        (r.game.state(), r.game.ply()),
        (S::Paused, 1),
        "exactly one move, back to Paused"
    );
    r.cmd(C::Step);
    r.cmd(r.move_cmd("e7e5", Actor::Engine));
    assert_eq!((r.game.state(), r.game.ply()), (S::Paused, 2));
    let step = r.cmd(C::PlayResume);
    assert!(matches!(step.facts[..], [Fact::PlayResume]));
    assert_eq!(r.game.state(), S::AwaitingMove);
    let step = r.cmd(C::Abort);
    assert!(matches!(step.facts[..], [Fact::Aborted]));
    assert_eq!(r.game.state(), S::Aborted);
    assert_eq!(r.game.outcome(), None);
}

#[test]
fn ut_dom_040_none_of_pause_step_abort_outside_cc() {
    for setup in [training(), strict_hc(), strict_hh()] {
        let mut r = Run::new(setup);
        for c in [C::Pause, C::Step, C::PlayResume, C::Abort] {
            assert_eq!(
                denied(r.try_cmd(c.clone()).unwrap_err()),
                R::StateNotAllowed,
                "{c:?}"
            );
        }
    }
}

// ------------------------------------------------------------------ resign, void

#[test]
fn ut_dom_041_rule_2_resign_loses_for_the_resigning_side() {
    let mut r = Run::new(untimed_hh());
    let o = ended_of(&r.cmd(C::Resign { side: Side::Black }));
    assert_eq!(
        (o.result, o.reason),
        (GameResult::WhiteWins, EndReason::Resignation),
        "either side, whoever is to move"
    );
    let mut r = Run::new(untimed_hh());
    let o = ended_of(&r.cmd(C::Resign { side: Side::White }));
    assert_eq!(o.result, GameResult::BlackWins);
    // self-play resignation ends the game
    let mut s = untimed_hh();
    s.black = human(1);
    let mut r = Run::new(s);
    assert_eq!(
        ended_of(&r.cmd(C::Resign { side: Side::White })).reason,
        EndReason::Resignation
    );
    // refused in c-c, and for a computer side
    let mut r = Run::new(strict_cc());
    assert_eq!(
        denied(r.try_cmd(C::Resign { side: Side::White }).unwrap_err()),
        R::StateNotAllowed
    );
    let mut r = Run::new(strict_hc());
    assert_eq!(
        denied(r.try_cmd(C::Resign { side: Side::Black }).unwrap_err()),
        R::StateNotAllowed
    );
    assert_eq!(
        ended_of(&r.cmd(C::Resign { side: Side::White })).result,
        GameResult::BlackWins
    );
}

#[test]
fn ut_dom_041_game_4_abort_is_refused_in_human_games_and_reset_void_in_cc() {
    let mut r = Run::new(strict_hc());
    assert_eq!(denied(r.try_cmd(C::Abort).unwrap_err()), R::StateNotAllowed);
    let mut r = Run::new(strict_cc());
    let e = r
        .try_cmd(C::Void {
            reason: VoidReason::Reset,
            reset_player: None,
        })
        .unwrap_err();
    assert_eq!(denied(e), R::StateNotAllowed);
    let step = r.cmd(C::Void {
        reason: VoidReason::Unreadable,
        reset_player: None,
    });
    assert!(matches!(
        step.facts[..],
        [Fact::Voided {
            reason: VoidReason::Unreadable,
            reset_player: None
        }]
    ));
    assert_eq!(r.game.state(), S::Voided);
}

// ------------------------------------------------------------------ suspend and resume

#[test]
fn ut_dom_043_d_14_suspend_freezes_the_clocks_and_resume_restarts_them() {
    let mut r = Run::new(timed_hh());
    r.tick(3_000);
    let step = r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    assert!(
        matches!(
            step.facts[..],
            [
                Fact::Suspend {
                    cause: SuspendCause::Detach
                },
                Fact::Clock {
                    clocks: Some(ClockValues {
                        white_ms: 57_000,
                        black_ms: 60_000
                    }),
                    running: None
                }
            ]
        ),
        "{:?}",
        step.facts
    );
    assert_eq!(r.game.state(), S::Suspended);
    r.tick(3_600_000);
    assert_eq!(r.game.view(at(r.now)).clocks.white_ms, Some(57_000));
    let step = r.cmd(C::SuspendResume);
    assert!(matches!(step.facts[..], [Fact::SuspendResume]));
    assert_eq!(r.game.state(), S::AwaitingMove);
    r.tick(2_000);
    assert_eq!(r.game.view(at(r.now)).clocks.white_ms, Some(55_000));
    // no implicit resume: a move while suspended is denied
    r.cmd(C::Suspend {
        cause: SuspendCause::Shutdown,
    });
    assert_eq!(
        denied(r.try_cmd(r.move_cmd("e2e4", Actor::Human)).unwrap_err()),
        R::StateNotAllowed
    );
}

#[test]
fn ut_dom_043_d_14_offer_to_a_human_survives_a_suspend_and_one_to_a_computer_is_dropped() {
    let mut r = Run::new(untimed_hh());
    r.cmd(C::OfferDraw { by: Side::White });
    let step = r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    assert!(
        !step
            .facts
            .iter()
            .any(|f| matches!(f, Fact::DrawDropped { .. }))
    );
    r.cmd(C::SuspendResume);
    assert_eq!(
        (r.game.state(), r.game.pending_offer()),
        (S::DrawOffered, Some(Side::White))
    );

    let mut r = Run::new(training());
    r.cmd(C::OfferDraw { by: Side::White });
    let step = r.cmd(C::Suspend {
        cause: SuspendCause::Shutdown,
    });
    assert!(
        matches!(
            step.facts[..],
            [
                Fact::DrawDropped {
                    by: Side::White,
                    cause: DropCause::Suspend
                },
                Fact::Suspend {
                    cause: SuspendCause::Shutdown
                },
                Fact::Clock { .. }
            ]
        ),
        "{:?}",
        step.facts
    );
    r.cmd(C::SuspendResume);
    assert_eq!(
        (r.game.state(), r.game.pending_offer()),
        (S::AwaitingMove, None)
    );
}

#[test]
fn ut_dom_043_d_14_a_search_in_progress_resumes_as_awaiting_move() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    r.cmd(C::EngineStarted);
    r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    r.cmd(C::SuspendResume);
    assert_eq!(r.game.state(), S::AwaitingMove);
    // paused c-c games resume paused
    let mut r = Run::new(strict_cc());
    r.cmd(C::Pause);
    r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    r.cmd(C::SuspendResume);
    assert_eq!(r.game.state(), S::Paused);
    // a stepped search resumes paused too (the replay sees a Paused game)
    r.cmd(C::Step);
    r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    r.cmd(C::SuspendResume);
    assert_eq!(r.game.state(), S::Paused);
}

#[test]
fn ut_dom_116_sf_21_every_cause_is_carried_and_does_not_change_the_resume_target() {
    for cause in [
        SuspendCause::Detach,
        SuspendCause::Shutdown,
        SuspendCause::Recovered,
    ] {
        let mut r = Run::new(training());
        r.mv("e2e4");
        let step = r.cmd(C::Suspend { cause });
        assert!(matches!(&step.facts[0], Fact::Suspend { cause: c } if *c == cause));
        let out = r.replay();
        assert_eq!(out.error, None, "replay accepts {cause:?}");
        r.cmd(C::SuspendResume);
        assert_eq!(r.game.state(), S::AwaitingMove);
    }
}

#[test]
fn ut_dom_105_d_58_recovery_is_suspend_then_resume_and_matches_the_played_game() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    r.engine("e7e5");
    r.cmd(C::OfferDraw { by: Side::White });
    r.cmd(C::EngineFailed); // offer dropped
    r.mv("g1f3");
    // crash: the journal is the facts so far; the process restarts
    let out = Game::replay(items(&r.facts));
    assert!(out.interrupted && out.terminal.is_none() && out.error.is_none());
    let replayed = out.game.unwrap();
    let step = replayed
        .decide(
            C::Suspend {
                cause: SuspendCause::Recovered,
            },
            at(r.now),
        )
        .unwrap();
    assert!(matches!(
        step.facts[0],
        Fact::Suspend {
            cause: SuspendCause::Recovered
        }
    ));
    let step = step.game.decide(C::SuspendResume, at(r.now)).unwrap();
    assert!(step.game.journal_eq(&r.game));
}

// ------------------------------------------------------------------ terminal commands (T16..T19)

#[test]
fn ut_dom_120_t18_void_reset_needs_humans_and_replay_only_row_from_completed() {
    let mut r = Run::new(untimed_hh());
    let step = r.cmd(C::Void {
        reason: VoidReason::Reset,
        reset_player: Some(pid(1)),
    });
    assert!(
        matches!(step.facts[..], [Fact::Voided { reason: VoidReason::Reset, reset_player: Some(p) }] if p == pid(1))
    );
    assert_eq!(r.game.state(), S::Voided);
    assert_eq!(
        denied(r.try_cmd(C::Resign { side: Side::White }).unwrap_err()),
        R::Voided
    );

    // from Suspended
    let mut r = Run::new(untimed_hh());
    r.cmd(C::Suspend {
        cause: SuspendCause::Detach,
    });
    r.cmd(C::Void {
        reason: VoidReason::Unreadable,
        reset_player: None,
    });
    assert_eq!(r.game.state(), S::Voided);

    // decide(Void) from Completed is rejected; evolve(Voided) from Completed (replay) is accepted
    let mut r = Run::new(untimed_hh());
    r.cmd(C::Resign { side: Side::White });
    let e = r
        .try_cmd(C::Void {
            reason: VoidReason::Reset,
            reset_player: Some(pid(1)),
        })
        .unwrap_err();
    assert_eq!(denied(e), R::GameOver);
    let g = r
        .game
        .evolve(Fact::Voided {
            reason: VoidReason::Reset,
            reset_player: Some(pid(1)),
        })
        .unwrap();
    assert_eq!(g.state(), S::Voided);
}

#[test]
fn ut_dom_030_t19_mark_finalized_only_from_completed() {
    let mut r = Run::new(untimed_hh());
    assert_eq!(
        denied(r.try_cmd(C::MarkFinalized).unwrap_err()),
        R::StateNotAllowed
    );
    r.cmd(C::Resign { side: Side::White });
    let step = r.cmd(C::MarkFinalized);
    assert!(
        step.facts.is_empty(),
        "Finalized is a store event, not a journal record"
    );
    assert_eq!(r.game.state(), S::Finalized);
    assert_eq!(
        denied(r.try_cmd(C::MarkFinalized).unwrap_err()),
        R::GameOver
    );
    // Aborted and Voided cannot be finalized
    let mut r = Run::new(strict_cc());
    r.cmd(C::Abort);
    assert!(r.try_cmd(C::MarkFinalized).is_err());
}

#[test]
fn ut_dom_030_a_finished_game_denies_every_play_command() {
    let mut r = Run::new(untimed_hh());
    r.cmd(C::Resign { side: Side::White });
    for c in [
        C::Takeback,
        C::Pause,
        C::Step,
        C::EngineStarted,
        C::Flag,
        C::Abort,
        C::OfferDraw { by: Side::White },
        C::Suspend {
            cause: SuspendCause::Detach,
        },
        C::Checkpoint,
    ] {
        assert!(r.try_cmd(c.clone()).is_err(), "{c:?}");
    }
    assert_eq!(
        denied(r.try_cmd(r.move_cmd("e2e4", Actor::Human)).unwrap_err()),
        R::GameOver
    );
}

// ------------------------------------------------------------------ view

#[test]
fn ut_dom_046_view_is_consistent_with_the_state() {
    let mut r = Run::new(timed_hh());
    r.tick(1_000);
    r.mv("e2e4");
    let v = r.game.view(at(r.now));
    assert_eq!((v.state, v.ply, v.turn), (S::AwaitingMove, 1, Side::Black));
    assert_eq!(v.plies.len(), 1);
    assert_eq!(v.plies[0].san, "e4");
    assert_eq!(v.plies[0].uci(), "e2e4");
    assert_eq!(v.outcome, None);
    assert_eq!(v.clocks.as_of, at(r.now).wall);
    let ctx = r.game.context();
    assert_eq!(
        v.allowed_actions,
        chess_domain::Policy::allowed_actions(&ctx)
    );
    assert!(v.allowed_actions.contains(&ActionKind::MakeMove));
    assert!(v.allowed_actions.contains(&ActionKind::OfferDraw));
    assert!(!v.allowed_actions.contains(&ActionKind::Takeback));
    r.cmd(C::Resign { side: Side::White });
    let v = r.game.view(at(r.now));
    assert_eq!(v.outcome.unwrap().reason, EndReason::Resignation);
    assert!(v.allowed_actions.contains(&ActionKind::PostGameAnalysis));
    assert_eq!(v.clocks.running_side, None);
}

#[test]
fn ut_dom_046_checkpoint_command_emits_a_clock_fact_and_changes_nothing_else() {
    let mut r = Run::new(timed_hh());
    r.tick(2_000);
    let before = r.game.view(at(r.now));
    let step = r.cmd(C::Checkpoint);
    assert!(
        matches!(
            step.facts[..],
            [Fact::Clock {
                clocks: Some(ClockValues {
                    white_ms: 58_000,
                    black_ms: 60_000
                }),
                running: Some(Side::White)
            }]
        ),
        "{:?}",
        step.facts
    );
    assert_eq!(
        r.game.view(at(r.now)),
        before,
        "a checkpoint changes nothing the player can see"
    );
    assert_eq!(
        r.game.view(at(r.now)).clocks.running_side,
        Some(Side::White)
    );
}

#[test]
fn ut_dom_034_nfr_3_ply_limit_is_a_typed_error() {
    assert_eq!(chess_domain::MAX_PLIES, 10_000);
}

#[test]
fn ut_dom_030_position_accessor_matches_played_moves() {
    let mut r = Run::new(training());
    r.mv("e2e4");
    let p: &Position = r.game.position();
    assert_eq!(p.turn(), Side::Black);
}

#[test]
fn ut_dom_039_an_engine_failure_does_not_drop_an_offer_to_a_human() {
    let mut r = Run::new(untimed_hh());
    r.cmd(C::OfferDraw { by: Side::White });
    assert_eq!(
        denied(r.try_cmd(C::EngineFailed).unwrap_err()),
        R::StateNotAllowed
    );
    assert_eq!(
        (r.game.state(), r.game.pending_offer()),
        (S::DrawOffered, Some(Side::White))
    );
}
