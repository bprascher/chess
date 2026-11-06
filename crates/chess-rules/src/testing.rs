//! DD-RUL-015: verification helpers behind feature `test-support` (always present in this
//! crate's own unit tests). `perft` is built on this crate's `legal_moves` and `play`, so it
//! exercises the adapter conversions (castling, en passant, promotion), not only the upstream
//! generator.

use crate::position::Position;

/// Number of leaf nodes of the legal-move tree of the given depth. Depth 0 is 1.
///
/// A `play` failure on a move from `legal_moves` would be a defect; it counts as an empty
/// subtree here so that the published-count comparison fails instead of panicking.
pub fn perft(position: &Position, depth: u32) -> u64 {
    match depth {
        0 => 1,
        1 => u64::try_from(position.legal_moves().len()).unwrap_or(u64::MAX),
        _ => position
            .legal_moves()
            .into_iter()
            .map(|m| position.play(m).map_or(0, |next| perft(&next, depth - 1)))
            .sum(),
    }
}

// UT-RUL-003 (RULE-1, NFR-3): perft through this crate's own API against the published node
// counts of the Chess Programming Wiki "Perft Results" page (test-strategy "Reference data").
// The counts are recalled, not fetched in this session: see the evidence file, tag [UNV], and
// the cross-check against the upstream generator below. The deep runs are #[ignore]d because
// of their run time; run them with
//   cargo test -p chess-rules --release -- --ignored
#[cfg(test)]
mod tests {
    use super::perft;
    use crate::fen::Fen;
    use crate::position::Position;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    const POS3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    const POS4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
    const POS4_MIRRORED: &str = "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1";
    const POS5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
    const POS6: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

    fn pos(fen: &str) -> Position {
        Position::from_fen(&Fen::parse(fen).expect("fixture FEN parses"))
    }

    fn check(fen: &str, counts: &[u64]) {
        let p = pos(fen);
        for (i, expected) in counts.iter().enumerate() {
            let depth = u32::try_from(i + 1).expect("small depth");
            assert_eq!(perft(&p, depth), *expected, "{fen} depth {depth}");
        }
    }

    #[test]
    fn ut_rul_003_rule_1_perft_start_position_depth_4() {
        check(START, &[20, 400, 8_902, 197_281]);
    }

    #[test]
    fn ut_rul_003_rule_1_perft_kiwipete_depth_3() {
        check(KIWIPETE, &[48, 2_039, 97_862]);
    }

    #[test]
    fn ut_rul_003_rule_1_perft_position_3_depth_5() {
        check(POS3, &[14, 191, 2_812, 43_238, 674_624]);
    }

    #[test]
    fn ut_rul_003_rule_1_perft_position_4_depth_4_and_its_colour_mirror() {
        check(POS4, &[6, 264, 9_467, 422_333]);
        check(POS4_MIRRORED, &[6, 264, 9_467, 422_333]);
    }

    #[test]
    fn ut_rul_003_rule_1_perft_position_5_depth_3() {
        check(POS5, &[44, 1_486, 62_379]);
    }

    #[test]
    fn ut_rul_003_rule_1_perft_position_6_depth_3() {
        check(POS6, &[46, 2_079, 89_890]);
    }

    // Depth 4 (Kiwipete, positions 5 and 6) is part of the default suite only if it runs in
    // reasonable time in a debug build; measured and recorded in the evidence file.
    #[test]
    fn ut_rul_003_rule_1_perft_kiwipete_position_5_position_6_depth_4() {
        let p = pos(KIWIPETE);
        assert_eq!(perft(&p, 4), 4_085_603, "kiwipete depth 4");
        let p = pos(POS5);
        assert_eq!(perft(&p, 4), 2_103_487, "position 5 depth 4");
        let p = pos(POS6);
        assert_eq!(perft(&p, 4), 3_894_594, "position 6 depth 4");
    }

    #[test]
    #[ignore = "run time: start depth 5 is 4.8 million leaves; run with --release -- --ignored"]
    fn ut_rul_003_rule_1_perft_start_position_depth_5() {
        assert_eq!(perft(&pos(START), 5), 4_865_609);
    }

    #[test]
    #[ignore = "run time: Kiwipete depth 5 is 194 million leaves; run with --release -- --ignored"]
    fn ut_rul_003_rule_1_perft_kiwipete_depth_5() {
        assert_eq!(perft(&pos(KIWIPETE), 5), 193_690_690);
    }

    #[test]
    #[ignore = "run time: positions 4, 5, 6 at depth 5; run with --release -- --ignored"]
    fn ut_rul_003_rule_1_perft_positions_4_5_6_depth_5() {
        assert_eq!(perft(&pos(POS4), 5), 15_833_292, "position 4");
        assert_eq!(perft(&pos(POS5), 5), 89_941_194, "position 5");
        assert_eq!(perft(&pos(POS6), 5), 164_075_551, "position 6");
    }

    #[test]
    #[ignore = "run time: start depth 6 is 119 million leaves, position 3 depth 6 is 11 million; run with --release -- --ignored"]
    fn ut_rul_003_rule_1_perft_start_depth_6_and_position_3_depth_6() {
        assert_eq!(perft(&pos(START), 6), 119_060_324);
        assert_eq!(perft(&pos(POS3), 6), 11_030_083);
    }

    /// Cross-check of the recalled counts and of the adapter against the upstream generator:
    /// our perft (through the adapter) must equal `shakmaty::perft` at every shared depth.
    #[test]
    fn ut_rul_003_nfr_3_our_perft_equals_the_upstream_perft() {
        for fen in [START, KIWIPETE, POS3, POS4, POS4_MIRRORED, POS5, POS6] {
            let ours = pos(fen);
            for depth in 1..=3 {
                assert_eq!(
                    perft(&ours, depth),
                    shakmaty::perft(ours.chess(), depth),
                    "{fen} depth {depth}"
                );
            }
        }
    }
}
