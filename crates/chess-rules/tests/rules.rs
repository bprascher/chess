//! UT-RUL-004 (special-rule fixtures), UT-RUL-007 (termination predicates and material table),
//! UT-RUL-015 (illegal-move reasons), UT-RUL-006 (PositionKey). Requirements RULE-1, RULE-2,
//! RULE-4, NFR-9. Fixture rules are FIDE Laws (RULE-1); FENs are written for this suite.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{IllegalReason, Move, PieceKind, Position, Side};
use common::{START_FEN, mv, play_all, pos, sq, ucis};

fn has(p: &Position, uci: &str) -> bool {
    ucis(p).iter().any(|u| u == uci)
}

fn moves_from(p: &Position, from: &str) -> Vec<String> {
    ucis(p)
        .into_iter()
        .filter(|u| u.starts_with(from))
        .collect()
}

// ---------------------------------------------------------------- start position

#[test]
fn ut_rul_004_rule_1_start_position_has_20_moves_and_the_standard_fen() {
    let p = Position::standard();
    assert_eq!(p.legal_moves().len(), 20);
    assert_eq!(p.to_fen().as_str(), START_FEN);
    assert_eq!(p.turn(), Side::White);
    assert_eq!(p.fullmove_number(), 1);
    assert_eq!(p.halfmove_clock(), 0);
    let king = p.piece_at(sq("e1")).unwrap();
    assert_eq!((king.side, king.kind), (Side::White, PieceKind::King));
    assert_eq!(p.piece_at(sq("e4")), None);
}

// ---------------------------------------------------------------- castling

#[test]
fn ut_rul_004_rule_1_castling_both_sides_is_a_two_square_king_move() {
    let p = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    assert!(has(&p, "e1g1") && has(&p, "e1c1"));
    // never "king takes rook"
    assert!(!has(&p, "e1h1") && !has(&p, "e1a1"));
    let after = p.play(mv("e1g1")).unwrap();
    assert_eq!(
        after.to_fen().as_str(),
        "r3k2r/8/8/8/8/8/8/R4RK1 b kq - 1 1"
    );
    let after = p.play(mv("e1c1")).unwrap();
    assert_eq!(
        after.to_fen().as_str(),
        "r3k2r/8/8/8/8/8/8/2KR3R b kq - 1 1"
    );
    // black castles too
    let b = pos("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
    assert!(has(&b, "e8g8") && has(&b, "e8c8"));
}

#[test]
fn ut_rul_004_rule_1_castling_blocked_by_a_piece_is_absent() {
    let p = pos("r3k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1");
    assert!(!has(&p, "e1c1"));
    assert!(has(&p, "e1g1"));
    let q = pos("r3k2r/8/8/8/8/8/8/R3KB1R w KQkq - 0 1");
    assert!(!has(&q, "e1g1"));
    assert!(has(&q, "e1c1"));
}

#[test]
fn ut_rul_004_rule_1_castling_through_check_is_absent() {
    // black rook on f8 attacks f1: kingside path is attacked, queenside is free
    let p = pos("1k3r2/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert!(!has(&p, "e1g1"));
    assert!(has(&p, "e1c1"));
    // landing square attacked
    let q = pos("1k4r1/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert!(!has(&q, "e1g1"));
}

#[test]
fn ut_rul_004_rule_1_castling_out_of_check_is_absent() {
    let p = pos("1k2r3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert!(p.is_check());
    assert!(!has(&p, "e1g1") && !has(&p, "e1c1"));
}

#[test]
fn ut_rul_004_rule_1_castling_allowed_when_only_the_rook_path_square_b1_is_attacked() {
    // b1 is crossed by the rook, not the king: queenside castling stays legal
    let p = pos("1r1k4/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert!(has(&p, "e1c1"));
}

#[test]
fn ut_rul_004_rule_1_castling_rights_lost_after_king_or_rook_moves_or_rook_is_captured() {
    let p = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    // king moves away and back: rights gone
    let moved = play_all(&p, &["e1e2", "e8e7", "e2e1", "e7e8"]);
    assert!(!has(&moved, "e1g1") && !has(&moved, "e1c1"));
    assert_eq!(moved.to_fen().as_str(), "r3k2r/8/8/8/8/8/8/R3K2R w - - 4 3");
    // rook moves away and back: only that side is gone
    let rook = play_all(&p, &["h1h2", "e8e7", "h2h1", "e7e8"]);
    assert!(has(&rook, "e1c1") && !has(&rook, "e1g1"));
    // a rook captured on its home square removes the right of its owner
    let cap = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    let cap = play_all(&cap, &["a1a8"]);
    assert!(cap.to_fen().as_str().contains(" Kk "), "{}", cap.to_fen());
}

// ---------------------------------------------------------------- en passant

#[test]
fn ut_rul_004_rule_1_en_passant_capture_removes_the_pawn_and_ends_on_the_empty_square() {
    let p = pos("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 3");
    assert!(has(&p, "d4e3"));
    let after = p.play(mv("d4e3")).unwrap();
    assert_eq!(after.piece_at(sq("e4")), None, "captured pawn is gone");
    assert_eq!(after.piece_at(sq("d4")), None);
    assert_eq!(after.piece_at(sq("e3")).unwrap().kind, PieceKind::Pawn);
    assert_eq!(after.halfmove_clock(), 0);
}

#[test]
fn ut_rul_004_rule_1_en_passant_is_only_available_immediately_after_the_double_push() {
    let p = play_all(&Position::standard(), &["e2e4", "a7a6", "e4e5", "d7d5"]);
    assert!(has(&p, "e5d6"), "capture available right after d7d5");
    let later = play_all(&p, &["a2a3", "a6a5"]);
    assert!(!has(&later, "e5d6"), "right is gone one move later");
}

#[test]
fn ut_rul_004_rule_1_en_passant_that_exposes_the_king_along_the_rank_is_illegal() {
    // pinned-pawn case: exd6 would remove both pawns from the 5th rank and expose Ka5 to Rh5
    let p = pos("7k/8/8/K2pP2r/8/8/8/8 w - d6 0 1");
    assert_eq!(moves_from(&p, "e5"), vec!["e5e6".to_string()]);
}

#[test]
fn ut_rul_004_rule_1_en_passant_can_capture_a_checking_pawn() {
    // black pawn just pushed c7-c5 and gives check to Kd4 (a pawn on c5 attacks b4 and d4)
    let p = pos("4k3/8/8/2pP4/3K4/8/8/8 w - c6 0 1");
    // d5xc6 e.p. removes the checking pawn
    assert!(p.is_check());
    assert!(has(&p, "d5c6"));
}

// ---------------------------------------------------------------- promotion

#[test]
fn ut_rul_004_rule_1_promotion_to_each_piece_and_by_capture() {
    let p = pos("1n5k/P7/8/8/8/8/7K/8 w - - 0 1");
    let mut from_a7 = moves_from(&p, "a7");
    from_a7.sort();
    assert_eq!(
        from_a7,
        [
            "a7a8b", "a7a8n", "a7a8q", "a7a8r", "a7b8b", "a7b8n", "a7b8q", "a7b8r"
        ]
        .map(String::from)
        .to_vec()
    );
    let after = p.play(mv("a7b8n")).unwrap();
    let piece = after.piece_at(sq("b8")).unwrap();
    assert_eq!((piece.side, piece.kind), (Side::White, PieceKind::Knight));
    assert_eq!(after.piece_at(sq("a7")), None);
}

#[test]
fn ut_rul_004_rule_1_pawn_to_the_last_rank_without_a_piece_letter_is_illegal() {
    let p = pos("8/P6k/8/8/8/8/7K/8 w - - 0 1");
    let err = p.play(mv("a7a8")).unwrap_err();
    assert_eq!(err.reason, IllegalReason::PromotionRequired);
    assert!(!has(&p, "a7a8"));
}

#[test]
fn ut_rul_004_rule_1_black_promotes_on_rank_1() {
    let p = pos("7k/8/8/8/8/8/p7/7K b - - 0 1");
    for u in ["a2a1q", "a2a1r", "a2a1b", "a2a1n"] {
        assert!(has(&p, u), "{u}");
    }
}

// ---------------------------------------------------------------- pins, check

#[test]
fn ut_rul_004_rule_1_pinned_bishop_cannot_move_but_pinned_rook_slides_along_the_pin() {
    let bishop = pos("4r1k1/8/8/8/8/8/4B3/4K3 w - - 0 1");
    assert!(moves_from(&bishop, "e2").is_empty());
    assert_eq!(
        ucis(&bishop),
        ["e1d1", "e1d2", "e1f1", "e1f2"].map(String::from).to_vec()
    );
    let rook = pos("4r1k1/8/8/8/8/8/4R3/4K3 w - - 0 1");
    assert_eq!(
        moves_from(&rook, "e2"),
        ["e2e3", "e2e4", "e2e5", "e2e6", "e2e7", "e2e8"]
            .map(String::from)
            .to_vec()
    );
}

#[test]
fn ut_rul_004_rule_1_in_check_only_check_resolving_moves_are_legal() {
    // Ke1 checked by Re8; blocking with Bd... use a block square and a capture
    let p = pos("4r1k1/8/8/8/8/8/3N4/R3K3 w - - 0 1");
    assert!(p.is_check());
    // knight can block on e4 (d2-e4), king can step aside, rook a1 cannot help
    assert!(has(&p, "d2e4"));
    assert!(moves_from(&p, "a1").is_empty());
    assert!(!has(&p, "e1e2"), "stays on the checked file");
}

#[test]
fn ut_rul_004_rule_1_checkmate_fools_mate() {
    let p = play_all(&Position::standard(), &["f2f3", "e7e5", "g2g4", "d8h4"]);
    assert!(p.is_check() && p.is_checkmate() && !p.is_stalemate());
    assert!(p.legal_moves().is_empty());
}

#[test]
fn ut_rul_004_rule_1_stalemate_is_not_check_and_has_no_moves() {
    let p = pos("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
    assert!(p.is_stalemate());
    assert!(!p.is_check() && !p.is_checkmate());
    assert!(p.legal_moves().is_empty());
    assert!(!Position::standard().is_stalemate());
}

// ---------------------------------------------------------------- UT-RUL-007

#[test]
fn ut_rul_007_rule_1_material_table_matches_the_sdd_in_both_colours() {
    // (fen, white can mate, black can mate, dead)  SDD 3.11.3 table, colours mirrored below
    let rows: [(&str, bool, bool, bool); 10] = [
        // K v K
        ("4k3/8/8/8/8/8/8/4K3 w - - 0 1", false, false, true),
        // K+B v K
        ("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1", false, false, true),
        // K+N v K
        ("4k3/8/8/8/8/8/8/1N2K3 w - - 0 1", false, false, true),
        // K+B v K+B same colour (c1 dark, f8 dark)
        ("4kb2/8/8/8/8/8/8/2B1K3 w - - 0 1", false, false, true),
        // K+B v K+B opposite colours (c1 dark, c8 light)
        ("2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1", true, true, false),
        // K+N v K+N
        ("1n2k3/8/8/8/8/8/8/1N2K3 w - - 0 1", true, true, false),
        // K+NN v K
        ("4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1", true, false, false),
        // K+BB same colour (c1 dark, e3 dark... use c1 and a3: both dark) v K
        ("4k3/8/8/8/8/B7/8/2B1K3 w - - 0 1", false, false, true),
        // K+BB opposite colours (c1 dark, f1 light) v K
        ("4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1", true, false, false),
        // K+R v K
        ("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", true, false, false),
    ];
    for (fen, white, black, dead) in rows {
        let p = pos(fen);
        assert_eq!(p.can_side_checkmate(Side::White), white, "white: {fen}");
        assert_eq!(p.can_side_checkmate(Side::Black), black, "black: {fen}");
        assert_eq!(p.is_dead_by_material(), dead, "dead: {fen}");
    }
}

#[test]
fn ut_rul_007_rule_1_material_table_for_queen_and_pawn_and_mirrored_colours() {
    for fen in [
        "4k3/8/8/8/8/8/8/Q3K3 w - - 0 1",
        "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
    ] {
        let p = pos(fen);
        assert!(p.can_side_checkmate(Side::White), "{fen}");
        assert!(!p.can_side_checkmate(Side::Black), "{fen}");
        assert!(!p.is_dead_by_material(), "{fen}");
    }
    // mirrored: black has K+NN v K
    let p = pos("1n2kn2/8/8/8/8/8/8/4K3 w - - 0 1");
    assert!(!p.can_side_checkmate(Side::White) && p.can_side_checkmate(Side::Black));
    assert!(!p.is_dead_by_material());
    // mirrored: black has K+B+B opposite colours v K
    let p = pos("2b1kb2/8/8/8/8/8/8/4K3 w - - 0 1");
    assert!(!p.can_side_checkmate(Side::White) && p.can_side_checkmate(Side::Black));
    // mirrored: black K+B v K
    let p = pos("2b1k3/8/8/8/8/8/8/4K3 w - - 0 1");
    assert!(p.is_dead_by_material());
}

#[test]
fn ut_rul_007_rule_1_blocked_pawns_are_not_dead_by_material() {
    // RULE-1 (c): a blocked-pawn dead position is deliberately not detected
    let p = pos("k7/8/8/p7/P7/8/8/7K w - - 0 1");
    assert!(!p.is_dead_by_material());
    assert!(!p.is_stalemate() && !p.is_checkmate());
    // the pawns really are blocked: no pawn moves for either side
    assert!(moves_from(&p, "a4").is_empty());
    assert!(!p.legal_moves().is_empty());
}

#[test]
fn ut_rul_007_rule_2_checkmate_on_the_move_that_completes_the_fifty_move_count_is_still_mate() {
    // halfmove clock 99, White mates with Qa2-a8 (quiet move) which makes the clock 100
    let p = pos("7k/8/6K1/8/8/8/Q7/8 w - - 99 80");
    let after = p.play(mv("a2a8")).unwrap();
    assert_eq!(after.halfmove_clock(), 100);
    assert!(after.is_checkmate());
}

#[test]
fn ut_rul_007_rule_1_halfmove_clock_counts_quiet_moves_and_resets_on_pawn_move_or_capture() {
    let p = Position::standard();
    let p = play_all(&p, &["g1f3", "g8f6"]);
    assert_eq!(p.halfmove_clock(), 2);
    let p = play_all(&p, &["f3g1", "f6g8"]);
    assert_eq!(p.halfmove_clock(), 4);
    assert_eq!(p.fullmove_number(), 3);
    let after_pawn = p.play(mv("e2e4")).unwrap();
    assert_eq!(after_pawn.halfmove_clock(), 0);
    // capture resets too
    let cap = play_all(&Position::standard(), &["e2e4", "d7d5", "e4d5"]);
    assert_eq!(cap.halfmove_clock(), 0);
    // fifty-move boundary values stay exact
    let at_99 = pos("4k3/8/8/8/8/8/8/R3K3 w - - 99 60");
    assert_eq!(at_99.play(mv("a1a2")).unwrap().halfmove_clock(), 100);
}

// ---------------------------------------------------------------- UT-RUL-006 PositionKey

#[test]
fn ut_rul_006_rule_1_repetition_after_a_four_ply_shuffle_gives_equal_keys() {
    let start = Position::standard();
    let back = play_all(&start, &["g1f3", "g8f6", "f3g1", "f6g8"]);
    assert_eq!(start.repetition_key(), back.repetition_key());
    // counters are not part of the key
    assert_ne!(start.halfmove_clock(), back.halfmove_clock());
}

#[test]
fn ut_rul_006_rule_1_transposition_gives_equal_keys() {
    let a = play_all(&Position::standard(), &["e2e4", "e7e5", "g1f3", "b8c6"]);
    let b = play_all(&Position::standard(), &["g1f3", "b8c6", "e2e4", "e7e5"]);
    assert_eq!(a.repetition_key(), b.repetition_key());
}

#[test]
fn ut_rul_006_rule_1_en_passant_target_counts_only_when_a_legal_capture_exists() {
    // after 1.e4 no black pawn can capture: key equals the one for the same board with "-"
    let after_e4 = Position::standard().play(mv("e2e4")).unwrap();
    assert!(after_e4.to_fen().as_str().contains(" e3 "));
    let no_ep = pos("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    assert_eq!(after_e4.repetition_key(), no_ep.repetition_key());
    // with a black pawn on d4 the capture exists: keys differ
    let with_capturer = pos("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 3");
    let without = pos("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3");
    assert_ne!(with_capturer.repetition_key(), without.repetition_key());
    // a pinned capturer is not a legal capture: keys equal
    let pinned_a = pos("7k/8/8/K2pP2r/8/8/8/8 w - d6 0 1");
    let pinned_b = pos("7k/8/8/K2pP2r/8/8/8/8 w - - 0 1");
    assert_eq!(pinned_a.repetition_key(), pinned_b.repetition_key());
}

#[test]
fn ut_rul_006_rule_1_different_castling_rights_or_side_to_move_give_different_keys() {
    let all = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    let fewer = pos("r3k2r/8/8/8/8/8/8/R3K2R w Qkq - 0 1");
    assert_ne!(all.repetition_key(), fewer.repetition_key());
    let black = pos("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
    assert_ne!(all.repetition_key(), black.repetition_key());
    // same board reached after the king walked and returned: rights lost, key differs
    let walked = play_all(&all, &["e1e2", "e8e7", "e2e1", "e7e8"]);
    assert_ne!(all.repetition_key(), walked.repetition_key());
}

#[test]
fn ut_rul_006_dat_3_epd_key_is_the_first_four_fen_fields() {
    let key = pos(START_FEN).repetition_key();
    assert_eq!(
        key.to_epd(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"
    );
    // key ep is the legal one: after e4 there is no capture, so "-"
    let e4 = Position::standard().play(mv("e2e4")).unwrap();
    assert_eq!(
        e4.repetition_key().to_epd(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq -"
    );
    // with a capturer the square is shown
    let p = pos("rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 3");
    assert_eq!(
        p.repetition_key().to_epd(),
        "rnbqkbnr/ppp1pppp/8/8/3pP3/8/PPPP1PPP/RNBQKBNR b KQkq e3"
    );
}

#[test]
fn ut_rul_006_rule_1_keys_work_as_hash_set_members() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let start = Position::standard();
    assert!(seen.insert(start.repetition_key()));
    let moved = start.play(mv("g1f3")).unwrap();
    assert!(seen.insert(moved.repetition_key()));
    assert!(!seen.insert(start.repetition_key()));
}

// ---------------------------------------------------------------- UT-RUL-015 reasons

fn reason(fen: &str, uci: &str) -> IllegalReason {
    let p = pos(fen);
    let m: Move = mv(uci);
    p.play(m).unwrap_err().reason
}

#[test]
fn ut_rul_015_nfr_9_each_illegal_reason_has_a_fixture() {
    use IllegalReason as R;
    // no piece on the square
    assert_eq!(reason(START_FEN, "e4e5"), R::NoPiece);
    // wrong turn
    assert_eq!(reason(START_FEN, "e7e5"), R::WrongTurn);
    // geometry impossible: a knight cannot reach f4 from g1; a bishop cannot move straight
    assert_eq!(
        reason(START_FEN, "g1f4"),
        R::CannotReach {
            piece: PieceKind::Knight
        }
    );
    assert_eq!(
        reason("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1", "c1c3"),
        R::CannotReach {
            piece: PieceKind::Bishop
        }
    );
    assert_eq!(
        reason(START_FEN, "e2d3"),
        R::CannotReach {
            piece: PieceKind::Pawn
        }
    );
    // path blocked
    assert_eq!(reason(START_FEN, "a1a3"), R::Blocked);
    assert_eq!(reason(START_FEN, "c1e3"), R::Blocked);
    assert_eq!(
        reason("4k3/8/8/8/8/4p3/4P3/4K3 w - - 0 1", "e2e4"),
        R::Blocked
    );
    // own piece on the target
    assert_eq!(reason(START_FEN, "g1e2"), R::OwnPieceOnTarget);
    assert_eq!(reason(START_FEN, "d1d2"), R::OwnPieceOnTarget);
    // castling reasons
    assert_eq!(
        reason("r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1", "e1g1"),
        R::CastlingRightsLost
    );
    assert_eq!(
        reason("r3k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1", "e1c1"),
        R::CastlingBlocked
    );
    assert_eq!(
        reason("1k3r2/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1g1"),
        R::CastlingThroughCheck
    );
    assert_eq!(
        reason("1k2r3/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1g1"),
        R::CastlingOutOfCheck
    );
    // promotion required
    assert_eq!(
        reason("8/P6k/8/8/8/8/7K/8 w - - 0 1", "a7a8"),
        R::PromotionRequired
    );
    // pinned piece: bishop pinned on the e-file
    assert_eq!(
        reason("4r1k1/8/8/8/8/8/4B3/4K3 w - - 0 1", "e2d3"),
        R::LeavesKingInCheck
    );
    // moving into check
    assert_eq!(
        reason("4r1k1/8/8/8/8/8/8/4K3 w - - 0 1", "e1e2"),
        R::KingInCheck
    );
    assert_eq!(
        reason("5r1k/8/8/8/8/8/8/4K3 w - - 0 1", "e1f1"),
        R::LeavesKingInCheck
    );
    // not answering a check
    assert_eq!(
        reason("4r1k1/8/8/8/8/8/3N4/R3K3 w - - 0 1", "a1a2"),
        R::KingInCheck
    );
}

#[test]
fn ut_rul_015_nfr_9_black_castling_reasons_and_other_cases() {
    use IllegalReason as R;
    assert_eq!(
        reason("r3k2r/8/8/8/8/8/8/R3K2R b - - 0 1", "e8g8"),
        R::CastlingRightsLost
    );
    assert_eq!(
        reason("r2qk2r/8/8/8/8/8/8/R3K2R b kq - 0 1", "e8c8"),
        R::CastlingBlocked
    );
    // a promotion letter on a move that does not promote, or to a king or pawn: a wrong
    // request with its own reason, not `Other` (RF-06; was pinned to `Other` before rework)
    let p = Position::standard();
    assert_eq!(
        p.explain_illegal(sq("e2"), sq("e4"), Some(PieceKind::Queen)),
        R::NotAPromotion
    );
    let q = pos("8/P6k/8/8/8/8/7K/8 w - - 0 1");
    assert_eq!(
        q.explain_illegal(sq("a7"), sq("a8"), Some(PieceKind::King)),
        R::CannotReach {
            piece: PieceKind::King
        }
    );
    assert_eq!(
        q.explain_illegal(sq("a7"), sq("a8"), Some(PieceKind::Pawn)),
        R::CannotReach {
            piece: PieceKind::Pawn
        }
    );
    // a legal castling asked of the explainer is Other as well
    let c = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    assert_eq!(c.explain_illegal(sq("e1"), sq("g1"), None), R::Other);
    // black pawn geometry: double push from rank 7, blocked, and a diagonal onto an empty square
    let b = pos("4k3/4p3/8/8/8/8/8/4K3 b - - 0 1");
    assert_eq!(
        b.explain_illegal(sq("e7"), sq("e4"), None),
        R::CannotReach {
            piece: PieceKind::Pawn
        }
    );
    assert_eq!(
        b.explain_illegal(sq("e7"), sq("d6"), None),
        R::CannotReach {
            piece: PieceKind::Pawn
        }
    );
    assert_eq!(
        b.explain_illegal(sq("e7"), sq("e7"), None),
        R::CannotReach {
            piece: PieceKind::Pawn
        }
    );
    let blocked = pos("4k3/4p3/4P3/8/8/8/8/4K3 b - - 0 1");
    assert_eq!(
        blocked.explain_illegal(sq("e7"), sq("e5"), None),
        R::Blocked
    );
    assert_eq!(
        blocked.explain_illegal(sq("e7"), sq("e6"), None),
        R::Blocked
    );
    // king and queen shapes
    assert_eq!(
        p.explain_illegal(sq("e1"), sq("e3"), None),
        R::CannotReach {
            piece: PieceKind::King
        }
    );
    assert_eq!(
        p.explain_illegal(sq("d1"), sq("b2"), None),
        R::CannotReach {
            piece: PieceKind::Queen
        }
    );
    // pawn captures its own piece diagonally
    let own = pos("4k3/8/8/8/8/3N4/4P3/4K3 w - - 0 1");
    assert_eq!(
        own.explain_illegal(sq("e2"), sq("d3"), None),
        R::OwnPieceOnTarget
    );
}

#[test]
fn ut_rul_015_nfr_9_castling_shape_guards_and_queen_side_reasons() {
    use IllegalReason as R;
    // a two-file king move that is not castling is a geometry error, not a castling reason
    let king = |fen: &str, from: &str, to: &str| pos(fen).explain_illegal(sq(from), sq(to), None);
    let cannot = R::CannotReach {
        piece: PieceKind::King,
    };
    assert_eq!(king("4k3/8/8/8/8/8/8/3K4 w - - 0 1", "d1", "f1"), cannot); // not on the e-file
    assert_eq!(king("4k3/8/8/8/8/8/4K3/8 w - - 0 1", "e2", "g2"), cannot); // not the home rank
    assert_eq!(king("4k3/8/8/8/8/8/8/4K3 w - - 0 1", "e1", "g3"), cannot); // two files, two ranks
    assert_eq!(king("4k3/8/8/8/8/8/8/4K3 w - - 0 1", "e1", "e3"), cannot);
    // queen-side reasons use the queen-side squares
    assert_eq!(
        reason("1k1r4/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1c1"),
        R::CastlingThroughCheck
    );
    assert_eq!(
        reason("r3k3/8/8/8/8/8/8/R1B1K3 w Q - 0 1", "e1c1"),
        R::CastlingBlocked
    );
    assert_eq!(
        reason("r3k3/8/8/8/8/8/8/R3K3 w - - 0 1", "e1c1"),
        R::CastlingRightsLost
    );
    // pawn captures that geometry allows but a pin forbids (ordinary capture, en passant)
    assert_eq!(
        reason("4r1k1/8/8/8/8/3p4/4P3/4K3 w - - 0 1", "e2d3"),
        R::LeavesKingInCheck
    );
    assert_eq!(
        reason("7k/8/8/K2pP2r/8/8/8/8 w - d6 0 1", "e5d6"),
        R::LeavesKingInCheck
    );
    // a pawn cannot jump two files even onto an enemy piece
    assert_eq!(
        reason("4k3/8/8/8/8/2p5/P7/4K3 w - - 0 1", "a2c3"),
        R::CannotReach {
            piece: PieceKind::Pawn
        }
    );
    // a rook cannot move diagonally even when the diagonal is blocked
    assert_eq!(
        reason(START_FEN, "a1c3"),
        R::CannotReach {
            piece: PieceKind::Rook
        }
    );
}

#[test]
fn ut_rul_001_rule_1_side_other_and_piece_names() {
    assert_eq!(Side::White.other(), Side::Black);
    assert_eq!(Side::Black.other(), Side::White);
    let names: Vec<&str> = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ]
    .iter()
    .map(|k| k.name())
    .collect();
    assert_eq!(names, ["pawn", "knight", "bishop", "rook", "queen", "king"]);
}

#[test]
fn ut_rul_015_nfr_9_explain_illegal_on_a_legal_move_is_other() {
    // a defect indicator by SDD 3.11.2: never shown as a rule
    let p = Position::standard();
    assert_eq!(
        p.explain_illegal(sq("e2"), sq("e4"), None),
        IllegalReason::Other
    );
}

#[test]
fn ut_rul_015_nfr_9_play_never_changes_self_and_agrees_with_is_legal() {
    let p = pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let before = p.to_fen();
    let legal = p.legal_moves();
    for m in &legal {
        assert!(p.is_legal(*m));
        assert!(p.play(*m).is_ok());
    }
    assert_eq!(p.to_fen(), before);
    // every from/to pair (no promotion) that is not legal fails play and is_legal
    for from in 0..64u8 {
        for to in 0..64u8 {
            let m = Move {
                from: chess_rules::Square::from_index(from).unwrap(),
                to: chess_rules::Square::from_index(to).unwrap(),
                promotion: None,
            };
            assert_eq!(p.is_legal(m), legal.contains(&m));
            assert_eq!(p.play(m).is_ok(), legal.contains(&m));
        }
    }
}

// ---------------------------------------------------------------- UT-RUL-024, 025

#[test]
fn ut_rul_024_nfr_9_error_values_render_non_empty_messages_without_paths() {
    use chess_rules::{
        FenError, FenField, IllegalMove, IllegalPosition, MoveParseError, SquareError,
    };
    let reasons = [
        IllegalReason::NoPiece,
        IllegalReason::WrongTurn,
        IllegalReason::CannotReach {
            piece: PieceKind::Knight,
        },
        IllegalReason::Blocked,
        IllegalReason::OwnPieceOnTarget,
        IllegalReason::CastlingRightsLost,
        IllegalReason::CastlingBlocked,
        IllegalReason::CastlingOutOfCheck,
        IllegalReason::CastlingThroughCheck,
        IllegalReason::PromotionRequired,
        IllegalReason::NotAPromotion,
        IllegalReason::LeavesKingInCheck,
        IllegalReason::KingInCheck,
        IllegalReason::Other,
    ];
    let mut messages: Vec<String> = reasons.iter().map(ToString::to_string).collect();
    messages.push(
        IllegalMove {
            reason: IllegalReason::Blocked,
        }
        .to_string(),
    );
    messages.push(MoveParseError::BadUci.to_string());
    messages.push(SquareError::OutOfRange { value: 99 }.to_string());
    messages.push(SquareError::BadText.to_string());
    messages.push(FenError::TooLong { len: 200 }.to_string());
    messages.push(FenError::FieldCount { found: 3 }.to_string());
    messages.push(
        FenError::BadField {
            field: FenField::Castling,
            detail: "x",
        }
        .to_string(),
    );
    for illegal in [
        IllegalPosition::MissingKing,
        IllegalPosition::TooManyKings,
        IllegalPosition::PawnOnBackRank,
        IllegalPosition::OpponentInCheck,
        IllegalPosition::Other("too much material".to_string()),
    ] {
        messages.push(FenError::Illegal(illegal).to_string());
    }
    for m in &messages {
        assert!(!m.trim().is_empty());
        assert!(
            !m.contains('/') || m.contains("rank"),
            "no path-like text: {m}"
        );
        assert!(
            !m.contains(".rs") && !m.contains("src"),
            "no internal text: {m}"
        );
    }
    // the message of the knight example names the piece
    assert!(
        IllegalReason::CannotReach {
            piece: PieceKind::Knight
        }
        .to_string()
        .contains("knight")
    );
}

#[test]
fn ut_rul_025_architecture_4_8_public_types_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<chess_rules::File>();
    assert_send_sync::<chess_rules::Rank>();
    assert_send_sync::<chess_rules::Square>();
    assert_send_sync::<chess_rules::Side>();
    assert_send_sync::<chess_rules::PieceKind>();
    assert_send_sync::<chess_rules::Piece>();
    assert_send_sync::<Move>();
    assert_send_sync::<Position>();
    assert_send_sync::<chess_rules::PositionKey>();
    assert_send_sync::<chess_rules::Fen>();
    assert_send_sync::<chess_rules::FenError>();
    assert_send_sync::<chess_rules::IllegalMove>();
    assert_send_sync::<IllegalReason>();
    assert_send_sync::<chess_rules::MoveParseError>();
    assert_send_sync::<chess_rules::SquareError>();
}
