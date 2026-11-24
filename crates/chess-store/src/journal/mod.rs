//! The per-game move journal (SDD 3.6, 3.7.3, 3.11.1 to 3.11.3; DD-ST-015 to DD-ST-018).
pub mod files;
pub mod format;
pub mod reader;
pub mod rebase;
pub mod reconcile;
pub mod recover;
pub mod writer;

pub use files::{CloseRepair, MovedAside, Settled, move_aside_damaged, settle_rebase_files};
pub use format::{JournalRecord, Kind, Payload};
pub use reader::{DamageKind, DamagedInfo, Scan, scan_bytes, scan_file};
pub use rebase::{
    RebaseError, RebaseFile, RebaseInput, Rebased, build_rebased, swap_in_rebased,
    write_rebase_file,
};
pub use reconcile::{
    ClockPair, Folded, Plan, ReconcileInput, RunSource, StoredMove, apply_plan, fold, reconcile,
    reconciled_clocks,
};
pub use recover::{
    Recovered, RecoveredGame, RecoveryOutcome, RecoveryReport, ResumeError, StartupReport,
};
pub use writer::{JournalWriter, LeaseState, MoveJournal};
