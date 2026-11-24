//! DD-ST-006: numbered migrations, the schema-version guard and the pre-upgrade backup
//! (SDD 3.11.5; architecture 7.1; DAT-1, DAT-6, D-16).
//!
//! Order at an upgrade of an existing store: **exclusive guard first** (FIND-010: a refused
//! upgrade must never have replaced the last good `chess.db.bak-v<N>`, D-108), backup (replaces
//! an earlier copy of the same version), `BEGIN EXCLUSIVE`, re-read `user_version`, guard again
//! (another process may have registered in between), migrations, `PRAGMA user_version`,
//! `COMMIT`. A failed backup stops the upgrade with the store unchanged (`BackupFailed`). A failed migration rolls back (SQLite DDL is transactional;
//! checked by a test, not assumed). A brand-new empty database (`user_version` 0, no tables)
//! is created by the same migrations with no backup: there is nothing to protect.
//!
//! The `exclusive_guard` is the check that no other live process exists (`Exclusive`, SDD
//! 3.11.5). The `Opener` (part B, F-A03) passes one built on the process registry; the migrator
//! itself knows nothing of the registry and calls the guard twice, before the backup and inside
//! the exclusive transaction.

use std::path::PathBuf;

use rusqlite::{Connection, TransactionBehavior};

use crate::backup::write_replacing_copy;
use crate::error::StoreError;

/// One numbered migration: the SQL that takes the schema from `version - 1` to `version`.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: u32,
    pub sql: &'static str,
}

/// The migrations of this build. Version 1 is the architecture 7.2 schema (DD-ST-007).
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("../migrations/0001_init.sql"),
}];

/// What an `upgrade` call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrateOutcome {
    /// `user_version` before the call.
    pub from: u32,
    /// `user_version` after the call.
    pub to: u32,
    /// A pre-upgrade copy was written (an existing store was upgraded).
    pub backed_up: bool,
}

#[derive(Debug, Clone)]
pub struct Migrator {
    migrations: Vec<Migration>,
}

impl Migrator {
    /// The migrations compiled into this build.
    pub fn embedded() -> Migrator {
        Migrator {
            migrations: MIGRATIONS.to_vec(),
        }
    }

    /// Test support (SDD 3.11.5): other migrations, for example a synthetic version 2. The
    /// versions must be 1, 2, 3 ... without gaps.
    pub fn with(migrations: &[Migration]) -> Result<Migrator, StoreError> {
        let contiguous = migrations
            .iter()
            .zip(1u32..)
            .all(|(migration, expected)| migration.version == expected);
        if migrations.is_empty() || !contiguous {
            return Err(StoreError::InvariantViolated);
        }
        Ok(Migrator {
            migrations: migrations.to_vec(),
        })
    }

    /// The schema version this migrator produces.
    pub fn latest(&self) -> u32 {
        self.migrations.last().map_or(0, |m| m.version)
    }

    /// `PRAGMA user_version`.
    pub fn read_version(conn: &Connection) -> Result<u32, StoreError> {
        let value: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        u32::try_from(value).map_err(|_| StoreError::Corrupt)
    }

    /// Brings `conn` to `latest()`. `backup_path(n)` names the pre-upgrade copy of version `n`.
    pub fn upgrade(
        &self,
        conn: &mut Connection,
        backup_path: impl Fn(u32) -> PathBuf,
        exclusive_guard: &dyn Fn(&Connection) -> Result<(), StoreError>,
    ) -> Result<MigrateOutcome, StoreError> {
        let from = Migrator::read_version(conn)?;
        let latest = self.latest();
        if from > latest {
            return Err(StoreError::SchemaTooNew {
                db: from,
                app: latest,
            });
        }
        if from == latest {
            return Ok(MigrateOutcome {
                from,
                to: from,
                backed_up: false,
            });
        }
        let existing_store = from > 0;
        if !existing_store && table_count(conn)? > 0 {
            // user_version 0 with tables: not a database of this program, unless another
            // process migrated between the two reads (a first-launch race): then `Busy`, and
            // the caller repeats the step. Never touched.
            return if Migrator::read_version(conn)? == 0 {
                Err(StoreError::Corrupt)
            } else {
                Err(StoreError::Busy)
            };
        }
        if existing_store {
            // FIND-010 / F-A03: refuse before the backup, so a refused upgrade leaves the old
            // `chess.db.bak-v<N>` untouched.
            exclusive_guard(conn)?;
            let target = backup_path(from);
            write_replacing_copy(conn, &target)
                .map_err(|cause| StoreError::BackupFailed { cause })?;
            // Kill point (D1): the new copy is in place, the upgrade has not started.
            crate::fault::check("migrate.after_backup", &target).map_err(StoreError::from)?;
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Exclusive)?;
        // Re-read after taking the exclusive lock: another process may have migrated.
        if Migrator::read_version(&tx)? != from {
            return Err(StoreError::Busy);
        }
        exclusive_guard(&tx)?;
        for migration in self.migrations.iter().filter(|m| m.version > from) {
            tx.execute_batch(migration.sql)?;
        }
        tx.pragma_update(None, "user_version", latest)?;
        tx.commit()?;
        Ok(MigrateOutcome {
            from,
            to: latest,
            backed_up: existing_store,
        })
    }
}

fn table_count(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
        [],
        |row| row.get(0),
    )?)
}
