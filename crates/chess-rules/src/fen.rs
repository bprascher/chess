//! DD-RUL-008: `Fen`, a validated and normalised six-field FEN (SDD 3.11.6, IF-3). Parse, do
//! not validate: a `Fen` value is always a playable position. Only this module and `position`
//! import `shakmaty`.

use core::fmt;
use core::hash::{Hash, Hasher};

use shakmaty::fen::{Fen as ShakFen, ParseFenError};
use shakmaty::{CastlingMode, Chess, EnPassantMode, PositionErrorKinds};

use crate::error::{FenError, FenField, IllegalPosition};
use crate::position::Position;

/// Longest accepted FEN text: a legal six-field FEN needs at most 103 bytes (SDD 3.11.6 step 1).
pub const MAX_FEN_LEN: usize = 128;

/// A validated FEN. Equality, ordering of fields and hashing are on the normalised text, that
/// is on all six fields as parsed values (counters as numbers). The en passant square is kept
/// as given, so two spellings of one board that differ only there are two `Fen` values.
#[derive(Clone)]
pub struct Fen {
    text: String,
    position: Position,
}

impl fmt::Debug for Fen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Fen").field(&self.text).finish()
    }
}

impl PartialEq for Fen {
    fn eq(&self, other: &Fen) -> bool {
        self.text == other.text
    }
}

impl Eq for Fen {}

impl Hash for Fen {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl fmt::Display for Fen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl Fen {
    /// The six normalised fields as text (XC-26: the engine builds `position fen ...` from it).
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Exactly six fields separated by single spaces, at most `MAX_FEN_LEN` bytes. Each failure
    /// names its field or the illegal-position reason. A checkmate or stalemate is valid here.
    pub fn parse(text: &str) -> Result<Fen, FenError> {
        if text.len() > MAX_FEN_LEN {
            return Err(FenError::TooLong { len: text.len() });
        }
        let fields: Vec<&str> = text.split(' ').collect();
        let [placement, side, castling, ep, halfmove, fullmove] = fields.as_slice() else {
            return Err(FenError::FieldCount {
                found: fields.len(),
            });
        };
        let side_white = match *side {
            "w" => true,
            "b" => false,
            _ => return Err(bad(FenField::SideToMove, "must be w or b")),
        };
        check_placement(placement).map_err(|d| bad(FenField::Placement, d))?;
        check_castling(castling).map_err(|d| bad(FenField::Castling, d))?;
        check_ep(ep, side_white).map_err(|d| bad(FenField::EnPassant, d))?;
        check_counter(halfmove, 0).map_err(|d| bad(FenField::HalfmoveClock, d))?;
        check_counter(fullmove, 1).map_err(|d| bad(FenField::FullmoveNumber, d))?;

        let parsed = ShakFen::from_ascii(text.as_bytes()).map_err(|e| {
            let field = match e {
                ParseFenError::InvalidTurn => FenField::SideToMove,
                ParseFenError::InvalidCastling => FenField::Castling,
                ParseFenError::InvalidEpSquare => FenField::EnPassant,
                ParseFenError::InvalidHalfmoveClock => FenField::HalfmoveClock,
                ParseFenError::InvalidFullmoves => FenField::FullmoveNumber,
                _ => FenField::Placement,
            };
            bad(field, "not accepted")
        })?;
        let chess: Chess = parsed
            .into_position(CastlingMode::Standard)
            .map_err(|e| FenError::Illegal(illegal_position(e.kinds())))?;
        Ok(Fen::from_position(&Position::from_chess(chess)))
    }

    /// The FEN a position writes: the en passant square is written after every double push,
    /// whether or not a capture is possible (UR-26, SDD 3.11.5).
    pub(crate) fn from_position(position: &Position) -> Fen {
        let text = ShakFen::from_position(position.chess(), EnPassantMode::Always).to_string();
        Fen {
            text,
            position: position.clone(),
        }
    }

    pub(crate) fn position(&self) -> &Position {
        &self.position
    }
}

fn bad(field: FenField, detail: &'static str) -> FenError {
    FenError::BadField { field, detail }
}

fn check_placement(text: &str) -> Result<(), &'static str> {
    let mut ranks = 0usize;
    for rank in text.split('/') {
        ranks += 1;
        let mut squares = 0u32;
        let mut previous_digit = false;
        for c in rank.chars() {
            match c {
                '1'..='8' => {
                    if previous_digit {
                        return Err("two digits in a row in a rank");
                    }
                    squares += c.to_digit(10).unwrap_or(0);
                    previous_digit = true;
                }
                'p' | 'n' | 'b' | 'r' | 'q' | 'k' | 'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => {
                    squares += 1;
                    previous_digit = false;
                }
                _ => {
                    return Err(
                        "only the letters pnbrqk and PNBRQK and the digits 1 to 8 are allowed",
                    );
                }
            }
        }
        if squares != 8 {
            return Err("every rank must cover exactly eight squares");
        }
    }
    if ranks == 8 {
        Ok(())
    } else {
        Err("needs eight ranks separated by /")
    }
}

fn check_castling(text: &str) -> Result<(), &'static str> {
    if text == "-" {
        return Ok(());
    }
    let order = "KQkq";
    let mut next = 0usize;
    for c in text.chars() {
        match order.find(c) {
            Some(i) if i >= next => next = i + 1,
            _ => return Err("must be - or letters of KQkq, each once, in that order"),
        }
    }
    if text.is_empty() {
        Err("must be - or letters of KQkq, each once, in that order")
    } else {
        Ok(())
    }
}

fn check_ep(text: &str, white_to_move: bool) -> Result<(), &'static str> {
    if text == "-" {
        return Ok(());
    }
    let bytes = text.as_bytes();
    let (Some(&file), Some(&rank), 2) = (bytes.first(), bytes.get(1), bytes.len()) else {
        return Err("must be - or a square name such as e3");
    };
    if !(b'a'..=b'h').contains(&file) {
        return Err("must be - or a square name such as e3");
    }
    let wanted = if white_to_move { b'6' } else { b'3' };
    if rank == wanted {
        Ok(())
    } else {
        Err("the square must be on rank 6 when White is to move and on rank 3 when Black is")
    }
}

fn check_counter(text: &str, minimum: u32) -> Result<(), &'static str> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("must be a whole number written with digits");
    }
    match text.parse::<u32>() {
        Ok(n) if n >= minimum => Ok(()),
        Ok(_) => Err("must be at least 1"),
        Err(_) => Err("is too large"),
    }
}

fn illegal_position(kinds: PositionErrorKinds) -> IllegalPosition {
    if kinds.intersects(PositionErrorKinds::EMPTY_BOARD | PositionErrorKinds::MISSING_KING) {
        IllegalPosition::MissingKing
    } else if kinds.contains(PositionErrorKinds::TOO_MANY_KINGS) {
        IllegalPosition::TooManyKings
    } else if kinds.contains(PositionErrorKinds::PAWNS_ON_BACKRANK) {
        IllegalPosition::PawnOnBackRank
    } else if kinds.contains(PositionErrorKinds::OPPOSITE_CHECK) {
        IllegalPosition::OpponentInCheck
    } else if kinds.contains(PositionErrorKinds::INVALID_CASTLING_RIGHTS) {
        IllegalPosition::Other(
            "the castling rights do not match the pieces on the board".to_string(),
        )
    } else if kinds.contains(PositionErrorKinds::INVALID_EP_SQUARE) {
        IllegalPosition::Other(
            "the en passant square does not match the pieces on the board".to_string(),
        )
    } else if kinds.contains(PositionErrorKinds::IMPOSSIBLE_CHECK) {
        IllegalPosition::Other("the side to move is in a check that cannot arise".to_string())
    } else if kinds.contains(PositionErrorKinds::TOO_MUCH_MATERIAL) {
        IllegalPosition::Other("there is more material than a game can produce".to_string())
    } else {
        IllegalPosition::Other("the position cannot be played".to_string())
    }
}
