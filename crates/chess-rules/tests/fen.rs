//! UT-RUL-016 (FEN table), UT-RUL-027 (as_str, Display), FZ-RUL-002 (random input).
//! Requirements RULE-8, IF-3, XC-26.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{Fen, FenError, FenField, IllegalPosition, Position};
use common::{START_FEN, pos};
use proptest::prelude::*;

#[test]
fn ut_rul_016_if_3_valid_fens_parse_and_normalise() {
    let valid = [
        START_FEN,
        // Kiwipete, castling and all piece kinds.
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        // en passant square after a double push, both conventions of IF-3 accepted
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
        // capturable en passant
        "rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 3",
        // partial castling rights
        "r3k2r/8/8/8/8/8/8/R3K2R w Kq - 0 1",
        // large counters
        "4k3/8/8/8/8/8/8/4K3 w - - 99 120",
        // a checkmate and a stalemate are valid positions (3.11.6)
        "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
        "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
    ];
    for text in valid {
        let fen = Fen::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(fen.as_str(), text, "already normalised input is unchanged");
    }
}

#[test]
fn ut_rul_016_if_3_counters_are_normalised_to_numbers() {
    let fen = Fen::parse("4k3/8/8/8/8/8/8/4K3 w - - 007 0012").unwrap();
    assert_eq!(fen.as_str(), "4k3/8/8/8/8/8/8/4K3 w - - 7 12");
}

#[test]
fn ut_rul_016_if_3_wrong_field_count_names_the_count() {
    assert_eq!(Fen::parse(""), Err(FenError::FieldCount { found: 1 }));
    assert_eq!(
        Fen::parse("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"),
        Err(FenError::FieldCount { found: 4 })
    );
    assert_eq!(
        Fen::parse(&format!("{START_FEN} extra")),
        Err(FenError::FieldCount { found: 7 })
    );
    // upstream parsers accept fewer fields; this crate does not (3.11.6 step 2)
    assert!(matches!(
        Fen::parse("8/8/8/8/8/8/8/8 w"),
        Err(FenError::FieldCount { found: 2 })
    ));
    // double space makes an empty field, which is a count of 7
    assert!(Fen::parse("4k3/8/8/8/8/8/8/4K3  w - - 0 1").is_err());
}

#[test]
fn ut_rul_016_if_3_too_long_input_is_rejected_before_parsing() {
    let long = format!("{START_FEN}{}", " ".repeat(200));
    assert!(matches!(Fen::parse(&long), Err(FenError::TooLong { .. })));
    // exactly at the limit is not "too long" (it fails on the field count instead)
    let at_limit = "x".repeat(128);
    assert!(!matches!(
        Fen::parse(&at_limit),
        Err(FenError::TooLong { .. })
    ));
    let over = "x".repeat(129);
    assert!(matches!(Fen::parse(&over), Err(FenError::TooLong { .. })));
}

fn field_of(text: &str) -> FenField {
    match Fen::parse(text) {
        Err(FenError::BadField { field, .. }) => field,
        other => panic!("{text}: expected BadField, got {other:?}"),
    }
}

#[test]
fn ut_rul_016_if_3_each_field_syntax_error_names_its_field() {
    let cases = [
        // placement
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP w KQkq - 0 1",
            FenField::Placement,
        ), // 7 ranks
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR/8 w KQkq - 0 1",
            FenField::Placement,
        ), // 9 ranks
        (
            "rnbqkbn/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            FenField::Placement,
        ), // short rank
        (
            "rnbqkbnrr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            FenField::Placement,
        ), // long rank
        (
            "rnbqkbnx/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            FenField::Placement,
        ), // bad letter
        (
            "rnbqkbn1/pppppppp/9/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            FenField::Placement,
        ), // digit 9
        (
            "rnbqkbn0/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            FenField::Placement,
        ), // digit 0
        ("4k3/8/8/8/8/8/8/44 w - - 0 1", FenField::Placement), // adjacent digits
        ("4k3/8/8/8/8/8/8/4K3[Q] w - - 0 1", FenField::Placement), // pocket syntax
        // side
        ("4k3/8/8/8/8/8/8/4K3 x - - 0 1", FenField::SideToMove),
        ("4k3/8/8/8/8/8/8/4K3 W - - 0 1", FenField::SideToMove),
        ("4k3/8/8/8/8/8/8/4K3 wb - - 0 1", FenField::SideToMove),
        // castling
        ("4k3/8/8/8/8/8/8/4K3 w X - 0 1", FenField::Castling),
        ("4k3/8/8/8/8/8/8/4K3 w qk - 0 1", FenField::Castling), // wrong order
        ("4k3/8/8/8/8/8/8/4K3 w KK - 0 1", FenField::Castling), // repeated
        ("4k3/8/8/8/8/8/8/4K3 w -K - 0 1", FenField::Castling),
        // en passant
        ("4k3/8/8/8/8/8/8/4K3 w - e9 0 1", FenField::EnPassant),
        ("4k3/8/8/8/8/8/8/4K3 w - i3 0 1", FenField::EnPassant),
        ("4k3/8/8/8/8/8/8/4K3 w - e4 0 1", FenField::EnPassant), // wrong rank for any side
        ("4k3/8/8/8/8/8/8/4K3 w - e3 0 1", FenField::EnPassant), // rank 3 but White to move
        ("4k3/8/8/8/8/8/8/4K3 b - e6 0 1", FenField::EnPassant), // rank 6 but Black to move
        ("4k3/8/8/8/8/8/8/4K3 w - E6 0 1", FenField::EnPassant),
        // halfmove clock
        ("4k3/8/8/8/8/8/8/4K3 w - - -1 1", FenField::HalfmoveClock),
        ("4k3/8/8/8/8/8/8/4K3 w - - x 1", FenField::HalfmoveClock),
        ("4k3/8/8/8/8/8/8/4K3 w - - +1 1", FenField::HalfmoveClock),
        (
            "4k3/8/8/8/8/8/8/4K3 w - - 4294967296 1",
            FenField::HalfmoveClock,
        ),
        // fullmove number
        ("4k3/8/8/8/8/8/8/4K3 w - - 0 0", FenField::FullmoveNumber),
        ("4k3/8/8/8/8/8/8/4K3 w - - 0 -3", FenField::FullmoveNumber),
        ("4k3/8/8/8/8/8/8/4K3 w - - 0 y", FenField::FullmoveNumber),
        (
            "4k3/8/8/8/8/8/8/4K3 w - - 0 4294967296",
            FenField::FullmoveNumber,
        ),
    ];
    for (text, field) in cases {
        assert_eq!(field_of(text), field, "{text}");
    }
}

#[test]
fn ut_rul_016_if_3_illegal_positions_name_the_reason() {
    let illegal = |text: &str| match Fen::parse(text) {
        Err(FenError::Illegal(reason)) => reason,
        other => panic!("{text}: expected Illegal, got {other:?}"),
    };
    assert_eq!(
        illegal("8/8/8/8/8/8/8/4K3 w - - 0 1"),
        IllegalPosition::MissingKing
    );
    assert_eq!(
        illegal("4k3/8/8/8/8/8/8/8 w - - 0 1"),
        IllegalPosition::MissingKing
    );
    assert_eq!(
        illegal("8/8/8/8/8/8/8/8 w - - 0 1"),
        IllegalPosition::MissingKing
    );
    assert_eq!(
        illegal("4k3/8/8/8/8/8/8/3KK3 w - - 0 1"),
        IllegalPosition::TooManyKings
    );
    assert_eq!(
        illegal("4k1k1/8/8/8/8/8/8/4K3 w - - 0 1"),
        IllegalPosition::TooManyKings
    );
    assert_eq!(
        illegal("P3k3/8/8/8/8/8/8/4K3 w - - 0 1"),
        IllegalPosition::PawnOnBackRank
    );
    assert_eq!(
        illegal("4k3/8/8/8/8/8/8/p3K3 w - - 0 1"),
        IllegalPosition::PawnOnBackRank
    );
    // side not to move is in check: white rook gives check to the black king, white to move
    assert_eq!(
        illegal("4k3/8/8/8/8/8/8/4RK2 w - - 0 1"),
        IllegalPosition::OpponentInCheck
    );
    // castling right with no rook on the corner
    assert!(matches!(
        illegal("4k3/8/8/8/8/8/8/4K3 w K - 0 1"),
        IllegalPosition::Other(_)
    ));
    // en passant square with no pushed pawn behind it
    assert!(matches!(
        illegal("4k3/8/8/8/8/8/8/4K3 b - e3 0 1"),
        IllegalPosition::Other(_)
    ));
}

#[test]
fn ut_rul_016_if_3_material_and_check_reasons_that_have_no_dedicated_variant() {
    // sixteen queens cannot arise in a game
    match Fen::parse("7k/8/8/8/8/8/PPPPPPPP/QQQQKQQQ w - - 0 1") {
        Err(FenError::Illegal(IllegalPosition::Other(text))) => assert!(text.contains("material")),
        other => panic!("expected too much material, got {other:?}"),
    }
    // two knights checking the king at once cannot arise
    match Fen::parse("4k3/8/8/8/8/3n1n2/8/4K3 w - - 0 1") {
        Err(FenError::Illegal(IllegalPosition::Other(text))) => assert!(text.contains("check")),
        other => panic!("expected impossible check, got {other:?}"),
    }
}

#[test]
fn ut_rul_016_if_3_an_empty_counter_is_a_syntax_error_of_that_field() {
    // trailing space: the sixth field is empty
    match Fen::parse("4k3/8/8/8/8/8/8/4K3 w - - 0 ") {
        Err(FenError::BadField { field, detail }) => {
            assert_eq!(field, FenField::FullmoveNumber);
            assert!(detail.contains("whole number"), "{detail}");
        }
        other => panic!("expected BadField, got {other:?}"),
    }
}

#[test]
fn ut_rul_016_if_3_fen_values_hash_and_compare_on_their_text() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    assert!(set.insert(Fen::parse(START_FEN).unwrap()));
    assert!(!set.insert(Position::standard().to_fen()));
    assert_eq!(
        format!("{:?}", Position::standard().to_fen()),
        format!("Fen({START_FEN:?})")
    );
}

#[test]
fn ut_rul_016_if_3_every_error_message_names_the_problem() {
    for text in [
        "",
        "4k3/8/8/8/8/8/8/4K3 x - - 0 1",
        "4k3/8/8/8/8/8/8/3KK3 w - - 0 1",
    ] {
        let message = Fen::parse(text).unwrap_err().to_string();
        assert!(message.len() > 10, "{message:?}");
    }
    let message = Fen::parse("4k3/8/8/8/8/8/8/4K3 w - e9 0 1")
        .unwrap_err()
        .to_string();
    assert!(message.contains("en passant"), "{message}");
}

#[test]
fn ut_rul_027_xc_26_as_str_display_and_reparse_agree() {
    for text in [
        START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
    ] {
        let fen = Fen::parse(text).unwrap();
        assert_eq!(fen.as_str(), text);
        assert_eq!(fen.to_string(), text);
        assert_eq!(Fen::parse(fen.as_str()).unwrap(), fen);
    }
}

#[test]
fn ut_rul_027_if_3_either_en_passant_convention_is_accepted_and_kept_as_parsed() {
    // Observation for chess-2bb: the Fen type holds the normalised six fields. It does not
    // strip an en passant square for which no capture exists, so the two spellings of the
    // same board are two different Fen values.
    let with = Fen::parse("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
    let without = Fen::parse("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
    assert_ne!(with, without);
    // ...but they describe positions with the same repetition key.
    assert_eq!(
        pos(with.as_str()).repetition_key(),
        pos(without.as_str()).repetition_key()
    );
    // The position keeps the field it was given when it writes the FEN again.
    assert_eq!(pos(with.as_str()).to_fen(), with);
    assert_eq!(pos(without.as_str()).to_fen(), without);
}

#[test]
fn ut_rul_016_rule_8_standard_start_equals_only_with_all_six_fields() {
    // Needed by SDD step 5 (canonical_form, UT-RUL-031): equality of Fen is on all six fields.
    let standard = Position::standard().to_fen();
    assert_eq!(Fen::parse(START_FEN).unwrap(), standard);
    for variant in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 1 1", // halfmove
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 2", // fullmove
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qkq - 0 1",  // castling
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1", // side
    ] {
        assert_ne!(Fen::parse(variant).unwrap(), standard, "{variant}");
    }
    // other placement
    assert_ne!(
        Fen::parse("rnbqkbnr/pppppppp/8/8/8/2N5/PPPPPPPP/R1BQKBNR w KQkq - 0 1").unwrap(),
        standard
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]

    /// FZ-RUL-002 (random input, IF-3): arbitrary text never panics; any Ok round-trips.
    #[test]
    fn fz_rul_002_if_3_arbitrary_text_never_panics_and_ok_round_trips(text in ".{0,160}") {
        if let Ok(fen) = Fen::parse(&text) {
            prop_assert_eq!(Fen::parse(fen.as_str()).unwrap(), fen);
        }
    }

    /// FZ-RUL-002: a valid FEN with one character replaced or deleted never panics.
    #[test]
    fn fz_rul_002_if_3_mutated_valid_fens_never_panic(
        idx in 0usize..70,
        replacement in proptest::char::range(' ', '~'),
        delete in proptest::bool::ANY,
    ) {
        let base = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let mut chars: Vec<char> = base.chars().collect();
        let at = idx % chars.len();
        if delete { chars.remove(at); } else { chars[at] = replacement; }
        let mutated: String = chars.into_iter().collect();
        if let Ok(fen) = Fen::parse(&mutated) {
            prop_assert_eq!(Fen::parse(fen.as_str()).unwrap(), fen);
        }
    }
}
