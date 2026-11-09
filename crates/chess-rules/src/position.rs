//! DD-RUL-002, 003, 004, 007: `Position` (immutable, legal move generation, `play`), the exact
//! `PositionKey`, the position predicates and the illegal-move explainer. This module and `fen`
//! are the only ones that import `shakmaty`; every upstream value is converted at the boundary.

use shakmaty::san::SanPlus;
use shakmaty::{
    CastlingSide, Chess, Color, EnPassantMode, Position as _, Role, Square as ShakSquare,
};

use crate::error::{IllegalMove, IllegalReason};
use crate::fen::Fen;
use crate::notation::San;
use crate::types::{Move, Piece, PieceKind, Side, Square};

/// A chess position. `play` returns a new value and never changes `self`.
#[derive(Debug, Clone)]
pub struct Position {
    inner: Chess,
}

// ---------------------------------------------------------------- boundary conversions

fn side_from(color: Color) -> Side {
    match color {
        Color::White => Side::White,
        Color::Black => Side::Black,
    }
}

fn kind_from(role: Role) -> PieceKind {
    match role {
        Role::Pawn => PieceKind::Pawn,
        Role::Knight => PieceKind::Knight,
        Role::Bishop => PieceKind::Bishop,
        Role::Rook => PieceKind::Rook,
        Role::Queen => PieceKind::Queen,
        Role::King => PieceKind::King,
    }
}

fn square_from(sq: ShakSquare) -> Option<Square> {
    u8::try_from(sq.to_u32())
        .ok()
        .and_then(|i| Square::from_index(i).ok())
}

fn square_to(sq: Square) -> ShakSquare {
    // Invariant: a `Square` is always below 64 (checked constructors), so `new` cannot panic.
    ShakSquare::new(u32::from(sq.index()))
}

/// Converts an upstream move to ours. Castling becomes the king's two-square move. `None` only
/// for a drop (never generated for standard chess) or an off-board offset (impossible).
fn move_from(m: shakmaty::Move) -> Option<Move> {
    match m {
        shakmaty::Move::Normal {
            from,
            to,
            promotion,
            ..
        } => Some(Move {
            from: square_from(from)?,
            to: square_from(to)?,
            promotion: promotion.map(kind_from),
        }),
        shakmaty::Move::EnPassant { from, to } => Some(Move {
            from: square_from(from)?,
            to: square_from(to)?,
            promotion: None,
        }),
        shakmaty::Move::Castle { king, rook } => {
            let delta = if rook.file() > king.file() { 2 } else { -2 };
            Some(Move {
                from: square_from(king)?,
                to: square_from(king.offset(delta)?)?,
                promotion: None,
            })
        }
        shakmaty::Move::Put { .. } => None,
    }
}

/// The square `dx` files and `dy` ranks away, if it is on the board.
fn offset(sq: Square, dx: i16, dy: i16) -> Option<Square> {
    let file = i16::from(sq.file().index()) + dx;
    let rank = i16::from(sq.rank().index()) + dy;
    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
        return None;
    }
    Square::from_index(u8::try_from(file + 8 * rank).ok()?).ok()
}

// ---------------------------------------------------------------- Position

impl Position {
    /// The standard start position.
    pub fn standard() -> Position {
        Position {
            inner: Chess::default(),
        }
    }

    pub(crate) fn from_chess(inner: Chess) -> Position {
        Position { inner }
    }

    pub(crate) fn chess(&self) -> &Chess {
        &self.inner
    }

    /// Infallible: a `Fen` is already validated.
    pub fn from_fen(fen: &Fen) -> Position {
        fen.position().clone()
    }

    pub fn to_fen(&self) -> Fen {
        Fen::from_position(self)
    }

    pub fn turn(&self) -> Side {
        side_from(self.inner.turn())
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.inner
            .board()
            .piece_at(square_to(square))
            .map(|p| Piece {
                side: side_from(p.color),
                kind: kind_from(p.role),
            })
    }

    pub fn fullmove_number(&self) -> u32 {
        self.inner.fullmoves().get()
    }

    /// Plies since the last capture or pawn move.
    pub fn halfmove_clock(&self) -> u32 {
        self.inner.halfmoves()
    }

    /// All legal moves, generation order unspecified (at most 218). Castling is the king's
    /// two-square move.
    pub fn legal_moves(&self) -> Vec<Move> {
        self.inner
            .legal_moves()
            .iter()
            .filter_map(|m| move_from(*m))
            .collect()
    }

    /// The upstream move that corresponds to `m`, if `m` is legal: one definition of "the same
    /// move" by value (from, to, promotion), DD-RUL-002.
    fn find(&self, m: Move) -> Option<shakmaty::Move> {
        self.inner
            .legal_moves()
            .into_iter()
            .find(|candidate| move_from(*candidate) == Some(m))
    }

    pub fn is_legal(&self, m: Move) -> bool {
        self.find(m).is_some()
    }

    /// `Ok` iff `m` is in `legal_moves()`; otherwise `IllegalMove` with the reason from
    /// `explain_illegal`. `self` is never changed.
    pub fn play(&self, m: Move) -> Result<Position, IllegalMove> {
        match self.find(m) {
            Some(upstream) => {
                let mut next = self.inner.clone();
                next.play_unchecked(upstream);
                Ok(Position { inner: next })
            }
            None => Err(IllegalMove {
                reason: self.explain_illegal(m.from, m.to, m.promotion),
            }),
        }
    }

    /// Canonical SAN of `m` with the `+` or `#` suffix (DD-RUL-005, IFP-RUL-1): piece letter,
    /// minimal disambiguation, `x`, destination, `=Q`, castling as `O-O` and `O-O-O` with the
    /// letter O. Formatted by the upstream writer after the same lookup as `play`.
    pub fn san(&self, m: Move) -> Result<San, IllegalMove> {
        match self.find(m) {
            Some(upstream) => Ok(San::new(
                SanPlus::from_move(self.inner.clone(), upstream).to_string(),
            )),
            None => Err(IllegalMove {
                reason: self.explain_illegal(m.from, m.to, m.promotion),
            }),
        }
    }

    // ------------------------------------------------------------ predicates (DD-RUL-004)

    pub fn is_check(&self) -> bool {
        self.inner.is_check()
    }

    pub fn is_checkmate(&self) -> bool {
        self.inner.is_checkmate()
    }

    pub fn is_stalemate(&self) -> bool {
        self.inner.is_stalemate()
    }

    /// `false` only if no sequence of legal moves lets `side` deliver mate, judged by material
    /// alone (RULE-1 (c), RULE-4). Uses the upstream test, which the fixture table of SDD 3.11.3
    /// confirms (ASM-RUL-001). A blocked-pawn dead position is deliberately not detected.
    pub fn can_side_checkmate(&self, side: Side) -> bool {
        let color = match side {
            Side::White => Color::White,
            Side::Black => Color::Black,
        };
        !self.inner.has_insufficient_material(color)
    }

    /// Neither side can checkmate by any legal sequence because of the material left.
    pub fn is_dead_by_material(&self) -> bool {
        !self.can_side_checkmate(Side::White) && !self.can_side_checkmate(Side::Black)
    }

    /// FIDE "same position" identity: pieces, side to move, castling rights, and the en passant
    /// target only when a legal en passant capture exists (SDD 3.11.4). Exact, not a hash.
    pub fn repetition_key(&self) -> PositionKey {
        let placement = core::array::from_fn(|i| {
            u8::try_from(i)
                .ok()
                .and_then(|i| Square::from_index(i).ok())
                .and_then(|s| self.piece_at(s))
        });
        let castles = self.inner.castles();
        let rights = |color, side| castles.has(color, side);
        PositionKey {
            placement,
            side: self.turn(),
            castling: [
                rights(Color::White, CastlingSide::KingSide),
                rights(Color::White, CastlingSide::QueenSide),
                rights(Color::Black, CastlingSide::KingSide),
                rights(Color::Black, CastlingSide::QueenSide),
            ],
            en_passant: self
                .inner
                .ep_square(EnPassantMode::Legal)
                .and_then(square_from),
        }
    }

    // ------------------------------------------------------------ explainer (DD-RUL-007)

    /// Classifies, in the order of SDD 3.11.2, why moving the piece on `from` to `to` is not
    /// legal. Returns `Other` only when none applies, which includes a move that is in fact legal
    /// (a defect indicator, never shown as a rule); a promotion piece on a move that does not
    /// promote, or a promotion to a king or pawn, is `CannotReach` (RF-06).
    pub fn explain_illegal(
        &self,
        from: Square,
        to: Square,
        promotion: Option<PieceKind>,
    ) -> IllegalReason {
        let Some(piece) = self.piece_at(from) else {
            return IllegalReason::NoPiece;
        };
        if piece.side != self.turn() {
            return IllegalReason::WrongTurn;
        }
        if piece.kind == PieceKind::King
            && let Some(reason) = self.explain_castling(piece.side, from, to)
        {
            return reason;
        }
        if let Some(reason) = self.explain_geometry(piece, from, to) {
            return reason;
        }
        let last_rank = if piece.side == Side::White { 7 } else { 0 };
        let promotes = piece.kind == PieceKind::Pawn && to.rank().index() == last_rank;
        if promotes && promotion.is_none() {
            return IllegalReason::PromotionRequired;
        }
        // RF-06: a promotion piece on a move that does not promote, or a promotion to a king or
        // pawn, is a wrong request, not a library defect: it gets a stable reason of its own
        // and `Other` stays the defect indicator (SDD 3.11.2).
        if let Some(chosen) = promotion {
            if matches!(chosen, PieceKind::Pawn | PieceKind::King) {
                return IllegalReason::CannotReach { piece: chosen };
            }
            if !promotes {
                return IllegalReason::CannotReach { piece: piece.kind };
            }
        }
        if self.is_legal(Move {
            from,
            to,
            promotion,
        }) {
            return IllegalReason::Other;
        }
        if self.is_check() {
            IllegalReason::KingInCheck
        } else {
            IllegalReason::LeavesKingInCheck
        }
    }

    /// `Some(reason)` when the king move is a castling attempt (two files along its home rank
    /// from the e-file); `Some(Other)` when the castling conditions all hold.
    fn explain_castling(&self, side: Side, from: Square, to: Square) -> Option<IllegalReason> {
        let home_rank = if side == Side::White { 0 } else { 7 };
        if from.file().index() != 4
            || from.rank().index() != home_rank
            || to.rank() != from.rank()
            || to.file().index().abs_diff(from.file().index()) != 2
        {
            return None;
        }
        let (color, opponent) = match side {
            Side::White => (Color::White, Color::Black),
            Side::Black => (Color::Black, Color::White),
        };
        let kingside = to.file() > from.file();
        let castling_side = if kingside {
            CastlingSide::KingSide
        } else {
            CastlingSide::QueenSide
        };
        let Some(rook) = self
            .inner
            .castles()
            .rook(color, castling_side)
            .and_then(square_from)
        else {
            return Some(IllegalReason::CastlingRightsLost);
        };
        let step: i16 = if kingside { 1 } else { -1 };
        let between = (1..i16::from(from.file().index().abs_diff(rook.file().index())))
            .filter_map(|k| offset(from, step * k, 0));
        if between.into_iter().any(|s| self.piece_at(s).is_some()) {
            return Some(IllegalReason::CastlingBlocked);
        }
        if self.is_check() {
            return Some(IllegalReason::CastlingOutOfCheck);
        }
        let board = self.inner.board();
        let attacked = |s: Square| {
            board
                .attacks_to(square_to(s), opponent, board.occupied())
                .any()
        };
        let crossed = [offset(from, step, 0), offset(from, 2 * step, 0)];
        if crossed.into_iter().flatten().any(attacked) {
            return Some(IllegalReason::CastlingThroughCheck);
        }
        Some(IllegalReason::Other)
    }

    /// `Some(reason)` when the move is impossible for the piece ignoring check: wrong shape
    /// (`CannotReach`), blocked path (`Blocked`) or an own piece on the target.
    fn explain_geometry(&self, piece: Piece, from: Square, to: Square) -> Option<IllegalReason> {
        let dx = i16::from(to.file().index()) - i16::from(from.file().index());
        let dy = i16::from(to.rank().index()) - i16::from(from.rank().index());
        let cannot = Some(IllegalReason::CannotReach { piece: piece.kind });
        if dx == 0 && dy == 0 {
            return cannot;
        }
        let target = self.piece_at(to);
        let own_target = target.is_some_and(|t| t.side == piece.side);
        let path_blocked = |sx: i16, sy: i16, n: i16| {
            (1..n).any(|k| offset(from, sx * k, sy * k).is_some_and(|s| self.piece_at(s).is_some()))
        };
        match piece.kind {
            PieceKind::Knight => {
                if !matches!((dx.abs(), dy.abs()), (1, 2) | (2, 1)) {
                    return cannot;
                }
            }
            PieceKind::King => {
                if dx.abs().max(dy.abs()) != 1 {
                    return cannot;
                }
            }
            PieceKind::Bishop | PieceKind::Rook | PieceKind::Queen => {
                let diagonal = dx.abs() == dy.abs();
                let straight = dx == 0 || dy == 0;
                let shape_ok = match piece.kind {
                    PieceKind::Bishop => diagonal,
                    PieceKind::Rook => straight,
                    _ => diagonal || straight,
                };
                if !shape_ok {
                    return cannot;
                }
                if path_blocked(dx.signum(), dy.signum(), dx.abs().max(dy.abs())) {
                    return Some(IllegalReason::Blocked);
                }
            }
            PieceKind::Pawn => {
                let forward: i16 = if piece.side == Side::White { 1 } else { -1 };
                let start_rank = if piece.side == Side::White { 1 } else { 6 };
                if dx == 0 && dy == forward {
                    return if target.is_some() {
                        Some(IllegalReason::Blocked)
                    } else {
                        None
                    };
                }
                if dx == 0 && dy == 2 * forward && from.rank().index() == start_rank {
                    return if target.is_some() || path_blocked(0, forward, 2) {
                        Some(IllegalReason::Blocked)
                    } else {
                        None
                    };
                }
                if dx.abs() == 1 && dy == forward {
                    if own_target {
                        return Some(IllegalReason::OwnPieceOnTarget);
                    }
                    let en_passant = self
                        .inner
                        .maybe_ep_square()
                        .and_then(square_from)
                        .is_some_and(|ep| ep == to);
                    return if target.is_some() || en_passant {
                        None
                    } else {
                        cannot
                    };
                }
                return cannot;
            }
        }
        own_target.then_some(IllegalReason::OwnPieceOnTarget)
    }
}

// ---------------------------------------------------------------- PositionKey

/// Exact repetition identity and ECO lookup key (DD-RUL-003). Not a 64-bit hash: a collision
/// would end a game as a false draw (DR-RUL-003).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PositionKey {
    placement: [Option<Piece>; 64],
    side: Side,
    /// White king side, white queen side, black king side, black queen side.
    castling: [bool; 4],
    en_passant: Option<Square>,
}

impl PositionKey {
    /// The first four FEN fields: placement, side, castling, legal en passant target.
    pub fn to_epd(&self) -> String {
        crate::key_derive::epd(
            |i| {
                self.placement
                    .get(usize::from(i))
                    .copied()
                    .flatten()
                    .map(|p| (p.side == Side::White, p.kind.letter()))
            },
            self.side == Side::White,
            self.castling,
            self.en_passant.map(Square::index),
        )
    }
}
