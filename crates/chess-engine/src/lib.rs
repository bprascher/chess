//! chess-engine: the Stockfish UCI process wrapper (SDD-chess-engine 1.0).
//!
//! The crate talks to one child process per instance over UCI, cancels and times out searches,
//! recovers from engine failure, plans the personality into `setoption` lines, picks the seeded
//! "random error" move, and supplies deterministic fakes for tests. It decides nothing about
//! legality, strictness or when to cancel (that is `chess-app`).
#![forbid(unsafe_code)]

mod arbiter;
mod config;
mod driver;
mod engine;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod locate;
mod pinned;
mod plan;
mod pool;
mod select;
mod spawn;
mod types;
pub mod uci;

pub use arbiter::AnalysisPriority;
pub use config::EngineConfig;
pub use driver::{EngineInfo, HandshakeReport, UciEngine};
pub use engine::Engine;
pub use error::{EngineError, ErrorClass};
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeBehavior, FakeCall, FakeEngine, FakeReply};
pub use locate::{BinaryLocator, BinarySource, LocatedBinary};
pub use pinned::{EloRange, PINNED_UCI_ELO_RANGE};
pub use plan::{InstanceProfile, OptionPlan, PlanReport};
pub use pool::{ComputerKey, EngineHandle, EnginePool, GameKey};
pub use select::{MoveChoice, MoveKind, SplitMix64, clock_budget, select_move};
pub use spawn::{BoxFuture, ExitInfo, ProcessControl, ProcessSpawner, SpawnedProcess, Spawner};
pub use types::{
    EnginePersonality, Health, PvLine, Score, SearchInfo, SearchLimits, SearchPosition,
    SearchRequest, SearchResult, UciMove,
};
