//! Part B notation tests: SAN formatting (UT-RUL-008), SAN/position round trip (UT-RUL-009),
//! the typed and PGN move-text parser (UT-RUL-010 to 014, UT-RUL-028) and the random-input test
//! FZ-RUL-001. Requirements CLI-3, GUI-2 (c), RULE-6, RULE-7, NFR-3, D-45.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{IllegalReason, Move, MoveParseError, ParseMode, PieceKind, Position};
use common::{START_FEN, mv, play_all, pos};
use proptest::prelude::*;

fn san(p: &Position, uci: &str) -> String {
    p.san(mv(uci)).unwrap().to_string()
}

fn typed(p: &Position, text: &str) -> Result<String, MoveParseError> {
    p.parse_move(text, ParseMode::Typed).map(|m| m.uci)
}

fn pgn(p: &Position, text: &str) -> Result<String, MoveParseError> {
    p.parse_move(text, ParseMode::Pgn).map(|m| m.uci)
}

fn no_match(r: Result<String, MoveParseError>) -> IllegalReason {
    match r {
        Err(MoveParseError::NoMatch { reason }) => reason,
        other => panic!("expected NoMatch, got {other:?}"),
    }
}

const KNIGHTS_BF: &str = "4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1";
const ROOKS_A: &str = "4k3/8/8/R7/8/8/8/R3K3 w - - 0 1";
const QUEENS: &str = "4k3/8/8/8/8/Q7/8/Q1Q4K w - - 0 1";
const CASTLE: &str = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
const PROMO: &str = "7k/4P3/8/8/8/8/8/4K3 w - - 0 1";
const BISHOP_AND_PAWN: &str = "4k3/8/8/B7/8/2p5/1P6/4K3 w - - 0 1";
const PAWN_ONLY: &str = "4k3/8/8/8/8/2p5/1P6/4K3 w - - 0 1";
const BISHOP_ONLY: &str = "4k3/8/8/B7/8/2p5/8/4K3 w - - 0 1";

// ------------------------------------------------------------------ UT-RUL-008

#[test]
fn ut_rul_008_cli_3_san_of_pawn_and_piece_moves_from_the_start() {
    let p = Position::standard();
    assert_eq!(san(&p, "e2e4"), "e4");
    assert_eq!(san(&p, "g1f3"), "Nf3");
    assert_eq!(san(&p, "b1c3"), "Nc3");
}

#[test]
fn ut_rul_008_cli_3_san_of_captures() {
    let p = play_all(&Position::standard(), &["e2e4", "d7d5"]);
    assert_eq!(san(&p, "e4d5"), "exd5");
    let q = pos("4k3/8/8/3p4/8/8/8/3QK3 w - - 0 1");
    assert_eq!(san(&q, "d1d5"), "Qxd5");
}

#[test]
fn ut_rul_008_cli_3_san_of_en_passant_is_a_pawn_capture() {
    let p = play_all(&Position::standard(), &["e2e4", "a7a6", "e4e5", "d7d5"]);
    assert_eq!(san(&p, "e5d6"), "exd6");
}

#[test]
fn ut_rul_008_cli_3_san_of_castling_uses_the_letter_o() {
    let p = pos(CASTLE);
    assert_eq!(san(&p, "e1g1"), "O-O");
    assert_eq!(san(&p, "e1c1"), "O-O-O");
    let b = pos("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
    assert_eq!(san(&b, "e8g8"), "O-O");
    assert_eq!(san(&b, "e8c8"), "O-O-O");
}

#[test]
fn ut_rul_008_cli_3_san_of_promotion() {
    let p = pos("8/P6k/8/8/8/8/8/K7 w - - 0 1");
    assert_eq!(san(&p, "a7a8q"), "a8=Q");
    assert_eq!(san(&p, "a7a8n"), "a8=N");
    assert_eq!(san(&p, "a7a8r"), "a8=R");
    assert_eq!(san(&p, "a7a8b"), "a8=B");
    let check = pos("7k/P7/8/8/8/8/8/K7 w - - 0 1");
    assert_eq!(san(&check, "a7a8q"), "a8=Q+");
}

#[test]
fn ut_rul_008_cli_3_san_disambiguates_by_file_then_rank_then_both() {
    let k = pos(KNIGHTS_BF);
    assert_eq!(san(&k, "b1d2"), "Nbd2");
    assert_eq!(san(&k, "f1d2"), "Nfd2");
    let r = pos(ROOKS_A);
    assert_eq!(san(&r, "a1a3"), "R1a3");
    assert_eq!(san(&r, "a5a3"), "R5a3");
    let q = pos(QUEENS);
    assert_eq!(san(&q, "a1b2"), "Qa1b2");
    assert_eq!(san(&q, "c1b2"), "Qcb2");
    assert_eq!(san(&q, "a3b2"), "Q3b2");
}

#[test]
fn ut_rul_008_cli_3_san_has_check_and_mate_suffixes() {
    let p = play_all(&Position::standard(), &["e2e4", "d7d6"]);
    assert_eq!(san(&p, "f1b5"), "Bb5+");
    let mate = play_all(&Position::standard(), &["f2f3", "e7e5", "g2g4"]);
    assert_eq!(san(&mate, "d8h4"), "Qh4#");
}

#[test]
fn ut_rul_008_rule_1_san_of_an_illegal_move_is_illegal_move_with_a_reason() {
    let p = Position::standard();
    let err = p.san(mv("e2e5")).unwrap_err();
    assert_eq!(
        err.reason,
        IllegalReason::CannotReach {
            piece: PieceKind::Pawn
        }
    );
}

// ------------------------------------------------------------------ UT-RUL-010

#[test]
fn ut_rul_010_cli_3_typed_parser_accepts_the_documented_valid_forms() {
    let p = Position::standard();
    for (text, uci) in [
        ("e4", "e2e4"),
        ("Nf3", "g1f3"),
        ("E4", "e2e4"),
        ("e2e4", "e2e4"),
        ("E2E4", "e2e4"),
        ("Nf3+", "g1f3"),
        ("  e4  ", "e2e4"),
        ("Ng1f3", "g1f3"),
    ] {
        assert_eq!(typed(&p, text).unwrap(), uci, "{text}");
    }
    let after = play_all(&p, &["e2e4", "d7d5"]);
    for text in ["exd5", "ed5", "xd5", "exd5+", "EXD5"] {
        let r = typed(&after, text);
        // "EXD5" has an upper-case x, which is not a capture sign.
        if text == "EXD5" {
            assert!(r.is_err(), "{text}");
        } else {
            assert_eq!(r.unwrap(), "e4d5", "{text}");
        }
    }
}

#[test]
fn ut_rul_010_cli_3_typed_parser_accepts_castling_in_three_spellings() {
    let p = pos(CASTLE);
    for text in ["O-O", "0-0", "O-O+"] {
        assert_eq!(typed(&p, text).unwrap(), "e1g1", "{text}");
    }
    for text in ["O-O-O", "0-0-0"] {
        assert_eq!(typed(&p, text).unwrap(), "e1c1", "{text}");
    }
    // the same in Pgn mode
    assert_eq!(pgn(&p, "0-0").unwrap(), "e1g1");
    assert_eq!(pgn(&p, "O-O-O").unwrap(), "e1c1");
}

#[test]
fn ut_rul_010_cli_3_typed_parser_accepts_promotion_spellings_and_optional_x() {
    let p = pos(PROMO);
    for text in ["e8=Q", "e8Q", "e8q", "e8=q", "e7e8Q", "e7e8=Q", "e7e8q"] {
        assert_eq!(typed(&p, text).unwrap(), "e7e8q", "{text}");
    }
    assert_eq!(typed(&p, "e8=N").unwrap(), "e7e8n");
    let k = pos(KNIGHTS_BF);
    assert_eq!(typed(&k, "Nbd2").unwrap(), "b1d2");
    assert_eq!(typed(&k, "Nfd2").unwrap(), "f1d2");
    let r = pos(ROOKS_A);
    assert_eq!(typed(&r, "R1a3").unwrap(), "a1a3");
    assert_eq!(typed(&r, "R5a3").unwrap(), "a5a3");
    let q = pos("4k3/8/8/3p4/8/8/8/3QK3 w - - 0 1");
    assert_eq!(typed(&q, "Qd5").unwrap(), "d1d5", "x is optional");
    assert_eq!(typed(&q, "Qxd5").unwrap(), "d1d5");
}

// ------------------------------------------------------------------ UT-RUL-011

#[test]
fn ut_rul_011_cli_3_typed_parser_rejects_bad_shapes_with_specific_errors() {
    let p = Position::standard();
    assert_eq!(typed(&p, ""), Err(MoveParseError::Empty));
    assert_eq!(typed(&p, "   \t"), Err(MoveParseError::Empty));
    assert_eq!(
        typed(&p, "Nf3Nf3Nf3Nf3Nf3Nf3"),
        Err(MoveParseError::TooLong)
    );
    assert_eq!(typed(&p, "e4e4e4e4e4e4e4e4e"), Err(MoveParseError::TooLong));
    for text in [
        "\u{e9}4", "zz", "e9", "Ng", "+", "!?", "e", "Nf3x", "o-o", "O-0", "e2-e4", "Pe4", "N", "x",
    ] {
        assert_eq!(typed(&p, text), Err(MoveParseError::Syntax), "{text}");
    }
    assert_eq!(
        typed(&p, "nf3"),
        Err(MoveParseError::LowercasePiece {
            suggestion: "Nf3".to_string()
        })
    );
    assert_eq!(
        typed(&p, "qd1"),
        Err(MoveParseError::LowercasePiece {
            suggestion: "Qd1".to_string()
        })
    );
}

#[test]
fn ut_rul_011_cli_3_typed_parser_reports_illegal_moves_as_no_match() {
    let p = Position::standard();
    assert_eq!(
        no_match(typed(&p, "Ne4")),
        IllegalReason::CannotReach {
            piece: PieceKind::Knight
        }
    );
    // A wrong capture sign: a knight can reach f3, but not as a capture.
    assert!(matches!(
        typed(&p, "Nxf3"),
        Err(MoveParseError::NoMatch { .. })
    ));
    // A promotion on a move that does not promote.
    assert!(matches!(
        typed(&p, "e4=Q"),
        Err(MoveParseError::NoMatch { .. })
    ));
    assert!(matches!(
        typed(&p, "e5"),
        Err(MoveParseError::NoMatch { .. })
    ));
}

#[test]
fn ut_rul_011_cli_3_pgn_mode_matches_typed_mode_except_the_documented_differences() {
    let p = Position::standard();
    assert_eq!(pgn(&p, "e4").unwrap(), "e2e4");
    assert_eq!(pgn(&p, ""), Err(MoveParseError::Empty));
    assert!(matches!(
        pgn(&p, "nf3"),
        Err(MoveParseError::LowercasePiece { .. })
    ));
    // The coordinate form with a dash or a promotion letter is a typed-input convenience.
    let promo = pos(PROMO);
    assert_eq!(
        pgn(&promo, "e8"),
        Err(MoveParseError::PromotionRequired {
            choices: vec![
                PieceKind::Queen,
                PieceKind::Rook,
                PieceKind::Bishop,
                PieceKind::Knight
            ]
        })
    );
}

// ------------------------------------------------------------------ UT-RUL-012

#[test]
fn ut_rul_012_cli_3_bishop_rule_lower_b_is_the_file_upper_b_is_the_bishop() {
    let p = pos(BISHOP_AND_PAWN);
    assert_eq!(typed(&p, "bxc3").unwrap(), "b2c3", "pawn");
    assert_eq!(typed(&p, "Bxc3").unwrap(), "a5c3", "bishop");
    assert_eq!(pgn(&p, "Bxc3").unwrap(), "a5c3");
    assert_eq!(pgn(&p, "bxc3").unwrap(), "b2c3");
}

#[test]
fn ut_rul_012_gui_2_c_upper_b_falls_back_to_the_pawn_in_typed_mode_only() {
    let p = pos(PAWN_ONLY);
    assert_eq!(typed(&p, "Bxc3").unwrap(), "b2c3");
    assert_eq!(typed(&p, "bxc3").unwrap(), "b2c3");
    assert_eq!(
        no_match(pgn(&p, "Bxc3")),
        IllegalReason::CannotReach {
            piece: PieceKind::Bishop
        },
        "an import cannot silently read an illegal bishop SAN as a pawn move"
    );
}

#[test]
fn ut_rul_012_cli_3_lower_b_is_never_a_bishop() {
    let p = pos(BISHOP_ONLY);
    assert!(matches!(
        typed(&p, "bxc3"),
        Err(MoveParseError::NoMatch { .. })
    ));
    assert_eq!(typed(&p, "Bxc3").unwrap(), "a5c3");
}

// ------------------------------------------------------------------ UT-RUL-013

#[test]
fn ut_rul_013_cli_3_ambiguous_lists_every_candidate_in_canonical_san() {
    let p = pos(KNIGHTS_BF);
    match typed(&p, "Nd2") {
        Err(MoveParseError::Ambiguous { candidates }) => {
            let names: Vec<&str> = candidates.iter().map(|c| c.as_str()).collect();
            assert_eq!(names, ["Nbd2", "Nfd2"]);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    let q = pos(QUEENS);
    match typed(&q, "Qb2") {
        Err(MoveParseError::Ambiguous { candidates }) => assert_eq!(candidates.len(), 3),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    // Ambiguity is also an error in Pgn mode.
    assert!(matches!(
        pgn(&p, "Nd2"),
        Err(MoveParseError::Ambiguous { .. })
    ));
}

#[test]
fn ut_rul_013_rule_1_no_match_carries_the_nearest_illegal_reason() {
    let pinned = pos("4r1k1/8/8/8/8/8/4N3/4K3 w - - 0 1");
    assert_eq!(
        no_match(typed(&pinned, "Nc3")),
        IllegalReason::LeavesKingInCheck
    );
    let start = Position::standard();
    assert_eq!(no_match(typed(&start, "Ra3")), IllegalReason::Blocked);
    assert_eq!(
        no_match(typed(&start, "Ne2")),
        IllegalReason::OwnPieceOnTarget
    );
    assert_eq!(
        no_match(typed(&start, "O-O")),
        IllegalReason::CastlingBlocked
    );
    let lost = pos("r3k2r/8/8/8/8/8/8/R3K2R w Qkq - 0 1");
    assert_eq!(
        no_match(typed(&lost, "O-O")),
        IllegalReason::CastlingRightsLost
    );
    let through = pos("3k1r2/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert_eq!(
        no_match(typed(&through, "O-O")),
        IllegalReason::CastlingThroughCheck
    );
    let out = pos("3kr3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert_eq!(
        no_match(typed(&out, "O-O")),
        IllegalReason::CastlingOutOfCheck
    );
    // A coordinate move from an empty square and from the wrong side.
    assert_eq!(no_match(typed(&start, "e4e5")), IllegalReason::NoPiece);
    assert_eq!(no_match(typed(&start, "e7e5")), IllegalReason::WrongTurn);
}

// ------------------------------------------------------------------ UT-RUL-014

#[test]
fn ut_rul_014_cli_3_coordinate_promotion_forms_and_promotion_required() {
    let p = pos(PROMO);
    assert_eq!(typed(&p, "e7e8n").unwrap(), "e7e8n");
    assert_eq!(typed(&p, "e7e8=R").unwrap(), "e7e8r");
    let choices = vec![
        PieceKind::Queen,
        PieceKind::Rook,
        PieceKind::Bishop,
        PieceKind::Knight,
    ];
    for text in ["e7e8", "e8", "e7e8+"] {
        assert_eq!(
            typed(&p, text),
            Err(MoveParseError::PromotionRequired {
                choices: choices.clone()
            }),
            "{text}"
        );
    }
    // A promotion letter that the move cannot have.
    assert!(matches!(
        typed(&p, "e7e8k"),
        Err(MoveParseError::Syntax) | Err(MoveParseError::NoMatch { .. })
    ));
}

// ------------------------------------------------------------------ UT-RUL-028

#[test]
fn ut_rul_028_d_45_matched_move_carries_uci_and_canonical_san_of_the_move() {
    let p = pos(KNIGHTS_BF);
    let m = p.parse_move("Nbd2", ParseMode::Typed).unwrap();
    assert_eq!(m.mv, mv("b1d2"));
    assert_eq!(m.uci, m.mv.to_uci());
    assert_eq!(m.san, p.san(m.mv).unwrap());
    // Input spelling is not kept: the SAN is computed from the move.
    let loose = p.parse_move("b1d2", ParseMode::Typed).unwrap();
    assert_eq!(loose.san.as_str(), "Nbd2");
    // An engine move string maps to the move the matcher gives.
    let engine = Move::from_uci("b1d2").unwrap();
    assert_eq!(engine, loose.mv);
}

// ------------------------------------------------------------------ UT-RUL-009, FZ-RUL-001

fn playout_positions(choices: &[u16]) -> Vec<Position> {
    let mut cur = Position::standard();
    let mut out = vec![cur.clone()];
    for c in choices {
        let legal = cur.legal_moves();
        if legal.is_empty() {
            break;
        }
        cur = cur.play(legal[usize::from(*c) % legal.len()]).unwrap();
        out.push(cur.clone());
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 150, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn ut_rul_009_cli_3_san_round_trips_through_the_pgn_mode_parser(
        choices in proptest::collection::vec(any::<u16>(), 0..120),
    ) {
        for position in playout_positions(&choices) {
            for m in position.legal_moves() {
                let s = position.san(m).unwrap();
                let back = position.parse_move(s.as_str(), ParseMode::Pgn);
                prop_assert_eq!(back.map(|x| x.mv), Ok(m), "san {}", s);
                let typed_back = position.parse_move(s.as_str(), ParseMode::Typed);
                prop_assert_eq!(typed_back.map(|x| x.mv), Ok(m), "typed san {}", s);
                let coord = position.parse_move(&m.to_uci(), ParseMode::Typed);
                prop_assert_eq!(coord.map(|x| x.mv), Ok(m), "uci {}", m.to_uci());
            }
        }
    }

    #[test]
    fn fz_rul_001_cli_3_parse_move_never_panics_and_any_ok_is_legal(
        choices in proptest::collection::vec(any::<u16>(), 0..60),
        text in prop_oneof![
            any::<String>(),
            "[ -~]{0,24}",
            "[a-hA-H1-8KQRBNOxX=+#!?0 -]{0,20}",
        ],
    ) {
        let position = playout_positions(&choices).pop().unwrap();
        for mode in [ParseMode::Typed, ParseMode::Pgn] {
            match position.parse_move(&text, mode) {
                Ok(m) => {
                    prop_assert!(position.legal_moves().contains(&m.mv));
                    prop_assert_eq!(m.uci, m.mv.to_uci());
                }
                Err(e) => prop_assert!(!e.to_string().is_empty()),
            }
        }
    }
}

#[test]
fn fz_rul_001_nfr_3_parse_move_survives_hostile_inputs() {
    let p = pos(START_FEN);
    let long = "N".repeat(100_000);
    let nul = "\0\0\0";
    let high = "\u{10ffff}\u{1f600}e4";
    for text in [
        long.as_str(),
        nul,
        high,
        "\u{0}e4",
        "e4\u{0}",
        "((((",
        "e4e4e4e4e4e4e4e4e4",
    ] {
        for mode in [ParseMode::Typed, ParseMode::Pgn] {
            assert!(p.parse_move(text, mode).is_err());
        }
    }
}

// ------------------------------------------------------------------ boundaries and hints

#[test]
fn ut_rul_011_cli_3_the_length_limit_is_16_bytes_after_trimming() {
    let p = Position::standard();
    let sixteen = "Nf3Nf3Nf3Nf3Nf3N"; // 16 bytes: not a move, but not too long
    assert_eq!(sixteen.len(), 16);
    assert_eq!(typed(&p, sixteen), Err(MoveParseError::Syntax));
    let padded = format!("  {sixteen}  ");
    assert_eq!(typed(&p, &padded), Err(MoveParseError::Syntax));
    let seventeen = format!("{sixteen}f");
    assert_eq!(typed(&p, &seventeen), Err(MoveParseError::TooLong));
    assert_eq!(pgn(&p, &seventeen), Err(MoveParseError::TooLong));
}

#[test]
fn ut_rul_011_cli_3_no_match_uses_the_origin_hint_of_the_text() {
    let p = Position::standard();
    // Both rooks are candidates for "Rx3" squares; the hint picks which one is explained.
    assert_eq!(no_match(typed(&p, "Raa3")), IllegalReason::Blocked);
    assert_eq!(
        no_match(typed(&p, "Rha3")),
        IllegalReason::CannotReach {
            piece: PieceKind::Rook
        }
    );
    assert_eq!(no_match(typed(&p, "R1a3")), IllegalReason::Blocked);
}

#[test]
fn ut_rul_014_cli_3_coordinate_promotion_to_a_square_the_pawn_cannot_reach_is_no_match() {
    let p = pos(PROMO);
    assert!(matches!(
        typed(&p, "e7d8q"),
        Err(MoveParseError::NoMatch { .. })
    ));
    assert!(matches!(
        typed(&p, "e7e8=K"),
        Err(MoveParseError::Syntax) | Err(MoveParseError::NoMatch { .. })
    ));
    // A promotion letter on a move that does not promote.
    let q = Position::standard();
    assert!(matches!(
        typed(&q, "e2e4q"),
        Err(MoveParseError::NoMatch { .. })
    ));
    assert!(matches!(
        typed(&q, "e4q"),
        Err(MoveParseError::NoMatch { .. })
    ));
}

#[test]
fn ut_rul_010_cli_3_a_pawn_push_needs_no_origin_and_a_capture_needs_x_or_a_file() {
    // Pawns c4, d3, e4 and a black pawn on d5: c4 and e4 can capture it; the d-pawn is blocked.
    let p = pos("4k3/8/8/3p4/2P1P3/3P4/8/4K3 w - - 0 1");
    assert_eq!(typed(&p, "cxd5").unwrap(), "c4d5");
    assert_eq!(typed(&p, "exd5").unwrap(), "e4d5");
    // "xd5" alone is ambiguous: two pawns can capture.
    match typed(&p, "xd5") {
        Err(MoveParseError::Ambiguous { candidates }) => assert_eq!(candidates.len(), 2),
        other => panic!("{other:?}"),
    }
    // "d5" without x and without a file is not a capture reading.
    assert!(matches!(
        typed(&p, "d5"),
        Err(MoveParseError::NoMatch { .. })
    ));
}

#[test]
fn ut_rul_012_cli_3_b2d5_is_a_bishop_move_when_a_bishop_matches_else_the_coordinate() {
    // Bishop on g2 and a pawn on b2: "B2d5" is the bishop (the property test found this).
    let p = pos("4k3/8/8/3p4/8/8/1P4B1/4K3 w - - 0 1");
    assert_eq!(typed(&p, "B2d5").unwrap(), "g2d5", "bishop reading");
    assert_eq!(typed(&p, "b2b4").unwrap(), "b2b4");
    // No bishop can do it: the coordinate reading b2 -> b3 stands.
    let q = pos("4k3/8/8/8/8/8/1P6/4K3 w - - 0 1");
    assert_eq!(typed(&q, "B2b3").unwrap(), "b2b3");
    assert_eq!(typed(&q, "B2B4").unwrap(), "b2b4");
}
