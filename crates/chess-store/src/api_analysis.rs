//! DTOs of part D5 (SDD 3.7.2: Statistics, Analysis, Jobs). Every identity is a `PlayerId` or
//! `GameId` (D-40). Nothing here is
//! serialized to a front-end and nothing derives `ts-rs`.
//!
//! **Why `StatsGameRow` and not `chess_domain::StatsRow`:** `StatsRow.color` is a
//! `chess_rules::Side`, and `chess-store` has no edge to `chess-rules` (architecture 3.2,
//! `ci/check-crate-edges.mjs`). The store therefore returns this row, which has the same fields
//! with the store's own `StatsSide`; `chess-app` (which depends on both crates) maps it to
//! `StatsRow` field by field. Evidence finding FIND-D5-01.

use chess_domain::{GameId, Mode, OpponentRef, ResultForViewed, WallMs};

/// The largest `pgn` text `save_variation` stores, in bytes: 1 MiB (ESTIMATE, FIND-D5R-01; a
/// real line is a few hundred bytes, and the app's own import limit is 10 MB for a whole game
/// file). Above it the store answers `StoreError::VariationTooLarge`. The check is on the byte
/// length of the text as given; a text of exactly this length is stored.
pub const MAX_VARIATION_PGN_BYTES: usize = 1024 * 1024;

/// A seat of the viewed Player in a statistics row (the store's own type, see the module doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsSide {
    White,
    Black,
}

impl StatsSide {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            StatsSide::White => "white",
            StatsSide::Black => "black",
        }
    }
}

/// One game as the row queries return it: the fields of `chess_domain::StatsRow`, read from
/// `GAME`, with no counting predicate applied (D-44). `status` is `GameStatus`, so voided,
/// aborted and unfinished games appear, and so do self-play and custom-start games.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsGameRow {
    pub status: chess_domain::GameStatus,
    pub self_play: bool,
    pub custom_start: bool,
    pub mode: Mode,
    /// The seat the viewed Player had (the requested side when a side was given; `White` for a
    /// self-play game when no side was given).
    pub color: StatsSide,
    /// `None` for a game without a result (aborted, voided, unfinished).
    pub result_for_viewed: Option<ResultForViewed>,
    pub started_at: WallMs,
    /// Stored `MOVE` rows of the game.
    pub plies: u32,
    /// `total_active_ms`; 0 when the column is `NULL` (a game that never completed).
    pub active_ms: u64,
    /// `category_now` is the Strict-rating band of a computer, `None` for a human.
    pub opponent: OpponentRef,
}

/// `MOVE_EVAL.flag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalFlag {
    Blunder,
    Mistake,
    Inaccuracy,
}

impl EvalFlag {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            EvalFlag::Blunder => "blunder",
            EvalFlag::Mistake => "mistake",
            EvalFlag::Inaccuracy => "inaccuracy",
        }
    }
}

impl std::str::FromStr for EvalFlag {
    type Err = ();
    fn from_str(text: &str) -> Result<EvalFlag, ()> {
        match text {
            "blunder" => Ok(EvalFlag::Blunder),
            "mistake" => Ok(EvalFlag::Mistake),
            "inaccuracy" => Ok(EvalFlag::Inaccuracy),
            _ => Err(()),
        }
    }
}

/// One `MOVE_EVAL` row. `cp` and `mate` are from **White's point of view** (decision 12,
/// CR-9.3) and at most one is set; `cp_loss` is from the **mover's** point of view (ANA-3).
/// `ply` is the 1-based `MOVE.ply` the evaluation follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveEvalRow {
    pub game: GameId,
    pub ply: u32,
    pub cp: Option<i32>,
    pub mate: Option<i32>,
    pub best_uci: Option<String>,
    pub cp_loss: Option<i32>,
    pub flag: Option<EvalFlag>,
    pub depth: Option<u32>,
    pub engine_version: String,
}

/// `ANALYSIS_JOB.state` (STATE-ST-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
}

impl JobState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Done => "done",
            JobState::Failed => "failed",
        }
    }
}

impl std::str::FromStr for JobState {
    type Err = ();
    fn from_str(text: &str) -> Result<JobState, ()> {
        match text {
            "queued" => Ok(JobState::Queued),
            "running" => Ok(JobState::Running),
            "done" => Ok(JobState::Done),
            "failed" => Ok(JobState::Failed),
            _ => Err(()),
        }
    }
}

/// What `get_game_analysis` returns (architecture 5.1 `GetGameAnalysis`): the stored
/// evaluations in ply order, the job state (`None` when no job exists), the number of plies
/// that have an evaluation (`evaluated`) and the number of stored moves (`total`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAnalysis {
    pub evals: Vec<MoveEvalRow>,
    pub job: Option<JobState>,
    pub evaluated: u32,
    pub total: u32,
}

/// One saved variation (decision 13, CR-9.4): one line per row, no Player. A variation is
/// identified by `(game, from_ply, pgn)` (FIND-D5-02 B, Brian Prascher 2026-11-22): there is no
/// handle type, `save_variation` is idempotent on that key and `delete_variation` takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariationRecord {
    pub game: GameId,
    /// Main-line plies played before the line's first move.
    pub from_ply: u32,
    pub pgn: String,
    pub created_at: WallMs,
}
