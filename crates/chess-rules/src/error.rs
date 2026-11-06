//! DD-RUL-014: typed errors. `Display` is English, names the problem and carries no path or
//! internal text (NFR-9, architecture 5.5). Part B adds the remaining `MoveParseError`
//! variants (`Empty`, `TooLong`, `Syntax`, ...) and the PGN errors.

use crate::types::PieceKind;

/// A coordinate or square name that is not on the board (DD-RUL-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SquareError {
    #[error("coordinate {value} is outside the board")]
    OutOfRange { value: u32 },
    #[error("not a square name (a file a to h and a rank 1 to 8, lower case)")]
    BadText,
}

/// Errors of move text parsing. Only `BadUci` exists in part A (`Move::from_uci`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoveParseError {
    #[error(
        "not a UCI move (four lower-case characters, a fifth letter q, r, b or n for a promotion)"
    )]
    BadUci,
}

/// Why a move is not legal (DD-RUL-007, SDD 3.11.2). The enum is the stable part (NFR-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IllegalReason {
    #[error("there is no piece on that square")]
    NoPiece,
    #[error("that piece belongs to the other side")]
    WrongTurn,
    #[error("no {} can reach that square", .piece.name())]
    CannotReach { piece: PieceKind },
    #[error("the path is blocked by a piece")]
    Blocked,
    #[error("the destination holds one of your own pieces")]
    OwnPieceOnTarget,
    #[error("castling is not allowed: the king or that rook has moved")]
    CastlingRightsLost,
    #[error("castling is not allowed: a piece stands between the king and the rook")]
    CastlingBlocked,
    #[error("castling is not allowed while the king is in check")]
    CastlingOutOfCheck,
    #[error("castling is not allowed: the king would cross or land on an attacked square")]
    CastlingThroughCheck,
    #[error("a pawn reaching the last rank must promote (q, r, b or n)")]
    PromotionRequired,
    #[error("the move would leave your king in check")]
    LeavesKingInCheck,
    #[error("your king is in check and this move does not resolve it")]
    KingInCheck,
    #[error("the move is not legal")]
    Other,
}

/// `Position::play` on a move that is not in `legal_moves()` (IFP-RUL-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("illegal move: {reason}")]
pub struct IllegalMove {
    pub reason: IllegalReason,
}

/// The six FEN fields, for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FenField {
    #[error("piece placement")]
    Placement,
    #[error("side to move")]
    SideToMove,
    #[error("castling")]
    Castling,
    #[error("en passant")]
    EnPassant,
    #[error("halfmove clock")]
    HalfmoveClock,
    #[error("fullmove number")]
    FullmoveNumber,
}

/// A syntactically valid FEN that does not describe a playable position (IF-3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IllegalPosition {
    #[error("each side needs exactly one king and one is missing")]
    MissingKing,
    #[error("a side has more than one king")]
    TooManyKings,
    #[error("a pawn stands on the first or last rank")]
    PawnOnBackRank,
    #[error("the side that is not to move is in check")]
    OpponentInCheck,
    #[error("{0}")]
    Other(String),
}

/// FEN parse and validation errors (IFP-RUL-4). Each message names the field or the reason.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FenError {
    #[error("the FEN is {len} bytes long, more than the limit of 128")]
    TooLong { len: usize },
    #[error("a FEN has exactly six fields separated by single spaces, found {found}")]
    FieldCount { found: usize },
    #[error("invalid {field} field: {detail}")]
    BadField {
        field: FenField,
        detail: &'static str,
    },
    #[error("illegal position: {0}")]
    Illegal(IllegalPosition),
}
