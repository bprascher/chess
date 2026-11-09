//! DD-RUL-014: typed errors. `Display` is English, names the problem and carries no path or
//! internal text (NFR-9, architecture 5.5). Part B adds the remaining `MoveParseError`
//! variants (`Empty`, `TooLong`, `Syntax`, ...) and the PGN errors.

use crate::notation::San;
use crate::types::PieceKind;

/// A coordinate or square name that is not on the board (DD-RUL-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SquareError {
    #[error("coordinate {value} is outside the board")]
    OutOfRange { value: u32 },
    #[error("not a square name (a file a to h and a rank 1 to 8, lower case)")]
    BadText,
}

/// Errors of move text parsing (SDD section 5, D-51). Each variant carries exactly the data its
/// reason needs, so `chess-app` copies and does not recompute (OQ-RUL-003). `NoMatch` is text
/// that is understood but names no legal move: `chess-app` maps it to `IllegalMove`, not to a
/// parse error (D-51).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoveParseError {
    #[error(
        "not a UCI move (four lower-case characters, a fifth letter q, r, b or n for a promotion)"
    )]
    BadUci,
    #[error("no move was entered")]
    Empty,
    #[error("that text is too long to be a move")]
    TooLong,
    #[error("that is not a move (examples: e4, Nf3, exd5, O-O, e7e8=Q)")]
    Syntax,
    #[error("piece letters are upper case: did you mean {suggestion}?")]
    LowercasePiece { suggestion: String },
    #[error("a pawn reaching the last rank must promote (q, r, b or n)")]
    PromotionRequired { choices: Vec<PieceKind> },
    #[error("more than one move fits: {}", join_san(.candidates))]
    Ambiguous { candidates: Vec<San> },
    #[error("illegal move: {reason}")]
    NoMatch { reason: IllegalReason },
}

fn join_san(candidates: &[San]) -> String {
    candidates
        .iter()
        .map(San::as_str)
        .collect::<Vec<_>>()
        .join(", ")
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
    #[error("that move does not promote a pawn, so it takes no piece letter")]
    NotAPromotion,
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

/// Why a PGN game could not be read (SDD 3.11.8). The enum is the stable part (NFR-9).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PgnErrorKind {
    #[error("illegal move: {reason}")]
    IllegalSan { reason: IllegalReason },
    #[error("more than one move fits: {}", join_san(.candidates))]
    AmbiguousSan { candidates: Vec<San> },
    #[error("bad syntax: {reason}")]
    BadSyntax { reason: SyntaxReason },
    #[error("a comment is not closed before the end of the input")]
    UnterminatedComment,
    #[error("the parentheses of a variation do not match")]
    UnbalancedVariation,
    #[error("variations are nested deeper than the limit of {limit}")]
    VariationTooDeep { limit: usize },
    #[error("bad tag: {reason}")]
    BadTag { reason: TagReason },
    #[error("bad FEN tag: {0}")]
    BadFen(FenError),
}

/// Why movetext is `PgnErrorKind::BadSyntax` (C-R5, RF-07). The variant is the stable part; the
/// `Display` text is for display only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SyntaxReason {
    /// The word is not a move: empty, too long, not SAN, or a bad UCI form.
    #[error("not a move")]
    NotAMove,
    /// The word holds bytes that are not UTF-8.
    #[error("not text")]
    NotText,
    /// A piece letter in lower case (`nf3`).
    #[error("piece letters are upper case")]
    LowercasePieceLetter,
    #[error("a closing brace without an opening one")]
    StrayClosingBrace,
    #[error("a variation must follow a move")]
    VariationWithoutMove,
    /// `$` not followed by a number from 0 to 255.
    #[error("a NAG is $ and a number from 0 to 255")]
    BadNag,
    #[error("a NAG must follow a move")]
    NagWithoutMove,
    /// A tag section, a blank line and then another tag section (RF-03).
    #[error("a tag section needs movetext before the next tag section")]
    TagSectionWithoutMovetext,
    /// The reader's own bookkeeping failed; a defect indicator, never produced by input.
    #[error("internal: no open variation frame")]
    Internal,
}

/// Why a tag line is `PgnErrorKind::BadTag` (C-R5, RF-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TagReason {
    #[error("a tag name starts with an upper-case letter")]
    NameStart,
    #[error("a tag value is in double quotes")]
    ValueNotQuoted,
    #[error("a tag value is not closed on its line")]
    ValueNotClosed,
    #[error("a tag ends with a closing bracket")]
    MissingClosingBracket,
}

/// A PGN game that could not be read. `game_index` is 1-based, `line` is 1-based and counts
/// `\n`, `\r\n` and a lone `\r` as one break each, `col` counts bytes from 1 (SDD 3.11.8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("game {game_index}, line {line}, column {col}: {kind}")]
pub struct PgnParseError {
    pub game_index: usize,
    pub line: usize,
    pub col: usize,
    pub kind: PgnErrorKind,
}

/// A game model that cannot be written as PGN (SDD 3.11.7). Never partial output.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PgnWriteError {
    #[error(
        "tag name {name:?} is not a valid PGN tag name (an upper-case letter, then letters, digits or underscores)"
    )]
    BadTagName { name: String },
    #[error("tag {name:?} is set by a dedicated field and cannot be an extra tag")]
    ReservedTag { name: String },
    #[error(
        "a move in the game is not legal at its place (index {ply} in its move list, counting from 0)"
    )]
    IllegalMove {
        /// Pairs `(node index, variation index)` from the mainline down; empty for the mainline.
        variation_path: Vec<(usize, usize)>,
        /// 0-based index of the move in its sequence.
        ply: usize,
    },
    #[error("variations are nested deeper than the limit of {limit}")]
    VariationTooDeep { limit: usize },
    #[error("game {game_index}: {source}")]
    InGame {
        /// 1-based.
        game_index: usize,
        #[source]
        source: Box<PgnWriteError>,
    },
}
