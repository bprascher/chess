//! UT-RUL-005 (property: play/legal agree, FEN round trip, en passant square after every
//! double push). Requirements NFR-3, RULE-1. Random legal playouts via proptest.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{Fen, Move, PieceKind, Position, Side};
use proptest::prelude::*;

/// Plays up to `choices.len()` plies, picking the legal move `choice % len` each time.
/// Returns every position visited with the move played from it.
fn playout(choices: &[u16]) -> Vec<(Position, Move)> {
    let mut cur = Position::standard();
    let mut out = Vec::new();
    for c in choices {
        let legal = cur.legal_moves();
        if legal.is_empty() {
            break;
        }
        let m = legal[usize::from(*c) % legal.len()];
        let next = cur.play(m).unwrap();
        out.push((cur, m));
        cur = next;
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn ut_rul_005_nfr_3_play_succeeds_iff_the_move_is_legal_on_random_playouts(
        choices in proptest::collection::vec(any::<u16>(), 0..120),
        from in 0u8..64, to in 0u8..64,
    ) {
        for (position, _) in playout(&choices) {
            let legal = position.legal_moves();
            for m in &legal {
                prop_assert!(position.play(*m).is_ok());
            }
            let probe = Move {
                from: chess_rules::Square::from_index(from).unwrap(),
                to: chess_rules::Square::from_index(to).unwrap(),
                promotion: None,
            };
            prop_assert_eq!(position.play(probe).is_ok(), legal.contains(&probe));
            prop_assert_eq!(position.is_legal(probe), legal.contains(&probe));
        }
    }

    #[test]
    fn ut_rul_005_rule_1_fen_round_trips_and_the_position_after_play_is_valid(
        choices in proptest::collection::vec(any::<u16>(), 0..120),
    ) {
        for (position, m) in playout(&choices) {
            let after = position.play(m).unwrap();
            let fen = after.to_fen();
            let reparsed = Fen::parse(fen.as_str()).unwrap();
            prop_assert_eq!(&reparsed, &fen);
            let again = Position::from_fen(&reparsed);
            prop_assert_eq!(again.to_fen(), fen);
            prop_assert_eq!(again.repetition_key(), after.repetition_key());
            let mut a = again.legal_moves();
            let mut b = after.legal_moves();
            a.sort_by_key(Move::to_uci);
            b.sort_by_key(Move::to_uci);
            prop_assert_eq!(a, b);
        }
    }

    #[test]
    fn ut_rul_005_ur_26_to_fen_writes_the_en_passant_square_after_every_double_push(
        choices in proptest::collection::vec(any::<u16>(), 0..120),
    ) {
        for (position, m) in playout(&choices) {
            let moved = position.piece_at(m.from).unwrap();
            let after = position.play(m).unwrap();
            let fen = after.to_fen();
            let ep_field = fen.as_str().split(' ').nth(3).unwrap().to_string();
            let double_push = moved.kind == PieceKind::Pawn
                && m.from.file() == m.to.file()
                && m.from.rank().index().abs_diff(m.to.rank().index()) == 2;
            if double_push {
                let rank = if moved.side == Side::White { 2 } else { 5 };
                let expected = chess_rules::Square::new(m.from.file(), chess_rules::Rank::new(rank).unwrap());
                prop_assert_eq!(ep_field, expected.to_string());
            } else {
                prop_assert_eq!(ep_field, "-".to_string());
            }
        }
    }
}
