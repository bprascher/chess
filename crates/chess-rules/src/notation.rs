//! DD-RUL-005 and DD-RUL-006: canonical SAN (the formatter is `Position::san`, which lives with
//! the upstream types in `position`) and the move-text matcher. `Position::parse_move` is the
//! only typed-move parser in the system (TR-8, D-45). This module sees no upstream type.

use core::fmt;

use crate::error::{IllegalReason, MoveParseError};
use crate::position::Position;
use crate::types::{File, Move, Piece, PieceKind, Rank, Square};

/// Longest text `parse_move` accepts, after trimming (SDD 3.12). The longest legal SAN is 7
/// characters; the rest is room for annotations.
pub const MAX_MOVE_TEXT: usize = 16;

/// Standard algebraic notation of one move, canonical form with `+` or `#` (DD-RUL-005). Only
/// `Position::san` and `Position::parse_move` make one, so every value is canonical.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct San(String);

impl San {
    pub(crate) fn new(text: String) -> San {
        San(text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for San {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The two policies of the one matcher (DD-RUL-006). `Typed` is lenient (CLI-3, GUI-2 (c)):
/// coordinate moves (`e2e4`) and the upper-case `B` fallback.
///
/// `Pgn` (RULE-7) is the import policy. What it guarantees: a text yields a move only if that
/// move is in `legal_moves()` (a differential run found no exception, RF-08), and `B` is always
/// a bishop, so an import cannot silently read an illegal SAN as a pawn move. What it does
/// **not** guarantee is the PGN Standard's strict export grammar: it is a lenient reader of
/// SAN. It accepts: an upper-case file letter (`E4`); a from-file or from-rank on any move,
/// including over-disambiguation (`Ng1f3`, `e2e4` read as a SAN pawn move with origin
/// square); `0-0` and `0-0-0` for castling; a promotion with or without `=` and in either
/// case (`a8=Q`, `a8Q`, `a8q`); any run of `+ # ! ?` after the move (a trailing `!`, `?`, `!!`,
/// `??`, `!?` or `?!` becomes a NAG, any other run is dropped, `e4+++` is accepted). It
/// rejects a wrong capture sign (`Nxf3` for a non-capture), a coordinate move that is not valid
/// SAN (`g1f3`), lower-case piece letters (`nf3`) and any unparsable word; in the PGN reader
/// each of these fails the whole game. The reader (not this matcher) also skips a move number,
/// and accepts `e.p.` after a capture as an annotation (C-R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseMode {
    Typed,
    Pgn,
}

/// The result of `Position::parse_move`. `uci` and `san` are computed from `mv`, never from
/// the input text. No wire form (this crate has no `serde`, D-52).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedMove {
    pub mv: Move,
    pub uci: String,
    pub san: San,
}

impl Position {
    /// Matches typed or imported move text against the legal moves of this position (SDD
    /// 3.11.1). Never panics: every input returns a move that is in `legal_moves()` or a
    /// `MoveParseError`. `self` is never changed.
    pub fn parse_move(&self, text: &str, mode: ParseMode) -> Result<MatchedMove, MoveParseError> {
        match_move(self, text, mode).map(|(matched, _nag)| matched)
    }
}

/// `parse_move` plus the NAG that a `!` or `?` suffix asks for (Pgn mode hands it to the
/// reader; SDD 3.11.1 step 2).
pub(crate) fn match_move(
    pos: &Position,
    text: &str,
    mode: ParseMode,
) -> Result<(MatchedMove, Option<u8>), MoveParseError> {
    let trimmed = text.trim_matches(|c: char| c.is_ascii_whitespace());
    if trimmed.is_empty() {
        return Err(MoveParseError::Empty);
    }
    if trimmed.len() > MAX_MOVE_TEXT {
        return Err(MoveParseError::TooLong);
    }
    if !trimmed.is_ascii() {
        return Err(MoveParseError::Syntax);
    }
    let (core, nag) = strip_annotations(trimmed);
    if core.is_empty() {
        return Err(MoveParseError::Syntax);
    }
    let mv = resolve(pos, core, mode)?;
    let san = pos
        .san(mv)
        .map_err(|e| MoveParseError::NoMatch { reason: e.reason })?;
    Ok((
        MatchedMove {
            mv,
            uci: mv.to_uci(),
            san,
        },
        nag,
    ))
}

/// Removes trailing `+ # ! ?` (not validated: check and mate follow from the move). Returns the
/// NAG number of a trailing annotation mark, if it is one of the six standard marks.
fn strip_annotations(text: &str) -> (&str, Option<u8>) {
    let core = text.trim_end_matches(['+', '#', '!', '?']);
    let tail = text.get(core.len()..).unwrap_or("");
    let marks: String = tail.chars().filter(|c| matches!(c, '!' | '?')).collect();
    let nag = match marks.as_str() {
        "!" => Some(1),
        "?" => Some(2),
        "!!" => Some(3),
        "??" => Some(4),
        "!?" => Some(5),
        "?!" => Some(6),
        _ => None,
    };
    (core, nag)
}

fn resolve(pos: &Position, core: &str, mode: ParseMode) -> Result<Move, MoveParseError> {
    match core {
        "O-O" | "0-0" => return castle(pos, true),
        "O-O-O" | "0-0-0" => return castle(pos, false),
        _ => {}
    }
    if mode == ParseMode::Typed
        && let Some((from, to, promotion)) = coordinate(core)
    {
        // `B2d5` is both the coordinate move b2d5 and a bishop on the second rank to d5: the
        // upper-case `B` is a bishop when a bishop move matches (Q44), else the coordinate.
        if core.starts_with('B') {
            match resolve_san(pos, core) {
                Err(MoveParseError::NoMatch { .. } | MoveParseError::Syntax) => {}
                bishop => return bishop,
            }
        }
        return resolve_coordinate(pos, from, to, promotion);
    }
    let first = resolve_san(pos, core);
    // The leading `B` rule (Q44, CLI-3): `B` is a bishop when any legal bishop move matches;
    // otherwise, in Typed mode only, it is the b-file.
    if mode == ParseMode::Typed
        && matches!(first, Err(MoveParseError::NoMatch { .. }))
        && let Some(rest) = core.strip_prefix('B')
    {
        match resolve_san(pos, &format!("b{rest}")) {
            Ok(m) => return Ok(m),
            Err(
                e @ (MoveParseError::Ambiguous { .. } | MoveParseError::PromotionRequired { .. }),
            ) => {
                return Err(e);
            }
            Err(_) => {}
        }
    }
    first
}

const PROMOTION_CHOICES: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
];

fn square_at(file: u8, rank: u8) -> Option<Square> {
    Some(Square::new(File::new(file).ok()?, Rank::new(rank).ok()?))
}

fn file_of(byte: u8) -> Option<u8> {
    match byte {
        b'a'..=b'h' => Some(byte - b'a'),
        b'A'..=b'H' => Some(byte - b'A'),
        _ => None,
    }
}

fn rank_of(byte: u8) -> Option<u8> {
    match byte {
        b'1'..=b'8' => Some(byte - b'1'),
        _ => None,
    }
}

fn promotion_kind(byte: u8) -> Option<PieceKind> {
    match byte {
        b'Q' | b'q' => Some(PieceKind::Queen),
        b'R' | b'r' => Some(PieceKind::Rook),
        b'B' | b'b' => Some(PieceKind::Bishop),
        b'N' | b'n' => Some(PieceKind::Knight),
        _ => None,
    }
}

fn find_king(pos: &Position) -> Option<Square> {
    (0..64u8)
        .filter_map(|i| Square::from_index(i).ok())
        .find(|s| {
            pos.piece_at(*s)
                == Some(Piece {
                    side: pos.turn(),
                    kind: PieceKind::King,
                })
        })
}

/// Step 3: the legal king move two squares toward the rook, else `NoMatch` with the reason.
fn castle(pos: &Position, kingside: bool) -> Result<Move, MoveParseError> {
    let other = |reason| MoveParseError::NoMatch { reason };
    let from = find_king(pos).ok_or(other(IllegalReason::Other))?;
    let target_file = if kingside { 6 } else { 2 };
    let to = square_at(target_file, from.rank().index()).ok_or(other(IllegalReason::Other))?;
    let candidate = Move {
        from,
        to,
        promotion: None,
    };
    if pos.is_legal(candidate) {
        Ok(candidate)
    } else {
        Err(other(pos.explain_illegal(from, to, None)))
    }
}

/// Step 4: `[a-hA-H][1-8][a-hA-H][1-8](=?[QRBNqrbn])?`.
fn coordinate(core: &str) -> Option<(Square, Square, Option<PieceKind>)> {
    let b = core.as_bytes();
    let (squares, tail) = b.split_at_checked(4)?;
    let &[f1, r1, f2, r2] = squares else {
        return None;
    };
    let from = square_at(file_of(f1)?, rank_of(r1)?)?;
    let to = square_at(file_of(f2)?, rank_of(r2)?)?;
    let promotion = match tail {
        [] => None,
        [p] | [b'=', p] => Some(promotion_kind(*p)?),
        _ => return None,
    };
    Some((from, to, promotion))
}

fn resolve_coordinate(
    pos: &Position,
    from: Square,
    to: Square,
    promotion: Option<PieceKind>,
) -> Result<Move, MoveParseError> {
    let no_match = |reason| MoveParseError::NoMatch { reason };
    if promotion.is_some() {
        let m = Move {
            from,
            to,
            promotion,
        };
        return if pos.is_legal(m) {
            Ok(m)
        } else {
            Err(no_match(pos.explain_illegal(from, to, promotion)))
        };
    }
    let matching: Vec<Move> = pos
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == from && m.to == to)
        .collect();
    match matching.as_slice() {
        [] => Err(no_match(pos.explain_illegal(from, to, None))),
        [m] => Ok(*m),
        _ => Err(MoveParseError::PromotionRequired {
            choices: PROMOTION_CHOICES.to_vec(),
        }),
    }
}

/// A parsed SAN shape: `[piece][from-file][from-rank][x]to[=promotion]` (step 5).
struct ParsedSan {
    /// `None` is a pawn.
    piece: Option<PieceKind>,
    from_file: Option<u8>,
    from_rank: Option<u8>,
    capture: bool,
    to: Square,
    promotion: Option<PieceKind>,
}

/// A trailing promotion letter with an optional `=` before it. Whether a rank digit stands
/// before it is not checked here: the destination parse that follows needs one.
fn split_promotion(rest: &[u8]) -> (&[u8], Option<PieceKind>) {
    if let Some((&last, init)) = rest.split_last()
        && let Some(kind) = promotion_kind(last)
    {
        return (init.strip_suffix(b"=").unwrap_or(init), Some(kind));
    }
    (rest, None)
}

fn parse_san(core: &str) -> Result<ParsedSan, MoveParseError> {
    let bytes = core.as_bytes();
    let (&first, tail) = bytes.split_first().ok_or(MoveParseError::Syntax)?;
    let (piece, rest) = match first {
        b'K' => (Some(PieceKind::King), tail),
        b'Q' => (Some(PieceKind::Queen), tail),
        b'R' => (Some(PieceKind::Rook), tail),
        b'B' => (Some(PieceKind::Bishop), tail),
        b'N' => (Some(PieceKind::Knight), tail),
        b'k' | b'q' | b'r' | b'n' => {
            return Err(MoveParseError::LowercasePiece {
                suggestion: format!(
                    "{}{}",
                    char::from(first).to_ascii_uppercase(),
                    core.get(1..).unwrap_or("")
                ),
            });
        }
        _ => (None, bytes),
    };
    let (rest, promotion) = split_promotion(rest);
    let split = rest.len().checked_sub(2).ok_or(MoveParseError::Syntax)?;
    let (mut prefix, dest) = rest.split_at_checked(split).ok_or(MoveParseError::Syntax)?;
    let &[df, dr] = dest else {
        return Err(MoveParseError::Syntax);
    };
    let to = file_of(df)
        .zip(rank_of(dr))
        .and_then(|(f, r)| square_at(f, r))
        .ok_or(MoveParseError::Syntax)?;
    let mut from_file = None;
    let mut from_rank = None;
    if let Some((&c, after)) = prefix.split_first()
        && let Some(f) = file_of(c)
    {
        from_file = Some(f);
        prefix = after;
    }
    if let Some((&c, after)) = prefix.split_first()
        && let Some(r) = rank_of(c)
    {
        from_rank = Some(r);
        prefix = after;
    }
    let mut capture = false;
    if let Some((&b'x', after)) = prefix.split_first() {
        capture = true;
        prefix = after;
    }
    if !prefix.is_empty() {
        return Err(MoveParseError::Syntax);
    }
    Ok(ParsedSan {
        piece,
        from_file,
        from_rank,
        capture,
        to,
        promotion,
    })
}

fn is_capture(pos: &Position, m: Move) -> bool {
    pos.piece_at(m.to).is_some()
        || pos
            .piece_at(m.from)
            .is_some_and(|p| p.kind == PieceKind::Pawn && m.from.file() != m.to.file())
}

/// Steps 5, 7 and 8.
fn resolve_san(pos: &Position, core: &str) -> Result<Move, MoveParseError> {
    let parsed = parse_san(core)?;
    let kind = parsed.piece.unwrap_or(PieceKind::Pawn);
    let mut found: Vec<Move> = pos
        .legal_moves()
        .into_iter()
        .filter(|m| {
            pos.piece_at(m.from).is_some_and(|p| p.kind == kind)
                && m.to == parsed.to
                && parsed.from_file.is_none_or(|f| m.from.file().index() == f)
                && parsed.from_rank.is_none_or(|r| m.from.rank().index() == r)
                && (parsed.promotion.is_none() || m.promotion == parsed.promotion)
                && (!parsed.capture || is_capture(pos, *m))
                // A pawn move with neither an origin file nor an `x` is a push.
                && (kind != PieceKind::Pawn
                    || parsed.capture
                    || parsed.from_file.is_some()
                    || m.from.file() == m.to.file())
        })
        .collect();
    if found.is_empty() {
        return Err(MoveParseError::NoMatch {
            reason: nearest_reason(pos, &parsed, kind),
        });
    }
    if parsed.promotion.is_none() && found.iter().all(|m| m.promotion.is_some()) {
        return Err(MoveParseError::PromotionRequired {
            choices: PROMOTION_CHOICES.to_vec(),
        });
    }
    found.sort_by_key(Move::to_uci);
    match found.as_slice() {
        [only] => Ok(*only),
        _ => {
            let candidates = found
                .iter()
                .map(|m| pos.san(*m))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MoveParseError::NoMatch { reason: e.reason })?;
            Err(MoveParseError::Ambiguous { candidates })
        }
    }
}

/// The reason for "no match" (SDD 3.11.1 step 8): the first informative `explain_illegal` over
/// the side's pieces of that kind that fit the given origin hints, else "no <piece> can reach".
/// `Other` (the move is legal but contradicts the `x` or promotion in the text) reads as
/// "cannot reach" too, so a defect indicator is never shown as a rule.
fn nearest_reason(pos: &Position, parsed: &ParsedSan, kind: PieceKind) -> IllegalReason {
    let side = pos.turn();
    (0..64u8)
        .filter_map(|i| Square::from_index(i).ok())
        .filter(|s| pos.piece_at(*s) == Some(Piece { side, kind }))
        .filter(|s| parsed.from_file.is_none_or(|f| s.file().index() == f))
        .filter(|s| parsed.from_rank.is_none_or(|r| s.rank().index() == r))
        .map(|s| pos.explain_illegal(s, parsed.to, parsed.promotion))
        .find(|r| !matches!(r, IllegalReason::CannotReach { .. } | IllegalReason::Other))
        .unwrap_or(IllegalReason::CannotReach { piece: kind })
}
