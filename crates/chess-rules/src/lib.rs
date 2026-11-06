//! chess-rules: the pure rules library (SDD-chess-rules, DD-RUL-001 to DD-RUL-008, DD-RUL-014,
//! DD-RUL-015). Part A of chess-a0r.2.3: positions, FEN, legal move generation, position-level
//! draw and game-end predicates, perft. Notation, PGN and ECO are part B (chess-a0r.2.4).
//!
//! The crate does no I/O and holds no clock or random source. `shakmaty` types never appear in
//! the public API (D-3, DR-RUL-001): only the modules `position` and `fen` import it.
#![forbid(unsafe_code)]

mod error;
mod fen;
mod position;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;
mod types;

pub use error::{
    FenError, FenField, IllegalMove, IllegalPosition, IllegalReason, MoveParseError, SquareError,
};
pub use fen::{Fen, MAX_FEN_LEN};
pub use position::{Position, PositionKey};
pub use types::{File, Move, Piece, PieceKind, Rank, Side, Square};
