//! C-R6 (chess-94x), desk review R8, RF-08: what `ParseMode::Pgn` accepts, as pinned behaviour
//! (the doc comment on `ParseMode::Pgn` lists these), and `e.p.` after a capture.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

mod common;

use chess_rules::{ParseMode, PgnGame, PgnLimits, PgnReader, Position};
use common::{pos, ucis_of_game};

fn pgn_move(p: &Position, text: &str) -> Result<String, chess_rules::MoveParseError> {
    p.parse_move(text, ParseMode::Pgn).map(|m| m.uci)
}

#[test]
fn c_r6_pgn_mode_accepts_the_listed_spellings() {
    let start = Position::standard();
    for (text, uci) in [
        ("e4", "e2e4"),
        ("E4", "e2e4"),
        ("e2e4", "e2e4"),
        ("Ng1f3", "g1f3"),
        ("Nf3+++", "g1f3"),
        ("Nf3!?", "g1f3"),
    ] {
        assert_eq!(pgn_move(&start, text).as_deref(), Ok(uci), "{text}");
    }
    let castle = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    assert_eq!(pgn_move(&castle, "0-0").as_deref(), Ok("e1g1"));
    assert_eq!(pgn_move(&castle, "O-O-O").as_deref(), Ok("e1c1"));
    let promo = pos("8/P6k/8/8/8/8/7K/8 w - - 0 1");
    for text in ["a8=Q", "a8Q", "a8q"] {
        assert_eq!(pgn_move(&promo, text).as_deref(), Ok("a7a8q"), "{text}");
    }
}

#[test]
fn c_r6_pgn_mode_does_not_take_coordinate_moves_for_a_non_pawn_or_lower_case_pieces() {
    let start = Position::standard();
    assert!(pgn_move(&start, "g1f3").is_err());
    assert!(pgn_move(&start, "nf3").is_err());
}

fn read_ok(text: &str) -> PgnGame {
    let mut all: Vec<_> = PgnReader::new(text.as_bytes(), PgnLimits::default()).collect();
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0).unwrap()
}

#[test]
fn c_r6_e_p_after_a_capture_is_an_annotation() {
    for text in [
        "1. e4 a6 2. e5 d5 3. exd6 e.p. Nf6 *",
        "1. e4 a6 2. e5 d5 3. exd6e.p. Nf6 *",
        "1. e4 a6 2. e5 d5 3. exd6 E.P. Nf6 *",
        "1. e4 a6 2. e5 d5 3. exd6 {the pawn} e.p. Nf6 *",
        "1. e4 a6 2. e5 d5 3. exd6 e.p Nf6 *",
        "1. e4 a6 2. e5 d5 3. exd6 e.p. $1 Nf6 *",
    ] {
        let g = read_ok(text);
        assert_eq!(
            ucis_of_game(&g),
            ["e2e4", "a7a6", "e4e5", "d7d5", "e5d6", "g8f6"],
            "{text}"
        );
    }
    // And in a variation, for Black.
    let g = read_ok("1. a3 d5 2. a4 d4 3. c4 (3. e4 dxe3 e.p.) dxc3 e.p. *");
    assert_eq!(g.moves.len(), 6);
    assert_eq!(g.moves[4].variations.len(), 1);
}

#[test]
fn c_r6_e_p_anywhere_else_is_still_an_error() {
    for text in [
        "1. e4 e.p. e5 *",                           // after a non-capture
        "e.p. 1. e4 *",                              // before any move
        "1. e4 a6 2. e5 d5 3. exd6 e.p. e.p. Nf6 *", // twice
        "1. e4 d5 2. exd5 ( e.p. ) *",               // not after a capture in its own sequence
        "1. e4 d5 2. exd5 Nf6 e.p. *",               // after a move that is not a capture
        "1. e4 d5 2. e4e5e.p. *",                    // glued onto a non-capture
    ] {
        let mut all = PgnReader::new(text.as_bytes(), PgnLimits::default());
        assert!(all.next().unwrap().is_err(), "{text}");
    }
}
