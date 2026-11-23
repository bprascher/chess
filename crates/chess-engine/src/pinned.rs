//! DD-EN-015: the `UCI_Elo` range of the bundled engine, as a constant (D-46, FF-7, TR-9).
//!
//! This file is the single source of truth for the value. No other source file, test or
//! document repeats it: tests refer to the constant, and the engine integration test EI-EN-002
//! compares it with the live handshake of the real binary.

/// An inclusive `UCI_Elo` range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EloRange {
    pub min: u32,
    pub max: u32,
}

/// FINAL. The value is the range the Stockfish 19 build step observed
/// (docs/engineering/stockfish19-fetch-results.md, architecture ASM-3, task chess-a0r.3.4).
/// Brian Prascher approved it on 2026-11-09 ("Ok"); it is recorded as architecture D-112. It is
/// an input to the permanent roster identity (D-46), so changing it changes every future roster
/// and needs the calibration re-run of architecture 4.6 and a new decision. This constant is the
/// single definition: nothing else in the code, the tests or the documents repeats the number.
pub const PINNED_UCI_ELO_RANGE: EloRange = EloRange {
    min: 1320,
    max: 3190,
};
