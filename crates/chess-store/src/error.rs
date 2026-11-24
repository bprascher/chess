//! DD-ST-023: typed errors. `Display` prints a category only: no path, no SQL, no row data
//! (NFR-9, NFR-10; SWE-001 section 6, ERR row). Identifiers that a caller needs travel in the
//! variant fields, and only `PlayerId`/`GameId`/`Uuid` values appear there, never an integer
//! row key (TR-3, D-40). Details go to `tracing` at the point of conversion.
//!
//! Part A defines the variants part A raises; part B adds `Exclusive`, `OtherVersionRunning`
//! and `JournalError`. Variants of later parts (`UnlistedUnfinishedGames`, `VariationAnchor`,
//! `ImportInvariant`) are added with their code; the enums are `non_exhaustive` for that reason.

use std::io;
use std::path::PathBuf;

use chess_domain::{GameId, Mode, PlayerId};
use uuid::Uuid;

/// What is missing, in `StoreError::NotFound`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    Player,
    Game,
}

/// Why the store cannot be used (SDD 3.8 error table, `Unavailable{cause}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableCause {
    DiskFull,
    ReadOnly,
    Permission,
    /// Any other I/O or SQLite open failure; carries the kind only.
    Io(io::ErrorKind),
}

/// Why the pre-upgrade backup failed (`BackupFailed{cause}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupCause {
    /// The target name already exists (`write_verified_copy` never overwrites). The Migrator no
    /// longer returns this for `chess.db.bak-v<N>`: it replaces that file (D-108).
    TargetExists,
    /// The target could not be created or synced.
    Io(io::ErrorKind),
    /// `VACUUM INTO` failed.
    CopyFailed,
    /// The copy did not open or its `quick_check` was not `ok`.
    VerifyFailed,
}

/// Why an operation needs the data directory to itself (`StoreError::Exclusive{reason}`).
/// Part B raises `Migration`; import and restore (part C) add their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExclusiveReason {
    Migration,
}

/// Why a journal write failed (`JournalError::WriteFailed{cause}`, SDD 3.7.3 and section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteCause {
    DiskFull,
    ReadOnly,
    Io(io::ErrorKind),
    /// D-82: an earlier append failed and its cleanup failed, or its caller stopped waiting.
    /// The writer touches nothing any more.
    Poisoned,
    /// The record body is larger than 65 536 bytes (SF-09).
    TooLarge,
}

/// The journal's errors (SDD 3.7.3). `Display` prints a category only.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JournalError {
    /// Another descriptor holds the journal's `flock`. `owner_kind` is the front-end kind from
    /// the `GAME_LEASE` row when the caller looked it up (this layer does not read the store).
    #[error("the game is open elsewhere")]
    Locked { owner_kind: Option<String> },
    #[error("the journal write failed")]
    WriteFailed { cause: WriteCause },
    /// Nothing in the file can be read (bad magic, header checksum, or a different game).
    #[error("the journal is unreadable")]
    Unreadable,
    #[error("the journal version is not supported")]
    VersionMismatch { found: u16, expected: u16 },
    /// The header names another store.
    #[error("the journal belongs to another store")]
    ForeignStore,
}

impl JournalError {
    /// An I/O error while writing: the three causes of SDD section 4.
    pub(crate) fn write_failed(error: &io::Error) -> JournalError {
        JournalError::WriteFailed {
            cause: write_cause(error),
        }
    }
}

pub(crate) fn write_cause(error: &io::Error) -> WriteCause {
    match error.kind() {
        io::ErrorKind::StorageFull => WriteCause::DiskFull,
        io::ErrorKind::ReadOnlyFilesystem => WriteCause::ReadOnly,
        kind => WriteCause::Io(kind),
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// `SQLITE_BUSY` / `SQLITE_LOCKED` after the busy timeout.
    #[error("the store is busy")]
    Busy,
    #[error("the store is unavailable")]
    Unavailable { cause: UnavailableCause },
    /// `NOTADB`, `CORRUPT`, `quick_check` not `ok`, or stored text no enum accepts.
    #[error("the store is corrupt")]
    Corrupt,
    /// No `chess.db`, but traces of an earlier store.
    #[error("the store is missing")]
    Missing,
    /// `user_version` is greater than this build's schema version. Nothing was written.
    #[error("the store schema is newer than this build")]
    SchemaTooNew { db: u32, app: u32 },
    #[error("the pre-upgrade backup failed")]
    BackupFailed { cause: BackupCause },
    /// `unfinished_slot` primary key: `game_id` is the game that holds the slot.
    #[error("an unfinished game already exists")]
    SlotTaken {
        game_id: GameId,
        mode: Mode,
        player_id: PlayerId,
    },
    /// `archive_player` found `in_progress` games of the Player (PLY-4; F-A05, D-117 option A:
    /// the shape of `UnlistedUnfinishedGames`). `ids` are in start order.
    #[error("the player has unfinished games")]
    HasUnfinishedGames { ids: Vec<GameId> },
    /// `reset_player` found `in_progress` games of the Player that the caller did not list
    /// (SDD 3.11.4); the reset rolled back. `ids` are in start order.
    #[error("the player has unfinished games that were not listed")]
    UnlistedUnfinishedGames { ids: Vec<GameId> },
    #[error("the name is already used")]
    NameTaken,
    #[error("not found")]
    NotFound { entity: Entity, id: Uuid },
    /// `UNIQUE(roster_seed, roster_index)`: the caller repeats the read and the insert once.
    #[error("the roster index is taken")]
    RosterIndexTaken,
    /// A CHECK, FK or UNIQUE constraint failed: a bug in the caller.
    #[error("an invariant was violated")]
    InvariantViolated,
    /// D-71, D-109: the open probe's `sync_all` on a scratch file in the data directory failed
    /// (any error: `ENOTSUP`, `ENOTTY`, `ENODEV` ...); `errno` is the OS error. `Display` prints the category only; the path and errno travel here for the
    /// front-end message.
    #[error("the data directory cannot guarantee durable writes")]
    DurabilityUnsupported { path: PathBuf, errno: i32 },
    /// The operation needs the data directory alone and another process is live (SDD 3.11.5).
    #[error("another process is using the data directory")]
    Exclusive { reason: ExclusiveReason },
    /// A live process of another build version is registered (PLAT-5, architecture 7.5).
    /// `kind` is the front-end kind of that process (`cli`, `server`, `desktop`).
    #[error("another version of the program is running")]
    OtherVersionRunning { version: String, kind: String },
    /// `save_variation` with `from_ply` at or beyond the game's last ply (decision 13): a branch
    /// from the final position. The app maps it to `ValidationFailed{field: from_ply,
    /// reason: no_move_after_position}`.
    #[error("the variation starts at the final position")]
    VariationAnchor,
    /// `save_variation` with a `pgn` above `MAX_VARIATION_PGN_BYTES` (FIND-D5R-01): the store is
    /// the last guard, and one write of a huge text would hold the single writer for seconds.
    #[error("the variation text is too large")]
    VariationTooLarge { max_bytes: usize },
    /// A job command by a worker that does not hold the job (DD-ST-013, compare-and-set on
    /// `owner`): the job is absent, not running, or owned by another worker.
    #[error("the analysis job is not owned by this worker")]
    NotOwner,
    /// A panic in a store command, or a lost worker thread.
    #[error("internal store error")]
    Internal,
    /// The game's database updates are parked behind a batch that no retry cures (D3 ruling
    /// 2026-11-24): `apply_records`, `sync_backlog` and a live finalize, abort, void or reset of
    /// this game are refused until `Store::unpark_game` or the next start. The journal holds
    /// every record; only the database lags.
    #[error("the game's database updates are parked")]
    GameParked { game: GameId },
    /// The game holds `MAX_BATCHES_PER_GAME` batches or the apply backlog `MAX_PENDING_BATCHES`
    /// (ESTIMATE bounds, D-145); the batch was not queued and the game is parked. The journal holds it; the next start applies it.
    #[error("the apply backlog is full")]
    BacklogFull,
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> StoreError {
        map_sqlite(&error)
    }
}

/// Maps a SQLite error to the SDD error table. Only the result code is logged.
pub(crate) fn map_sqlite(error: &rusqlite::Error) -> StoreError {
    use rusqlite::ErrorCode;
    let mapped = match error {
        rusqlite::Error::SqliteFailure(failure, _) => match failure.code {
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => StoreError::Busy,
            // Codes that a retry cures (D3 ruling 2026-11-24, FIND-D3R-09): without these the
            // apply backlog would treat them as `Internal`, an error no retry cures, and park
            // the game. Memory pressure, an interrupt, a lock-protocol retry and a schema change
            // seen by a prepared statement all pass.
            ErrorCode::OutOfMemory
            | ErrorCode::OperationInterrupted
            | ErrorCode::FileLockingProtocolFailed
            | ErrorCode::SchemaChanged => StoreError::Busy,
            ErrorCode::DiskFull => StoreError::Unavailable {
                cause: UnavailableCause::DiskFull,
            },
            ErrorCode::ReadOnly => StoreError::Unavailable {
                cause: UnavailableCause::ReadOnly,
            },
            ErrorCode::PermissionDenied | ErrorCode::AuthorizationForStatementDenied => {
                StoreError::Unavailable {
                    cause: UnavailableCause::Permission,
                }
            }
            ErrorCode::SystemIoFailure | ErrorCode::CannotOpen => StoreError::Unavailable {
                cause: UnavailableCause::Io(io::ErrorKind::Other),
            },
            ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt => StoreError::Corrupt,
            ErrorCode::ConstraintViolation => StoreError::InvariantViolated,
            _ => StoreError::Internal,
        },
        // A stored value of the wrong type, out of range or not UTF-8: the file holds something
        // this program did not write (review FIND-004), not a bug in the caller.
        rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::IntegralValueOutOfRange(..)
        | rusqlite::Error::Utf8Error(..) => StoreError::Corrupt,
        _ => StoreError::Internal,
    };
    tracing::debug!(sqlite_code = ?error.sqlite_error_code(), "sqlite error mapped");
    mapped
}

/// An I/O error on the data directory.
pub(crate) fn map_io(error: &io::Error) -> StoreError {
    let cause = match error.kind() {
        io::ErrorKind::PermissionDenied => UnavailableCause::Permission,
        io::ErrorKind::StorageFull => UnavailableCause::DiskFull,
        io::ErrorKind::ReadOnlyFilesystem => UnavailableCause::ReadOnly,
        kind => UnavailableCause::Io(kind),
    };
    tracing::debug!(io_kind = ?error.kind(), "io error mapped");
    StoreError::Unavailable { cause }
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> StoreError {
        map_io(&error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::ffi;

    fn failure(code: i32) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(ffi::Error::new(code), None)
    }

    #[test]
    fn sqlite_result_codes_map_to_the_error_table() {
        let cases: [(i32, &str); 16] = [
            (ffi::SQLITE_BUSY, "Busy"),
            (ffi::SQLITE_LOCKED, "Busy"),
            (ffi::SQLITE_NOMEM, "Busy"),
            (ffi::SQLITE_PROTOCOL, "Busy"),
            (ffi::SQLITE_INTERRUPT, "Busy"),
            (ffi::SQLITE_SCHEMA, "Busy"),
            (ffi::SQLITE_FULL, "DiskFull"),
            (ffi::SQLITE_READONLY, "ReadOnly"),
            (ffi::SQLITE_PERM, "Permission"),
            (ffi::SQLITE_IOERR, "Io"),
            (ffi::SQLITE_CANTOPEN, "Io"),
            (ffi::SQLITE_NOTADB, "Corrupt"),
            (ffi::SQLITE_CORRUPT, "Corrupt"),
            (ffi::SQLITE_CONSTRAINT, "Invariant"),
            (ffi::SQLITE_MISUSE, "Internal"),
            (ffi::SQLITE_AUTH, "Permission"),
        ];
        for (code, expected) in cases {
            let label = match map_sqlite(&failure(code)) {
                StoreError::Busy => "Busy",
                StoreError::Unavailable {
                    cause: UnavailableCause::DiskFull,
                } => "DiskFull",
                StoreError::Unavailable {
                    cause: UnavailableCause::ReadOnly,
                } => "ReadOnly",
                StoreError::Unavailable {
                    cause: UnavailableCause::Permission,
                } => "Permission",
                StoreError::Unavailable {
                    cause: UnavailableCause::Io(_),
                } => "Io",
                StoreError::Corrupt => "Corrupt",
                StoreError::InvariantViolated => "Invariant",
                StoreError::Internal => "Internal",
                other => panic!("unexpected {other:?}"),
            };
            assert_eq!(label, expected, "result code {code}");
        }
        assert!(matches!(
            map_sqlite(&rusqlite::Error::QueryReturnedNoRows),
            StoreError::Internal
        ));
        assert!(matches!(
            StoreError::from(failure(ffi::SQLITE_BUSY)),
            StoreError::Busy
        ));
    }

    /// NF-7 (M44, M45, review NF-8): each stored-value conversion failure is `Corrupt`.
    #[test]
    fn stored_value_conversion_failures_map_to_corrupt() {
        use rusqlite::types::Type;
        let boxed = || -> Box<dyn std::error::Error + Send + Sync> { "bad".into() };
        let cases = [
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, boxed()),
            rusqlite::Error::InvalidColumnType(0, "x".into(), Type::Blob),
            rusqlite::Error::IntegralValueOutOfRange(0, 5),
            rusqlite::Error::Utf8Error(
                0,
                std::str::from_utf8(&std::hint::black_box(vec![0xFFu8])).unwrap_err(),
            ),
        ];
        for case in cases {
            assert!(matches!(map_sqlite(&case), StoreError::Corrupt), "{case:?}");
        }
        assert!(matches!(
            map_sqlite(&rusqlite::Error::InvalidColumnName("x".into())),
            StoreError::Internal
        ));
    }

    #[test]
    fn write_causes_follow_the_io_kind() {
        assert_eq!(
            write_cause(&io::Error::from(io::ErrorKind::StorageFull)),
            WriteCause::DiskFull
        );
        assert_eq!(
            write_cause(&io::Error::from(io::ErrorKind::ReadOnlyFilesystem)),
            WriteCause::ReadOnly
        );
        assert_eq!(
            write_cause(&io::Error::from(io::ErrorKind::NotFound)),
            WriteCause::Io(io::ErrorKind::NotFound)
        );
    }

    #[test]
    fn displays_print_a_category_only() {
        for text in [
            StoreError::Exclusive {
                reason: ExclusiveReason::Migration,
            }
            .to_string(),
            StoreError::OtherVersionRunning {
                version: "9.9.9-secret".into(),
                kind: "server".into(),
            }
            .to_string(),
            JournalError::Locked {
                owner_kind: Some("cli".into()),
            }
            .to_string(),
            JournalError::WriteFailed {
                cause: WriteCause::Poisoned,
            }
            .to_string(),
            JournalError::VersionMismatch {
                found: 2,
                expected: 1,
            }
            .to_string(),
        ] {
            assert!(
                !text.contains("9.9.9") && !text.contains('/') && !text.contains("cli"),
                "{text}"
            );
        }
    }

    #[test]
    fn io_kinds_map_to_unavailable_causes() {
        let kinds = [
            (
                io::ErrorKind::PermissionDenied,
                UnavailableCause::Permission,
            ),
            (io::ErrorKind::StorageFull, UnavailableCause::DiskFull),
            (
                io::ErrorKind::ReadOnlyFilesystem,
                UnavailableCause::ReadOnly,
            ),
            (
                io::ErrorKind::NotFound,
                UnavailableCause::Io(io::ErrorKind::NotFound),
            ),
        ];
        for (kind, cause) in kinds {
            match StoreError::from(io::Error::from(kind)) {
                StoreError::Unavailable { cause: got } => assert_eq!(got, cause),
                other => panic!("{other:?}"),
            }
        }
    }
}
