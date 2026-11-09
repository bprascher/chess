//! DD-DOM-017 (SDD 3.11.12): a terminal-independent board. `BoardGrid::from_fen` turns a FEN
//! and a UCI last-move text into a grid of styled cells (glyph, square shade, highlight), so
//! no front-end needs a `Position` or any rules type (TR-1.6, XC-12). `render_text` lays the
//! grid out as lines: each square is 3 columns wide, labels `A` to `H` and `1` to `8` sit on
//! all four sides, and an ESC byte is written for the `Ansi` style only (UI-4).

use chess_rules::{Fen, Move, PieceKind, Position, Side, Square};

use crate::error::BoardError;

/// The side drawn at the bottom. Replaces the rules `Side` in the public signature (OQ-DOM-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardStyle {
    /// ASCII letters `KQRBNP` and `kqrbnp`, `.` for an empty square; marks by characters.
    Plain,
    /// Unicode chess glyphs; shade and highlight by characters.
    Unicode,
    /// Unicode glyphs with basic SGR background colours.
    Ansi,
    /// Unicode glyphs, no escape bytes (`NO_COLOR`, read by the CLI).
    NoColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shade {
    Light,
    Dark,
}

/// Why a square stands out. Several flags can hold on one square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Highlight {
    pub last_move_from: bool,
    pub last_move_to: bool,
    /// The king of the side to move, when it is in check.
    pub check: bool,
    /// The view is not the live position; set on the last-move squares.
    pub rewound: bool,
}

/// Exactly `{glyph, shade, highlight}` (architecture 5.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub glyph: char,
    pub shade: Shade,
    pub highlight: Highlight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardView {
    /// UCI text such as `e2e4`, or `e7e8q`.
    pub last_move: Option<String>,
    pub rewound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardGrid {
    /// `rows[0]` is the top row on screen.
    pub rows: [[Cell; 8]; 8],
    pub top_labels: [char; 8],
    pub bottom_labels: [char; 8],
    pub left_labels: [char; 8],
    pub right_labels: [char; 8],
}

const FILES: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
const RANKS: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

fn glyph(style: BoardStyle, side: Side, kind: PieceKind) -> char {
    if style == BoardStyle::Plain {
        let letter = match kind {
            PieceKind::King => 'K',
            PieceKind::Queen => 'Q',
            PieceKind::Rook => 'R',
            PieceKind::Bishop => 'B',
            PieceKind::Knight => 'N',
            PieceKind::Pawn => 'P',
        };
        return match side {
            Side::White => letter,
            Side::Black => letter.to_ascii_lowercase(),
        };
    }
    let base: u32 = match side {
        Side::White => 0x2654,
        Side::Black => 0x265A,
    };
    let offset = match kind {
        PieceKind::King => 0,
        PieceKind::Queen => 1,
        PieceKind::Rook => 2,
        PieceKind::Bishop => 3,
        PieceKind::Knight => 4,
        PieceKind::Pawn => 5,
    };
    char::from_u32(base + offset).unwrap_or('?')
}

fn empty_glyph(style: BoardStyle, shade: Shade) -> char {
    match (style, shade) {
        (BoardStyle::Plain, _) => '.',
        (BoardStyle::Unicode | BoardStyle::NoColor, Shade::Dark) => '\u{b7}',
        _ => ' ',
    }
}

/// a1 is dark.
fn shade_of(square: Square) -> Shade {
    if (square.file().index() + square.rank().index()).is_multiple_of(2) {
        Shade::Dark
    } else {
        Shade::Light
    }
}

impl BoardGrid {
    /// Validates `fen` with `chess_rules::Fen::parse` and the last move with `Move::from_uci`.
    pub fn from_fen(
        fen: &str,
        view: &BoardView,
        style: BoardStyle,
        orientation: Orientation,
    ) -> Result<BoardGrid, BoardError> {
        let position = Position::from_fen(&Fen::parse(fen).map_err(|_| BoardError::BadFen)?);
        let last = match &view.last_move {
            Some(text) => Some(Move::from_uci(text).map_err(|_| BoardError::BadLastMove)?),
            None => None,
        };
        let king_in_check = if position.is_check() {
            (0..64u8)
                .filter_map(|i| Square::from_index(i).ok())
                .find(|s| {
                    position
                        .piece_at(*s)
                        .is_some_and(|p| p.kind == PieceKind::King && p.side == position.turn())
                })
        } else {
            None
        };
        let blank = Cell {
            glyph: ' ',
            shade: Shade::Light,
            highlight: Highlight::default(),
        };
        let mut rows = [[blank; 8]; 8];
        let mut files = FILES;
        let mut ranks = RANKS;
        ranks.reverse(); // top to bottom for White at the bottom: 8 .. 1
        if orientation == Orientation::Black {
            files.reverse();
            ranks.reverse();
        }
        for (r, row) in rows.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                let (rank_index, file_index) = match orientation {
                    Orientation::White => (7 - r, c),
                    Orientation::Black => (r, 7 - c),
                };
                let square = u8::try_from(rank_index * 8 + file_index)
                    .ok()
                    .and_then(|i| Square::from_index(i).ok());
                let Some(square) = square else { continue };
                let shade = shade_of(square);
                let glyph = match position.piece_at(square) {
                    Some(p) => glyph(style, p.side, p.kind),
                    None => empty_glyph(style, shade),
                };
                let from = last.is_some_and(|m| m.from == square);
                let to = last.is_some_and(|m| m.to == square);
                *cell = Cell {
                    glyph,
                    shade,
                    highlight: Highlight {
                        last_move_from: from,
                        last_move_to: to,
                        check: king_in_check == Some(square),
                        rewound: view.rewound && (from || to),
                    },
                };
            }
        }
        Ok(BoardGrid {
            rows,
            top_labels: files,
            bottom_labels: files,
            left_labels: ranks,
            right_labels: ranks,
        })
    }
}

const RESET: &str = "\u{1b}[0m";

fn ansi_background(cell: &Cell) -> &'static str {
    let h = cell.highlight;
    if h.check {
        "\u{1b}[41m"
    } else if h.rewound {
        "\u{1b}[44m"
    } else if h.last_move_from || h.last_move_to {
        "\u{1b}[43m"
    } else if cell.shade == Shade::Dark {
        "\u{1b}[42m"
    } else {
        "\u{1b}[47m"
    }
}

/// Three columns for a square: the glyph with a mark on each side. Plain, Unicode and NoColor
/// show check as `<g>`, a rewound last move as `(g)` and a last move as `[g]`; Ansi uses a
/// background colour and plain padding.
fn cell_text(cell: &Cell, style: BoardStyle) -> String {
    let g = cell.glyph;
    if style == BoardStyle::Ansi {
        return format!("{}\u{1b}[30m {g} {RESET}", ansi_background(cell));
    }
    let h = cell.highlight;
    let (open, close) = if h.check {
        ('<', '>')
    } else if h.rewound {
        ('(', ')')
    } else if h.last_move_from || h.last_move_to {
        ('[', ']')
    } else {
        (' ', ' ')
    };
    format!("{open}{g}{close}")
}

fn label_row(labels: &[char; 8]) -> String {
    let cells: String = labels.iter().map(|c| format!(" {c} ")).collect();
    format!("  {cells}  ")
}

/// Ten lines: the top labels, eight ranks with a label on each side, the bottom labels.
pub fn render_text(grid: &BoardGrid, style: &BoardStyle) -> Vec<String> {
    let mut lines = Vec::with_capacity(10);
    lines.push(label_row(&grid.top_labels));
    for (r, row) in grid.rows.iter().enumerate() {
        let left = grid.left_labels.get(r).copied().unwrap_or(' ');
        let right = grid.right_labels.get(r).copied().unwrap_or(' ');
        let cells: String = row.iter().map(|c| cell_text(c, *style)).collect();
        lines.push(format!("{left} {cells} {right}"));
    }
    lines.push(label_row(&grid.bottom_labels));
    lines
}
