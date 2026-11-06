//! DD-RUL-001: small, `Copy`, upstream-free value types. No constructor can build a value that
//! is off the board, so indexing by `Square` cannot go out of range.

use core::fmt;

use crate::error::{MoveParseError, SquareError};

/// A file, `a` = 0 to `h` = 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct File(u8);

/// A rank, `1` = 0 to `8` = 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rank(u8);

impl File {
    pub fn new(index: u8) -> Result<File, SquareError> {
        if index < 8 {
            Ok(File(index))
        } else {
            Err(SquareError::OutOfRange {
                value: u32::from(index),
            })
        }
    }

    pub fn index(self) -> u8 {
        self.0
    }

    /// Lower-case letter `a` to `h`.
    pub fn to_char(self) -> char {
        char::from(b'a'.saturating_add(self.0))
    }
}

impl Rank {
    pub fn new(index: u8) -> Result<Rank, SquareError> {
        if index < 8 {
            Ok(Rank(index))
        } else {
            Err(SquareError::OutOfRange {
                value: u32::from(index),
            })
        }
    }

    pub fn index(self) -> u8 {
        self.0
    }

    /// Digit `1` to `8`.
    pub fn to_char(self) -> char {
        char::from(b'1'.saturating_add(self.0))
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// A square, index `file + 8 * rank` (a1 = 0, h1 = 7, a8 = 56, h8 = 63).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Square(u8);

impl Square {
    /// Total: a valid file and rank always name a square.
    pub fn new(file: File, rank: Rank) -> Square {
        Square(file.0 + 8 * rank.0)
    }

    pub fn from_index(index: u8) -> Result<Square, SquareError> {
        if index < 64 {
            Ok(Square(index))
        } else {
            Err(SquareError::OutOfRange {
                value: u32::from(index),
            })
        }
    }

    /// Parses a name such as `e4`: file `a` to `h` and rank `1` to `8`, lower case only.
    pub fn from_algebraic(text: &str) -> Result<Square, SquareError> {
        let bytes = text.as_bytes();
        let (Some(&f), Some(&r), 2) = (bytes.first(), bytes.get(1), bytes.len()) else {
            return Err(SquareError::BadText);
        };
        if !(b'a'..=b'h').contains(&f) || !(b'1'..=b'8').contains(&r) {
            return Err(SquareError::BadText);
        }
        Ok(Square((f - b'a') + 8 * (r - b'1')))
    }

    pub fn index(self) -> u8 {
        self.0
    }

    pub fn file(self) -> File {
        File(self.0 % 8)
    }

    pub fn rank(self) -> Rank {
        Rank(self.0 / 8)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    /// English name, lower case, for messages ("no knight can reach f3").
    pub fn name(self) -> &'static str {
        match self {
            PieceKind::Pawn => "pawn",
            PieceKind::Knight => "knight",
            PieceKind::Bishop => "bishop",
            PieceKind::Rook => "rook",
            PieceKind::Queen => "queen",
            PieceKind::King => "king",
        }
    }

    /// Lower-case FEN and UCI letter (`p n b r q k`).
    pub(crate) fn letter(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }

    /// The four kinds a pawn can promote to.
    fn from_promotion_letter(c: char) -> Option<PieceKind> {
        match c {
            'q' => Some(PieceKind::Queen),
            'r' => Some(PieceKind::Rook),
            'b' => Some(PieceKind::Bishop),
            'n' => Some(PieceKind::Knight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub side: Side,
    pub kind: PieceKind,
}

/// A move as `from`, `to` and an optional promotion piece. Castling is the king's two-square
/// move (`e1g1`), as in standard UCI (DR-RUL-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

impl Move {
    pub fn to_uci(&self) -> String {
        let mut s = format!("{}{}", self.from, self.to);
        if let Some(kind) = self.promotion {
            s.push(kind.letter());
        }
        s
    }

    /// Lower case only; four characters, or five when promoting to `q`, `r`, `b` or `n`.
    pub fn from_uci(text: &str) -> Result<Move, MoveParseError> {
        if !text.is_ascii() || !(4..=5).contains(&text.len()) {
            return Err(MoveParseError::BadUci);
        }
        let from = text
            .get(0..2)
            .and_then(|t| Square::from_algebraic(t).ok())
            .ok_or(MoveParseError::BadUci)?;
        let to = text
            .get(2..4)
            .and_then(|t| Square::from_algebraic(t).ok())
            .ok_or(MoveParseError::BadUci)?;
        let promotion = match text.chars().nth(4) {
            None => None,
            Some(c) => Some(PieceKind::from_promotion_letter(c).ok_or(MoveParseError::BadUci)?),
        };
        Ok(Move {
            from,
            to,
            promotion,
        })
    }
}
