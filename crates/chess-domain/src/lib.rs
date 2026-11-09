//! chess-domain: the pure decision core (SDD-chess-domain 1.0). Part A (chess-a0r.2.6):
//! identity newtypes, `Fact` vocabulary, the `Game` state machine and replay, mode `Policy`,
//! `Clock` and `TimeTotals`, Elo. Part B (chess-a0r.2.7) adds `BoardGrid`, `RosterGen`,
//! `SplitMix64`, the ECO import functions, stats and `VariationTree`.
//!
//! The crate does no I/O and reads no clock or random source: time arrives as `Stamp`
//! arguments, randomness through the `Rng` trait. It re-exports nothing from `chess-rules`
//! (SDD 3.8), except that the public API necessarily names rules types (`Side`, `Move`,
//! `Position`, `Fen`), which callers import from `chess-rules` themselves.
#![forbid(unsafe_code)]

mod board_grid;
mod clock;
mod eco;
mod edge;
mod elo;
mod error;
mod game;
mod policy;
mod roster;
mod stats;
mod totals;
mod types;
mod variation;

pub use board_grid::{
    BoardGrid, BoardStyle, BoardView, Cell, Highlight, Orientation, Shade, render_text,
};
pub use clock::{Clock, ClockSpec, ClockSync, MoveTiming};
pub use eco::{
    EcoChoice, EcoCode, EcoFields, EcoImportOutcome, EcoMatch, EcoTable, classify_import,
    normalize_eco_tag, resolve,
};
#[cfg(any(test, feature = "test-support"))]
pub use edge::{FakeTimeSource, ScriptedRng};
pub use edge::{Rng, SplitMix64, TimeSource, error_pick_seed};
pub use elo::{
    EloCalc, GameFacts, RatedSide, RatingTrack, Settlement, SideUpdate, UnratedReason,
    round_half_away,
};
pub use error::{
    BoardError, ClockSpecError, Denied, DeniedReason, EcoUnanswered, EloError, GameError,
    HistoryError, IdError, MoveDelayError, ParseTextError, PersonalityError, ReplayError,
    RosterError, SetupError, SetupField, SetupReason, TreeError,
};
pub use game::{
    Fact, Game, GameCommand, GameView, MAX_PLIES, MoveCheck, PlyRecord, ReplayItem, ReplayOutcome,
    ReplayWarning, Step,
};
pub use policy::{
    Action, ActionKind, ColorChoice, GameContext, GameSetup, MoveDelay, PlayerAction,
    PlayerContext, Policy, StartPosition, assign_colors,
};
pub use roster::{
    CLASS_BOUNDS, ClassBounds, ComputerPlayerSpec, FAMILY_NAMES, GIVEN_NAMES, MAX_ROSTER_INDEX,
    Personality, ROSTER_NAMESPACE, RosterGen, RosterSeed, Span, UciEloRange, roster_uuid,
};
pub use stats::{
    OpponentRef, PlayerStats, RatingPoint, ResultForViewed, StatsRow, Tally, aggregate,
    check_history, counts_in_aggregates, rating_series,
};
pub use totals::{Accrual, TimeTotals, time_accrual};
pub use types::{
    Actor, Category, ClockValues, DropCause, EndReason, GameId, GameResult, GameState, GameStatus,
    Matchup, Mode, MonoMs, Outcome, PlayerId, PlayerKind, Seat, Stamp, SuspendCause, Track,
    VoidReason, WallMs,
};
pub use uuid::Uuid;
pub use variation::{MAX_NODES, NodeId, SavedLine, VariationTree};
