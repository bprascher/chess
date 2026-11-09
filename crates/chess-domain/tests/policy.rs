//! UT-DOM-020 to 026, 106, 115 (setup part), 120-adjacent: the mode policy table, setup
//! validation, slot holders, player actions, colour assignment. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    Action, ActionKind, Actor, ColorChoice, DeniedReason as R, GameContext, GameSetup,
    GameState as S, Matchup, Mode, MoveDelay, PlayerAction, PlayerContext, PlayerKind, Policy,
    SetupField, SetupReason, StartPosition, assign_colors,
};
use chess_rules::{Fen, Side};
use common::*;

use PlayerKind::{Computer as C, Human as H};

/// The contexts a real game can be in: the matchup and the two seat kinds agree.
fn contexts() -> Vec<GameContext> {
    let mut out = Vec::new();
    let seats = [
        (Matchup::HumanHuman, H, H),
        (Matchup::HumanComputer, H, C),
        (Matchup::HumanComputer, C, H),
        (Matchup::ComputerComputer, C, C),
    ];
    for mode in [Mode::Training, Mode::Strict] {
        for (matchup, white_kind, black_kind) in seats {
            for state in S::ALL {
                for turn in [Side::White, Side::Black] {
                    for pending_offer in [None, Some(Side::White), Some(Side::Black)] {
                        for offer_blocked_for in [None, Some(Side::White), Some(Side::Black)] {
                            out.push(GameContext {
                                mode,
                                matchup,
                                state: *state,
                                turn,
                                white_kind,
                                black_kind,
                                pending_offer,
                                offer_blocked_for,
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

fn actions() -> Vec<Action> {
    let mut v = vec![
        Action::Takeback,
        Action::LiveAnalysis,
        Action::WhatIf,
        Action::PostGameAnalysis,
        Action::Rewind,
        Action::SelectClock,
        Action::Pause,
        Action::PlayResume,
        Action::Step,
        Action::SetMoveDelay,
        Action::Abort,
        Action::CancelEngine,
        Action::RetryEngine,
    ];
    for s in [Side::White, Side::Black] {
        v.push(Action::OfferDraw { by: s });
        v.push(Action::Resign { side: s });
        for a in [Actor::Human, Actor::Engine] {
            v.push(Action::RespondDraw { by: s, actor: a });
            v.push(Action::MakeMove { by: s, actor: a });
        }
    }
    v
}

fn kind_of(c: &GameContext, s: Side) -> PlayerKind {
    if s == Side::White {
        c.white_kind
    } else {
        c.black_kind
    }
}

/// The oracle: SDD 3.11.4 and 3.11.5 written out again, as one decision list per action. It is
/// deliberately not shared with the implementation.
fn oracle(a: &Action, c: &GameContext) -> Result<(), R> {
    let st = c.state;
    let finished = matches!(st, S::Completed | S::Finalized | S::Aborted | S::Voided);
    let analysis = matches!(a, Action::PostGameAnalysis | Action::WhatIf);
    if st == S::Voided && !analysis {
        return Err(R::Voided);
    }
    if finished && st != S::Voided && !analysis {
        return Err(R::GameOver);
    }
    let in_progress = matches!(
        st,
        S::AwaitingMove | S::EngineThinking | S::EngineError | S::DrawOffered | S::Paused
    );
    let human_to_move = kind_of(c, c.turn) == H;
    let cc = c.matchup == Matchup::ComputerComputer;
    let hc = c.matchup == Matchup::HumanComputer;
    let strict = c.mode == Mode::Strict;
    match a {
        Action::Takeback => {
            if strict {
                Err(R::StrictMode)
            } else if hc && matches!(st, S::AwaitingMove | S::EngineThinking | S::EngineError) {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::LiveAnalysis => {
            if strict {
                Err(R::StrictMode)
            } else if st == S::AwaitingMove && human_to_move {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::WhatIf => {
            if finished {
                Ok(())
            } else if strict {
                Err(R::StrictMode)
            } else if in_progress && human_to_move {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::PostGameAnalysis => {
            if finished {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::Rewind => {
            if !cc && in_progress {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::SelectClock => {
            if strict {
                Ok(())
            } else {
                Err(R::TrainingUntimed)
            }
        }
        Action::OfferDraw { by } => {
            if cc || kind_of(c, *by) == C {
                Err(R::DrawNotAvailable)
            } else if st == S::DrawOffered {
                Err(R::DrawOfferPending)
            } else if matches!(st, S::EngineThinking | S::EngineError) {
                Err(R::DrawEngineBusy)
            } else if st != S::AwaitingMove {
                Err(R::StateNotAllowed)
            } else if *by != c.turn {
                Err(R::DrawNotOwnTurn)
            } else if c.offer_blocked_for == Some(*by) {
                Err(R::DrawRepeatBlocked)
            } else {
                Ok(())
            }
        }
        Action::RespondDraw { by, actor } => {
            let offeree_ok = c.pending_offer.map(Side::other) == Some(*by);
            let actor_ok = match actor {
                Actor::Human => kind_of(c, *by) == H,
                Actor::Engine => kind_of(c, *by) == C,
            };
            if st == S::DrawOffered && offeree_ok && actor_ok {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::Pause => {
            if cc && matches!(st, S::AwaitingMove | S::EngineThinking) {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::PlayResume | Action::Step => {
            if cc && st == S::Paused {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::SetMoveDelay => {
            if cc {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::Resign { side } => {
            if !cc && kind_of(c, *side) == H && (in_progress || st == S::Suspended) {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::Abort => {
            if cc && (in_progress || st == S::Suspended) {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::CancelEngine => {
            if hc && st == S::EngineThinking {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::RetryEngine => {
            if st == S::EngineError {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
        Action::MakeMove {
            by,
            actor: Actor::Human,
        } => {
            if kind_of(c, *by) != H || *by != c.turn {
                Err(R::NotHumanTurn)
            } else if st != S::AwaitingMove {
                Err(R::StateNotAllowed)
            } else {
                Ok(())
            }
        }
        Action::MakeMove {
            by,
            actor: Actor::Engine,
        } => {
            if kind_of(c, *by) == C && *by == c.turn && st == S::EngineThinking {
                Ok(())
            } else {
                Err(R::StateNotAllowed)
            }
        }
    }
}

#[test]
fn ut_dom_020_ply_5_full_product_equals_the_table_oracle() {
    let (mut n, mut allowed) = (0u32, 0u32);
    for c in contexts() {
        for a in actions() {
            let got = Policy::check(a, &c).map_err(|d| d.reason);
            let want = oracle(&a, &c);
            assert_eq!(got, want, "{a:?} in {c:?}");
            n += 1;
            allowed += u32::from(want.is_ok());
        }
    }
    assert!(
        n > 30_000 && allowed > 1_000,
        "the product is non-trivial: {n} cases, {allowed} allowed"
    );
}

fn kind_matches(a: &Action, k: ActionKind) -> bool {
    a.kind() == k
}

#[test]
fn ut_dom_021_ply_5_allowed_actions_agree_with_check_for_every_context() {
    for c in contexts() {
        let listed = Policy::allowed_actions(&c);
        let mut seen = Vec::new();
        for a in actions() {
            if Policy::check(a, &c).is_ok() && !seen.contains(&a.kind()) {
                seen.push(a.kind());
            }
        }
        for k in &listed {
            assert!(
                actions()
                    .iter()
                    .any(|a| kind_matches(a, *k) && Policy::check(*a, &c).is_ok()),
                "{k:?} listed but denied in {c:?}"
            );
        }
        for k in &seen {
            assert!(
                listed.contains(k),
                "{k:?} allowed by check but not listed in {c:?}"
            );
        }
        let mut dedup = listed.clone();
        dedup.dedup();
        assert_eq!(dedup, listed, "no duplicates");
        let order: Vec<usize> = listed
            .iter()
            .map(|k| ActionKind::ALL.iter().position(|x| x == k).unwrap())
            .collect();
        assert!(
            order.windows(2).all(|w| w[0] < w[1]),
            "fixed declaration order"
        );
    }
}

fn ctx(mode: Mode, matchup: Matchup, state: S, turn: Side) -> GameContext {
    let (white_kind, black_kind) = match matchup {
        Matchup::HumanHuman => (H, H),
        Matchup::HumanComputer => (H, C),
        Matchup::ComputerComputer => (C, C),
    };
    GameContext {
        mode,
        matchup,
        state,
        turn,
        white_kind,
        black_kind,
        pending_offer: None,
        offer_blocked_for: None,
    }
}

fn denied(a: Action, c: &GameContext) -> Option<R> {
    Policy::check(a, c).err().map(|d| d.reason)
}

#[test]
fn ut_dom_022_ply_5_strict_has_no_live_analysis_what_if_or_takeback_until_the_game_ends() {
    let c = ctx(
        Mode::Strict,
        Matchup::HumanComputer,
        S::AwaitingMove,
        Side::White,
    );
    assert_eq!(denied(Action::LiveAnalysis, &c), Some(R::StrictMode));
    assert_eq!(denied(Action::WhatIf, &c), Some(R::StrictMode));
    assert_eq!(denied(Action::Takeback, &c), Some(R::StrictMode));
    let done = ctx(
        Mode::Strict,
        Matchup::HumanComputer,
        S::Completed,
        Side::White,
    );
    assert_eq!(denied(Action::WhatIf, &done), None);
    assert_eq!(denied(Action::PostGameAnalysis, &done), None);
    assert_eq!(denied(Action::LiveAnalysis, &done), Some(R::GameOver));
    let t = ctx(
        Mode::Training,
        Matchup::HumanComputer,
        S::AwaitingMove,
        Side::White,
    );
    assert_eq!(denied(Action::LiveAnalysis, &t), None);
    assert_eq!(denied(Action::WhatIf, &t), None);
    let engine_turn = ctx(
        Mode::Training,
        Matchup::HumanComputer,
        S::AwaitingMove,
        Side::Black,
    );
    assert_eq!(
        denied(Action::LiveAnalysis, &engine_turn),
        Some(R::StateNotAllowed),
        "human's turn only"
    );
    assert_eq!(
        denied(Action::WhatIf, &engine_turn),
        Some(R::StateNotAllowed)
    );
}

#[test]
fn ut_dom_023_rule_2_the_five_draw_reasons_in_check_order() {
    let offer = |by| Action::OfferDraw { by };
    let mut c = ctx(
        Mode::Strict,
        Matchup::HumanHuman,
        S::AwaitingMove,
        Side::White,
    );
    assert_eq!(denied(offer(Side::White), &c), None);
    assert_eq!(denied(offer(Side::Black), &c), Some(R::DrawNotOwnTurn));
    c.offer_blocked_for = Some(Side::White);
    assert_eq!(denied(offer(Side::White), &c), Some(R::DrawRepeatBlocked));
    assert_eq!(
        denied(offer(Side::Black), &c),
        Some(R::DrawNotOwnTurn),
        "own-turn is checked before the block"
    );
    c.offer_blocked_for = None;
    c.state = S::DrawOffered;
    assert_eq!(denied(offer(Side::White), &c), Some(R::DrawOfferPending));
    let mut hc = ctx(
        Mode::Training,
        Matchup::HumanComputer,
        S::EngineThinking,
        Side::Black,
    );
    assert_eq!(denied(offer(Side::White), &hc), Some(R::DrawEngineBusy));
    assert_eq!(
        denied(offer(Side::Black), &hc),
        Some(R::DrawNotAvailable),
        "a computer cannot offer"
    );
    hc.state = S::Paused;
    assert_eq!(denied(offer(Side::White), &hc), Some(R::StateNotAllowed));
    let cc = ctx(
        Mode::Strict,
        Matchup::ComputerComputer,
        S::AwaitingMove,
        Side::White,
    );
    assert_eq!(denied(offer(Side::White), &cc), Some(R::DrawNotAvailable));
}

#[test]
fn ut_dom_024_eng_5_cancel_engine_only_while_thinking_in_a_human_computer_game() {
    for mode in [Mode::Training, Mode::Strict] {
        let c = ctx(mode, Matchup::HumanComputer, S::EngineThinking, Side::Black);
        assert_eq!(denied(Action::CancelEngine, &c), None);
        for s in S::ALL
            .iter()
            .filter(|s| **s != S::EngineThinking && !s.is_finished())
        {
            let c = ctx(mode, Matchup::HumanComputer, *s, Side::Black);
            assert_eq!(
                denied(Action::CancelEngine, &c),
                Some(R::StateNotAllowed),
                "{s:?}"
            );
        }
    }
    let cc = ctx(
        Mode::Strict,
        Matchup::ComputerComputer,
        S::EngineThinking,
        Side::White,
    );
    assert_eq!(denied(Action::CancelEngine, &cc), Some(R::StateNotAllowed));
}

#[test]
fn ut_dom_024_d_30_voided_and_finished_games_deny_everything_but_analysis() {
    for c in contexts().into_iter().filter(|c| c.state.is_finished()) {
        for a in actions() {
            let analysis = matches!(a, Action::PostGameAnalysis | Action::WhatIf);
            let got = denied(a, &c);
            if analysis {
                assert_eq!(got, None, "{a:?} {c:?}");
            } else if c.state == S::Voided {
                assert_eq!(got, Some(R::Voided));
            } else {
                assert_eq!(got, Some(R::GameOver));
            }
        }
    }
}

#[test]
fn ut_dom_025_check_setup_table() {
    let ok = |s: &GameSetup| Policy::check_setup(s).is_ok();
    assert!(ok(&training()) && ok(&strict_hh()) && ok(&strict_hc()) && ok(&strict_cc()));

    let err = |s: &GameSetup| Policy::check_setup(s).unwrap_err();
    let mut s = training();
    s.matchup = Matchup::HumanHuman;
    s.white = human(1);
    s.black = human(3);
    let e = err(&s);
    assert_eq!(
        (e.field, e.reason),
        (
            SetupField::Mode,
            SetupReason::TrainingRequiresHumanVsComputer
        )
    );
    let mut s = training();
    s.matchup = Matchup::ComputerComputer;
    s.white = computer(2);
    s.black = computer(4);
    assert_eq!(err(&s).reason, SetupReason::TrainingRequiresHumanVsComputer);

    let mut s = training();
    s.clock = Some(spec(5, 0));
    let e = err(&s);
    assert_eq!(
        (e.field, e.reason),
        (SetupField::Clock, SetupReason::TrainingUntimed)
    );

    // seat kinds that do not match the matchup
    let mut s = strict_hh();
    s.black = computer(2);
    let e = err(&s);
    assert_eq!(
        (e.field, e.reason),
        (SetupField::Players, SetupReason::PlayersWrongKind)
    );
    let mut s = strict_hc();
    s.black = human(3);
    assert_eq!(
        err(&s).reason,
        SetupReason::PlayersWrongKind,
        "h-c needs one computer"
    );
    s.white = computer(2);
    s.black = computer(5);
    assert_eq!(err(&s).reason, SetupReason::PlayersWrongKind);
    let mut s = strict_cc();
    s.white = human(1);
    assert_eq!(err(&s).reason, SetupReason::PlayersWrongKind);
    // h-c with the computer playing White is fine
    let mut s = strict_hc();
    s.white = computer(2);
    s.black = human(1);
    assert!(ok(&s));
}

#[test]
fn ut_dom_025_game_4_terminal_start_position_is_refused_for_a_new_game() {
    // checkmate (fool's mate), stalemate, and a dead-but-playable start
    for fen in [
        "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
        "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
    ] {
        let s = with_start(strict_hh(), fen);
        let e = Policy::check_setup(&s).unwrap_err();
        assert_eq!(
            (e.field, e.reason),
            (SetupField::StartFen, SetupReason::StartPositionTerminal),
            "{fen}"
        );
    }
    assert!(Policy::check_setup(&with_start(strict_hh(), "7k/8/8/8/8/8/8/KR6 w - - 0 1")).is_ok());
}

#[test]
fn ut_dom_025_mode_error_is_reported_before_the_clock_error() {
    let mut s = training();
    s.matchup = Matchup::HumanHuman;
    s.white = human(1);
    s.black = human(3);
    s.clock = Some(spec(5, 0));
    assert_eq!(Policy::check_setup(&s).unwrap_err().field, SetupField::Mode);
}

#[test]
fn ut_dom_026_d_27_slot_holders() {
    // h-h: two humans
    assert_eq!(
        Policy::slot_holders(&strict_hh()),
        vec![(pid(1), Mode::Strict), (pid(3), Mode::Strict)]
    );
    // h-h self-play: one slot
    let mut s = strict_hh();
    s.black = human(1);
    assert_eq!(Policy::slot_holders(&s), vec![(pid(1), Mode::Strict)]);
    // h-c: the human only, in the game's mode
    assert_eq!(
        Policy::slot_holders(&strict_hc()),
        vec![(pid(1), Mode::Strict)]
    );
    assert_eq!(
        Policy::slot_holders(&training()),
        vec![(pid(1), Mode::Training)]
    );
    let mut s = strict_hc();
    s.white = computer(2);
    s.black = human(9);
    assert_eq!(Policy::slot_holders(&s), vec![(pid(9), Mode::Strict)]);
    // c-c: two computers, or one for self-play
    assert_eq!(
        Policy::slot_holders(&strict_cc()),
        vec![(pid(4), Mode::Strict), (pid(5), Mode::Strict)]
    );
    let mut s = strict_cc();
    s.black = computer(4);
    assert_eq!(Policy::slot_holders(&s), vec![(pid(4), Mode::Strict)]);
}

#[test]
fn ut_dom_106_plr_4_check_player_action() {
    let pc = |kind, active, archived| PlayerContext {
        kind,
        active,
        archived,
    };
    let denied = |a, c| Policy::check_player_action(a, &c).err().map(|d| d.reason);
    for a in [
        PlayerAction::Rename,
        PlayerAction::Archive,
        PlayerAction::Reset,
    ] {
        assert_eq!(
            denied(a, pc(C, false, false)),
            Some(R::ComputerPlayer),
            "{a:?}"
        );
        assert_eq!(
            denied(a, pc(C, true, true)),
            Some(R::ComputerPlayer),
            "computer is checked first"
        );
    }
    assert_eq!(denied(PlayerAction::UseAsSide, pc(C, false, false)), None);
    assert_eq!(
        denied(PlayerAction::Archive, pc(H, true, false)),
        Some(R::ActivePlayer)
    );
    assert_eq!(denied(PlayerAction::Archive, pc(H, false, false)), None);
    assert_eq!(denied(PlayerAction::Rename, pc(H, true, false)), None);
    assert_eq!(denied(PlayerAction::Reset, pc(H, true, true)), None);
    assert_eq!(
        denied(PlayerAction::UseAsSide, pc(H, false, true)),
        Some(R::ArchivedPlayer)
    );
    assert_eq!(denied(PlayerAction::UseAsSide, pc(H, true, false)), None);
    assert!(
        R::ALL.iter().all(|r| r.as_str() != "game_open"),
        "no game_open reason exists"
    );
}

#[test]
fn ut_dom_025_assign_colors() {
    let mut rng = ScriptedRng::new(vec![]);
    assert_eq!(assign_colors(ColorChoice::White, &mut rng), Side::White);
    assert_eq!(assign_colors(ColorChoice::Black, &mut rng), Side::Black);
    let mut low = ScriptedRng::new(vec![0]);
    assert_eq!(assign_colors(ColorChoice::Random, &mut low), Side::White);
    let mut high = ScriptedRng::new(vec![u64::MAX]);
    assert_eq!(assign_colors(ColorChoice::Random, &mut high), Side::Black);
}

#[test]
fn ut_dom_115_d_64_start_position_normalises_to_standard_only_on_all_six_fields() {
    let from = |t: &str| StartPosition::from_fen(Fen::parse(t).unwrap());
    assert_eq!(from(START), StartPosition::Standard);
    assert!(!StartPosition::Standard.is_custom());
    assert_eq!(
        from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
        StartPosition::Standard
    );
    // counters: spelled with leading zeros the parsed values are equal
    assert_eq!(
        from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 00 01"),
        StartPosition::Standard
    );
    // one field different each
    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 1 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 2",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Kkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBN1 w Qkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1",
    ] {
        let sp = from(fen);
        assert!(sp.is_custom(), "{fen}");
        assert_eq!(
            sp.fen().unwrap().as_str(),
            Fen::parse(fen).unwrap().as_str()
        );
    }
    assert_eq!(
        StartPosition::Standard.fen(),
        None,
        "start_fen is null for the standard start"
    );
}

#[test]
fn ut_dom_115_d_64_start_position_side_to_move_and_position() {
    assert_eq!(StartPosition::Standard.side_to_move(), Side::White);
    let sp = custom("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
    assert_eq!(sp.side_to_move(), Side::Black);
    assert_eq!(StartPosition::Standard.position().to_fen().as_str(), START);
}

#[test]
fn ut_dom_103_move_delay_limits() {
    assert_eq!(MoveDelay::default().millis(), 1000);
    for ms in [0, 500, 1000, 9500, 10_000] {
        assert_eq!(MoveDelay::new(ms).unwrap().millis(), ms);
    }
    for ms in [1, 499, 501, 10_500, 20_000, u32::MAX] {
        assert!(MoveDelay::new(ms).is_err(), "{ms}");
    }
}

#[test]
fn ut_dom_025_game_setup_is_self_play_and_serde_shape_rejects_unknown_fields() {
    let mut s = strict_hh();
    assert!(!s.is_self_play());
    s.black = human(1);
    assert!(s.is_self_play());
    // deserialisation of the received shapes (the JSON codec itself is the app's)
    use serde::Deserialize;
    use serde::de::IntoDeserializer;
    use serde::de::value::{Error, StrDeserializer};
    let d: StrDeserializer<'_, Error> = "300+0".into_deserializer();
    assert_eq!(chess_domain::ClockSpec::deserialize(d).unwrap(), spec(5, 0));
    let d: StrDeserializer<'_, Error> = "05+0".into_deserializer();
    assert!(chess_domain::ClockSpec::deserialize(d).is_err());
    let d: StrDeserializer<'_, Error> = "hc".into_deserializer();
    assert_eq!(Matchup::deserialize(d).unwrap(), Matchup::HumanComputer);
    let d: StrDeserializer<'_, Error> = "HC".into_deserializer();
    assert!(Matchup::deserialize(d).is_err());
}
