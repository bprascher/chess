//! UT-DOM-001, 121, 111-adjacent: identity newtypes and stable text forms. Written before the code.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]
mod common;
use chess_domain::{
    Category, EndReason, GameId, GameResult, GameStatus, Matchup, Mode, PlayerId, Track, Uuid,
    VoidReason,
};

#[test]
fn ut_dom_121_td_3_ids_are_uuid_newtypes_with_text_round_trip() {
    let u = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
    let g = GameId::from_uuid(u);
    let p = PlayerId::from_uuid(u);
    assert_eq!(g.as_uuid(), u);
    assert_eq!(p.as_uuid(), u);
    let text = g.to_string();
    assert_eq!(text, "01234567-89ab-cdef-0123-456789abcdef");
    assert_eq!(text.parse::<GameId>().unwrap(), g);
    assert_eq!(p.to_string().parse::<PlayerId>().unwrap(), p);
    for bad in [
        "",
        "1",
        "not-a-uuid",
        "01234567-89ab-cdef-0123-456789abcdeg",
        "12345",
    ] {
        assert!(bad.parse::<GameId>().is_err(), "{bad:?}");
    }
}

#[test]
fn ut_dom_121_ids_deserialise_from_uuid_text_and_reject_other_shapes() {
    use serde::Deserialize;
    use serde::de::IntoDeserializer;
    use serde::de::value::{Error, StrDeserializer, U64Deserializer};
    let d: StrDeserializer<'_, Error> = "00000000-0000-0000-0000-000000000007".into_deserializer();
    assert_eq!(GameId::deserialize(d).unwrap(), common::gid(7));
    let d: StrDeserializer<'_, Error> = "7".into_deserializer();
    assert!(GameId::deserialize(d).is_err());
    let d: U64Deserializer<Error> = 7u64.into_deserializer();
    assert!(
        GameId::deserialize(d).is_err(),
        "an integer row key never crosses the API"
    );
}

#[test]
fn ut_dom_001_dat_3_stable_text_forms_round_trip() {
    macro_rules! round_trip {
        ($ty:ty, [$(($v:expr, $t:literal)),+ $(,)?]) => {$(
            assert_eq!($v.as_str(), $t);
            assert_eq!($t.parse::<$ty>().unwrap(), $v);
            assert_eq!($v.to_string(), $t);
        )+};
    }
    round_trip!(
        Mode,
        [(Mode::Training, "training"), (Mode::Strict, "strict")]
    );
    round_trip!(
        Matchup,
        [
            (Matchup::HumanHuman, "hh"),
            (Matchup::HumanComputer, "hc"),
            (Matchup::ComputerComputer, "cc")
        ]
    );
    round_trip!(
        Track,
        [(Track::Strict, "strict"), (Track::Training, "training")]
    );
    round_trip!(
        GameStatus,
        [
            (GameStatus::InProgress, "in_progress"),
            (GameStatus::Completed, "completed"),
            (GameStatus::Aborted, "aborted"),
            (GameStatus::Voided, "voided"),
            (GameStatus::Imported, "imported"),
        ]
    );
    round_trip!(
        GameResult,
        [
            (GameResult::WhiteWins, "1-0"),
            (GameResult::BlackWins, "0-1"),
            (GameResult::Draw, "1/2-1/2")
        ]
    );
    round_trip!(
        Category,
        [
            (Category::Beginner, "Beginner"),
            (Category::Novice, "Novice"),
            (Category::Intermediate, "Intermediate"),
            (Category::Master, "Master"),
            (Category::GrandMaster, "Grand Master"),
        ]
    );
    for bad in ["", "Training", "TRAINING", "hh ", "1-0 "] {
        assert!(bad.parse::<Mode>().is_err() || bad == "training");
    }
    assert!("HH".parse::<Matchup>().is_err());
    assert_eq!(Track::from(Mode::Training), Track::Training);
    assert_eq!(Track::from(Mode::Strict), Track::Strict);
}

#[test]
fn ut_dom_121_sf_15_end_reason_journal_codes_round_trip() {
    let nine = [
        (EndReason::Checkmate, "checkmate"),
        (EndReason::Stalemate, "stalemate"),
        (EndReason::InsufficientMaterial, "insufficient_material"),
        (EndReason::ThreefoldRepetition, "threefold_repetition"),
        (EndReason::FiftyMoveRule, "fifty_move_rule"),
        (EndReason::TimeForfeit, "time_forfeit"),
        (
            EndReason::TimeoutOpponentCannotMate,
            "timeout_opponent_cannot_mate",
        ),
        (EndReason::Resignation, "resignation"),
        (EndReason::DrawAgreed, "draw_agreed"),
    ];
    for (reason, code) in nine {
        assert_eq!(reason.journal_code(), Some(code));
        assert_eq!(EndReason::from_journal_code(code).unwrap(), reason);
    }
    for r in [
        EndReason::Aborted,
        EndReason::VoidedByReset,
        EndReason::VoidedUnreadable,
    ] {
        assert_eq!(r.journal_code(), None, "{r:?} is not a GameEnd reason");
    }
    for bad in [
        "aborted",
        "voided by reset",
        "voided: unreadable",
        "insufficient material",
        "time forfeit",
        "Checkmate",
        "",
    ] {
        assert!(EndReason::from_journal_code(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn ut_dom_001_sf_15_termination_text_and_pgn_termination() {
    let table = [
        (EndReason::Checkmate, "checkmate", "normal"),
        (EndReason::Stalemate, "stalemate", "normal"),
        (
            EndReason::InsufficientMaterial,
            "insufficient material",
            "normal",
        ),
        (
            EndReason::ThreefoldRepetition,
            "threefold repetition",
            "normal",
        ),
        (EndReason::FiftyMoveRule, "fifty-move rule", "normal"),
        (EndReason::TimeForfeit, "time forfeit", "time forfeit"),
        (
            EndReason::TimeoutOpponentCannotMate,
            "time forfeit, opponent cannot mate",
            "normal",
        ),
        (EndReason::Resignation, "resignation", "normal"),
        (EndReason::DrawAgreed, "draw agreed", "normal"),
        (EndReason::Aborted, "aborted", "abandoned"),
        (EndReason::VoidedByReset, "voided by reset", "abandoned"),
        (
            EndReason::VoidedUnreadable,
            "voided: unreadable",
            "abandoned",
        ),
    ];
    for (r, text, pgn) in table {
        assert_eq!(r.termination_text(), text);
        assert_eq!(r.pgn_termination(), pgn);
        assert_eq!(EndReason::from_termination_text(text).unwrap(), r);
    }
}

#[test]
fn ut_dom_121_sf_10_void_reason_journal_strings() {
    assert_eq!(VoidReason::Reset.journal_code(), "reset");
    assert_eq!(VoidReason::Unreadable.journal_code(), "unreadable");
    assert_eq!(
        VoidReason::from_journal_code("reset").unwrap(),
        VoidReason::Reset
    );
    assert_eq!(
        VoidReason::from_journal_code("unreadable").unwrap(),
        VoidReason::Unreadable
    );
    for bad in ["", "Reset", "voided by reset", "other"] {
        assert!(VoidReason::from_journal_code(bad).is_err());
    }
}

#[test]
fn ut_dom_001_game_result_winner() {
    use chess_rules::Side;
    assert_eq!(GameResult::WhiteWins.winner(), Some(Side::White));
    assert_eq!(GameResult::BlackWins.winner(), Some(Side::Black));
    assert_eq!(GameResult::Draw.winner(), None);
    assert_eq!(GameResult::win_for(Side::Black), GameResult::BlackWins);
}

#[test]
fn ut_dom_100_nfr_9_every_error_variant_renders_a_plain_message() {
    use chess_domain::{
        ClockSpecError, Denied, DeniedReason, EloError, GameError, IdError, MoveDelayError,
        ParseTextError, ReplayError, SetupError, SetupField, SetupReason,
    };
    use chess_rules::IllegalMove;
    let setup = SetupError {
        field: SetupField::StartFen,
        reason: SetupReason::StartPositionTerminal,
    };
    let illegal = IllegalMove {
        reason: chess_rules::IllegalReason::Other,
    };
    let mut messages: Vec<String> = vec![
        ParseTextError { kind: "mode" }.to_string(),
        IdError.to_string(),
        MoveDelayError.to_string(),
        EloError::RatingOverflow.to_string(),
        setup.to_string(),
    ];
    for e in [
        ClockSpecError::BaseOutOfRange,
        ClockSpecError::IncrementOutOfRange,
        ClockSpecError::BadText,
    ] {
        messages.push(e.to_string());
    }
    for reason in DeniedReason::ALL {
        messages.push(Denied { reason: *reason }.to_string());
        assert!(!reason.as_str().is_empty());
    }
    for e in [
        GameError::Denied(Denied {
            reason: DeniedReason::GameOver,
        }),
        GameError::StalePosition { live_ply: 3 },
        GameError::IllegalMove(illegal),
        GameError::NothingToTakeBack,
        GameError::FlagNotFallen,
        GameError::TooLong,
        GameError::InvalidTransition,
        GameError::Setup(setup),
    ] {
        messages.push(e.to_string());
    }
    for e in [
        ReplayError::NotStarted,
        ReplayError::SecondStart,
        ReplayError::BadSetup(setup),
        ReplayError::WrongState {
            state: chess_domain::GameState::Paused,
        },
        ReplayError::NotResumed,
        ReplayError::WrongPly {
            expected: 1,
            found: 2,
        },
        ReplayError::Illegal(illegal),
        ReplayError::FenMismatch { ply: 4 },
        ReplayError::BadTakeback,
        ReplayError::TooLong,
    ] {
        messages.push(e.to_string());
    }
    for field in [SetupField::Mode, SetupField::Clock, SetupField::Players] {
        assert!(!field.as_str().is_empty());
    }
    for m in &messages {
        assert!(!m.trim().is_empty(), "empty message");
        assert!(
            !m.contains('/') && !m.contains('\\'),
            "no path-like text: {m}"
        );
        assert!(!m.contains("e2e4"), "no move list: {m}");
    }
    let mut unique = messages.clone();
    unique.sort();
    unique.dedup();
    assert!(
        unique.len() > 30,
        "messages are distinct, got {}",
        unique.len()
    );
}
