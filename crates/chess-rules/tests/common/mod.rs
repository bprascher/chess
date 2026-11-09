//! Shared helpers for the chess-rules integration tests (public API only).
#![allow(
    dead_code,
    reason = "each test file uses a different subset of these helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

use chess_rules::{Fen, Move, Position, Square};

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn pos(fen: &str) -> Position {
    Position::from_fen(&Fen::parse(fen).unwrap())
}

pub fn sq(name: &str) -> Square {
    Square::from_algebraic(name).unwrap()
}

pub fn mv(uci: &str) -> Move {
    Move::from_uci(uci).unwrap()
}

/// Sorted UCI strings of all legal moves.
pub fn ucis(p: &Position) -> Vec<String> {
    let mut v: Vec<String> = p.legal_moves().iter().map(Move::to_uci).collect();
    v.sort();
    v
}

pub fn play_all(p: &Position, line: &[&str]) -> Position {
    let mut cur = p.clone();
    for u in line {
        cur = cur.play(mv(u)).unwrap();
    }
    cur
}

/// The move list of `ucis` played from `start`, as plain nodes (no NAGs, comments or variations).
pub fn nodes(start: &Position, ucis: &[&str]) -> Vec<chess_rules::PgnNode> {
    let mut cur = start.clone();
    let mut out = Vec::new();
    for u in ucis {
        let m = mv(u);
        out.push(chess_rules::PgnNode::new(m));
        cur = cur.play(m).unwrap();
    }
    out
}

/// A game from the standard start (default tags) with the given UCI moves and result.
pub fn game(ucis: &[&str], result: chess_rules::PgnResult) -> chess_rules::PgnGame {
    let start = Position::standard();
    let moves = nodes(&start, ucis);
    chess_rules::PgnGame::new(start, moves, result)
}
