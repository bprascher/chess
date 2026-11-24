//! DD-EN-002: request and result types. Constructors validate; plain data otherwise.

use std::fmt;
use std::time::Duration;

use chess_rules::Fen;
use tokio::sync::mpsc;

use crate::error::EngineError;

/// A syntactically valid UCI move: file rank file rank, then optionally one of `qrbn`.
/// Not necessarily legal (DR-EN-003): legality is the gateway's check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UciMove(String);

impl UciMove {
    pub fn parse(text: &str) -> Result<UciMove, EngineError> {
        let bytes = text.as_bytes();
        let ok = matches!(bytes.len(), 4 | 5)
            && bytes.iter().enumerate().all(|(i, b)| match i {
                0 | 2 => (b'a'..=b'h').contains(b),
                1 | 3 => (b'1'..=b'8').contains(b),
                _ => matches!(b, b'q' | b'r' | b'b' | b'n'),
            });
        if ok {
            Ok(UciMove(text.to_string()))
        } else {
            Err(EngineError::BadOutput)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UciMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// From the side to move's point of view, as UCI reports it (`chess-app` normalises).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    Cp(i32),
    Mate(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvLine {
    pub multipv: u8,
    pub score: Score,
    pub depth: u32,
    pub pv: Vec<UciMove>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub best: UciMove,
    pub pv: Vec<UciMove>,
    /// `Cp(0)` with `depth == 0` means the engine sent no `info` line before `bestmove`.
    pub score: Score,
    pub depth: u32,
    /// All MultiPV lines of the last completed depth (additive, AMD-3).
    pub lines: Vec<PvLine>,
    pub stopped_early: bool,
    pub elapsed: Duration,
}

/// A throttled progress report (`SearchRequest.updates`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchInfo {
    pub depth: u32,
    pub multipv: u8,
    pub score: Score,
    pub pv: Vec<UciMove>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchLimits {
    /// Mandatory (DR-EN-005): the basis of the ENG-4 timers.
    pub movetime: Duration,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
}

impl SearchLimits {
    /// Longest accepted `movetime`: one hour. A RULED value (Brian Prascher, 2026-11-10,
    /// chess-a0r.3.42), no longer an estimate. Anything longer is refused with
    /// `InvalidRequest` instead of overflowing the timer arithmetic of the driver (EF-02). A
    /// caller that wants "until the user leaves" passes this value and stops the search itself.
    pub const MAX_MOVETIME: Duration = Duration::from_secs(60 * 60);

    pub fn new(
        movetime: Duration,
        depth: Option<u32>,
        nodes: Option<u64>,
    ) -> Result<SearchLimits, EngineError> {
        let limits = SearchLimits {
            movetime,
            depth,
            nodes,
        };
        limits.validate()?;
        Ok(limits)
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        let bad = |reason| Err(EngineError::InvalidRequest { reason });
        if self.movetime.is_zero() {
            return bad("movetime must be greater than zero");
        }
        if self.movetime > Self::MAX_MOVETIME {
            return bad("movetime exceeds the maximum");
        }
        if self.depth == Some(0) {
            return bad("depth must be greater than zero");
        }
        if self.nodes == Some(0) {
            return bad("nodes must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchPosition {
    Fen(Fen),
    /// `start: None` is the standard start position.
    Moves {
        start: Option<Fen>,
        moves: Vec<UciMove>,
    },
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub position: SearchPosition,
    pub limits: SearchLimits,
    pub multipv: u8,
    pub new_game: bool,
    /// Error-pick seed from `chess_domain::error_pick_seed`; used by `select_move` only.
    pub seed: u64,
    pub updates: Option<mpsc::Sender<SearchInfo>>,
}

impl SearchRequest {
    /// Most moves a `SearchPosition::Moves` may carry (ESTIMATE, EF-07): a position line of
    /// that many moves is about 50 KB, below one pipe buffer, so writing it cannot block on a
    /// healthy engine.
    pub const MAX_MOVES: usize = 10_000;

    /// A single-line request from a position, for callers and tests.
    pub fn new(position: SearchPosition, limits: SearchLimits) -> SearchRequest {
        SearchRequest {
            position,
            limits,
            multipv: 1,
            new_game: false,
            seed: 0,
            updates: None,
        }
    }

    pub fn validate(&self, multipv_cap: u8) -> Result<(), EngineError> {
        self.limits.validate()?;
        if let SearchPosition::Moves { moves, .. } = &self.position
            && moves.len() > Self::MAX_MOVES
        {
            return Err(EngineError::InvalidRequest {
                reason: "too many moves in the position",
            });
        }
        if self.multipv == 0 || self.multipv > multipv_cap {
            return Err(EngineError::InvalidRequest {
                reason: "multipv out of range",
            });
        }
        Ok(())
    }
}

/// Fields of architecture 4.6 `Personality`. `error_rate` is permille / 1000.0, set by
/// `personality_to_engine` in `chess-app` (D-43). Input data: never mutated here (PLY-4).
#[derive(Debug, Clone, PartialEq)]
pub struct EnginePersonality {
    pub skill_level: u8,
    pub depth_limit: u32,
    pub movetime_ms: u32,
    pub uci_elo: Option<u32>,
    pub error_rate: f64,
    pub error_multipv: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    NotStarted,
    Ready,
    Searching,
    Dead,
}
