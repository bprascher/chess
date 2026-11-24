//! chess-store: everything on disk (SDD-chess-store 1.0). **Part A** (chess-a0r.3.6): `DataDir`,
//! the schema and migrations with the pre-upgrade backup, the open sequence up to the upgrade,
//! players and games access, the UUID-only `Store` handle and `roster_indexes()`.
//!
//! Not here yet: the move journal, leases, process registry, `apply_records` and the finalize,
//! abort, void and reset transactions (part B, chess-a0r.3.7 and following); automatic backup,
//! restore, row IO and `FaultPlan` (part C, chess-a0r.3.8). Module seams for them are named in
//! the modules that will grow: `thread` (apply backlog), `open` (registration, recovery),
//! `migrate` (the exclusive guard), `backup` (`BackupManager`), `repo` (new repo files).
//!
//! Layering (SDD 3.8): `repo::*` know nothing of threads; `thread` and `store` know no SQL text.
//! No integer row key crosses the public API (D-40). No `unsafe`, no `libc`, no `rustix` (D-107).
#![forbid(unsafe_code)]
// With the feature off the fault module is not part of the public surface (part D1, acceptance
// (1)). This doctest must fail to compile; it exists only in the feature-off build.
#![cfg_attr(
    not(feature = "test-support"),
    doc = "\n```compile_fail,E0432\nuse chess_store::fault::FaultPlan;\n```\n"
)]
// FIND-D5-02 B (VAR-12): the opaque variation handle is gone; a variation is `(game, from_ply, pgn)`.
#![doc = "\n```compile_fail,E0432\nuse chess_store::VariationRef;\n```\n"]

mod api;
mod api_analysis;
mod backlog;
mod backup;
mod datadir;
mod error;
#[cfg(not(feature = "test-support"))]
mod fault;
pub mod journal;
mod migrate;
mod open;
mod proc;
mod readpool;
mod repo;
mod store;
mod thread;

/// DD-ST-024: failpoints and `FaultPlan` for failure-injection tests (feature `test-support`).
#[cfg(feature = "test-support")]
pub mod fault;

pub use api::{
    AbortRequest, ApplyOutcome, BacklogStatus, ComputerProfile, EndOutcome, FinalizeOutcome,
    FinalizeRequest, GameFilter, GameRecord, LeaseBeat, LeaseInfo, LiveProcess, MoveRow,
    NewComputer, NewGame, NewPlayer, OpenReport, ParkCause, ParkedGame, PlayerRecord,
    ProcessIdentity, RatingChange, RatingViolation, RatingViolationKind, VoidSpec,
};
pub use api_analysis::{
    EvalFlag, GameAnalysis, JobState, MAX_VARIATION_PGN_BYTES, MoveEvalRow, StatsGameRow,
    StatsSide, VariationRecord,
};
pub use backlog::{MAX_BATCHES_PER_GAME, MAX_PENDING_BATCHES};
pub use datadir::{DIR_MODE, DataDir, EnsureReport, FILE_MODE, StoreId};
pub use error::{
    BackupCause, Entity, ExclusiveReason, JournalError, StoreError, UnavailableCause, WriteCause,
};
pub use migrate::{MIGRATIONS, MigrateOutcome, Migration, Migrator};
pub use repo::coord::{
    ANALYSIS_LEASE, LEASE_BEAT_EVERY, PROCESS_HEARTBEAT_EVERY, PROCESS_STALE_AFTER,
};
pub use store::{OpenedStore, Store, StoreOptions, WallClock};
#[cfg(feature = "test-support")]
pub use thread::MonotonicClock;
