//! DD-ST-005 `Opener`, part A: the open sequence of SDD 3.9.1 up to and including the upgrade.
//!
//! Order here: ensure the directory (`0700`), probe `F_FULLFSYNC` (D-71), missing-store test,
//! create-if-absent of `chess.db`, clean up scratch files of crashed processes (D1-01),
//! read-only version probe and integrity check, `store.id` for a first launch, write connection
//! with the pragmas, **process registration and the other-version check (part B, F-A03)**,
//! backup and migrate. Not here: the startup journal recovery (part D4). The
//! `CreateFreshStore` operation (set traces aside in `missing-<date>/`) is not here either: the
//! SDD names the behaviour but no store method for it (evidence finding F-A07).
//!
//! Deviation from the diagram, chosen to keep "file untouched" for a corrupt store (CT-ST-012):
//! `quick_check` runs on a read-only connection before the write connection and its
//! `journal_mode` pragma can touch the file.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use uuid::Uuid;

use crate::api::{OpenReport, ProcessIdentity};
use crate::datadir::{DataDir, StoreId, create_new_0600, sqlite_path, sync_dir};
use crate::error::{ExclusiveReason, StoreError, map_io};
use crate::fault;
use crate::migrate::Migrator;
use crate::proc::pid_alive;
use crate::repo::coord;

const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

pub(crate) struct Opened {
    pub write: Connection,
    pub readers: Vec<Connection>,
    pub report: OpenReport,
}

/// Write connection with the pragmas of architecture 7.1 and SDD 3.12.
///
/// Opened with explicit flags **without** `SQLITE_OPEN_URI` and through `sqlite_path`
/// (review FIND-009): the path is a path, so a relative directory named `file:...` is not parsed
/// as a URI (the bundled SQLite is built with `SQLITE_USE_URI`, so the `./` prefix is what
/// protects it). The busy timeout is set
/// first, because the switch to WAL takes a lock and two first launches may meet here.
pub(crate) fn open_write_connection(db: &Path) -> Result<Connection, StoreError> {
    let conn = Connection::open_with_flags(
        sqlite_path(db),
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    apply_write_pragmas(&conn)?;
    Ok(conn)
}

/// The pragmas of architecture 7.1 and SDD 3.12 (review FIND-003: each one is read back by a
/// test). `journal_mode` must really become WAL, otherwise the open is refused.
fn apply_write_pragmas(conn: &Connection) -> Result<(), StoreError> {
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(StoreError::Unavailable {
            cause: crate::error::UnavailableCause::Io(io::ErrorKind::Unsupported),
        });
    }
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "fullfsync", "ON")?;
    conn.pragma_update(None, "checkpoint_fullfsync", "ON")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

/// Read-only connection (`query_only`) for the read pool.
pub(crate) fn open_read_connection(db: &Path) -> Result<Connection, StoreError> {
    let conn = Connection::open_with_flags(
        sqlite_path(db),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.pragma_update(None, "query_only", "ON")?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}

/// D-71, D-109: before any write, call `fcntl(F_FULLFSYNC)` (`File::sync_all`, which has no
/// fallback on Apple targets) on a scratch file in the directory. **Any** error of that call
/// refuses the directory with `DurabilityUnsupported{path, errno}`. Errors creating, writing or
/// removing the scratch file stay `Unavailable` (they say nothing about sync support). The
/// scratch file is always removed.
// PENDING D-71 refusal wording (PM-held, Brian): only the front-end wording is open and it is
// not in this crate.
fn probe_durability(dir: &DataDir) -> Result<(), StoreError> {
    probe_durability_with(dir.root(), &|file| {
        fault::sync_file("sync.probe", dir.root(), file)
    })
}

/// `probe_durability` with the sync call as a seam, so a test can inject any errno.
fn probe_durability_with(
    root: &Path,
    sync: &dyn Fn(&File) -> io::Result<()>,
) -> Result<(), StoreError> {
    // Unique per call, so two opens in one process cannot collide on the scratch file.
    static PROBES: AtomicU64 = AtomicU64::new(0);
    let scratch = root.join(format!(
        ".durability-probe-{}-{}",
        std::process::id(),
        PROBES.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = create_new_0600(&scratch).map_err(|e| map_io(&e))?;
    let written = file.write_all(b"x");
    let synced = written.as_ref().ok().map(|()| sync(&file));
    drop(file);
    let removed = fs::remove_file(&scratch);
    if let Err(e) = written {
        return Err(map_io(&e));
    }
    if let Some(Err(e)) = synced {
        return Err(StoreError::DurabilityUnsupported {
            path: root.to_path_buf(),
            errno: e.raw_os_error().unwrap_or(0),
        });
    }
    removed.map_err(|e| map_io(&e))
}

fn quick_check(conn: &Connection) -> Result<(), StoreError> {
    let result: String = conn.pragma_query_value(None, "quick_check", |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        tracing::warn!("quick_check did not return ok");
        Err(StoreError::Corrupt)
    }
}

fn table_count(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
        [],
        |row| row.get(0),
    )?)
}

/// A new store identity. No `v4` feature and no random crate are admitted for this crate
/// (D-101, DL2-4), so the 16 bytes come from `RandomState`, which std seeds from the OS. The id
/// only has to tell two stores apart (the journal's foreign-store check), not to be secret.
fn new_store_uuid() -> Uuid {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u8(0);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    uuid::Builder::from_random_bytes(bytes).into_uuid()
}

/// The read-only probe connection and its first read (where a hot rollback journal shows).
fn read_only_probe(db: &Path) -> Result<Connection, StoreError> {
    let probe = Connection::open_with_flags(sqlite_path(db), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    // A first launch in another process holds locks while it switches to WAL.
    probe.busy_timeout(BUSY_TIMEOUT)?;
    Migrator::read_version(&probe)?;
    Ok(probe)
}

/// Largest `chess.db` or `chess.db-journal` that the NF-2 copy test will copy (condition H of
/// store-rework-specs 1.1). A freshly created schema of this crate is far smaller (measured by
/// `tests/nf2_narrowed.rs`, 16 times that is the floor of this constant), so a file above the
/// cap cannot be an interrupted first creation.
const NF2_COPY_CAP: u64 = 4 * 1024 * 1024;

/// Scratch directory of the NF-2 copy: created `0700` under the system temp directory, never
/// inside the data directory, removed (with everything in it) on drop.
struct Nf2Scratch(std::path::PathBuf);

impl Nf2Scratch {
    fn create() -> io::Result<Nf2Scratch> {
        use std::os::unix::fs::DirBuilderExt;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir();
        for _ in 0..16 {
            let path = base.join(format!(
                "chess-nf2-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            // `create` fails when the last component exists, a planted symlink included.
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Nf2Scratch(path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::from(io::ErrorKind::AlreadyExists))
    }
}

/// The copy cap; a test may lower it to put the boundary inside a small file.
fn nf2_copy_cap() -> u64 {
    #[cfg(test)]
    if let Some(cap) = NF2_CAP_OVERRIDE.with(std::cell::Cell::get) {
        return cap;
    }
    NF2_COPY_CAP
}

/// Copies `from` to the new file `to` (mode `0600`), reading at most the cap plus one byte: a file
/// that grew after the size check (FIND-RW1-06) is refused here, so the copy never exceeds the
/// cap whatever the file does between the check and the read.
fn copy_capped(from: &Path, to: &Path) -> io::Result<()> {
    use std::io::Read;
    let cap = nf2_copy_cap();
    let mut source = File::open(from)?;
    let mut target = create_new_0600(to)?;
    let copied = io::copy(&mut (&mut source).take(cap.saturating_add(1)), &mut target)?;
    if copied > cap {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "file above the NF-2 copy cap",
        ));
    }
    Ok(())
}

/// FIND-RW1-08: removes `chess-nf2-<pid>-<n>` directories of dead processes from `base` (the
/// temp directory). A killed opener leaves one, holding up to the cap of the user's database
/// and its journal. Only a real directory (never a symlink) of mode `0700` owned by `owner_uid`
/// (the owner of the data directory) whose pid is not this process and not alive is removed;
/// anything else under that name is left. Best effort, returns the number removed.
fn sweep_stale_nf2_scratch(base: &Path, owner_uid: u32, alive: &dyn Fn(u32) -> bool) -> usize {
    use std::os::unix::fs::MetadataExt;
    let Ok(entries) = fs::read_dir(base) else {
        return 0;
    };
    let own = std::process::id();
    let mut verdicts = std::collections::HashMap::new();
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|n| n.strip_prefix("chess-nf2-")) else {
            continue;
        };
        let Some((pid, counter)) = rest.split_once('-') else {
            continue;
        };
        let (Ok(pid), Ok(_)) = (pid.parse::<u32>(), counter.parse::<u64>()) else {
            continue;
        };
        if pid == own {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if !meta.is_dir() || meta.mode() & 0o777 != 0o700 || meta.uid() != owner_uid {
            continue;
        }
        let live = *verdicts.entry(pid).or_insert_with(|| alive(pid));
        if !live && fs::remove_dir_all(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

impl Drop for Nf2Scratch {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.0) {
            tracing::warn!(kind = ?e.kind(), "NF-2 scratch directory was not removed");
        }
    }
}

/// Condition G of the narrowed NF-2 rule, and the conditions H and I that make it safe: true
/// only when a **private copy** of `chess.db` and its hot journal, rolled back by SQLite, holds
/// no players and no games. The real files are only read. Any problem (a file above
/// `NF2_COPY_CAP`, no scratch directory, a copy error, an SQLite error on the copy) is "cannot
/// tell" and gives `false`.
fn rollback_copy_holds_no_players_or_games(db: &Path) -> bool {
    fn check(db: &Path) -> Result<bool, Box<dyn std::error::Error>> {
        let journal = {
            let mut name = db.as_os_str().to_owned();
            name.push("-journal");
            std::path::PathBuf::from(name)
        };
        for file in [db, journal.as_path()] {
            if fs::metadata(file)?.len() > nf2_copy_cap() {
                return Ok(false);
            }
        }
        let scratch = Nf2Scratch::create()?;
        fault::check("open.nf2_copy", db)?;
        #[cfg(test)]
        NF2_SCRATCH_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow().as_ref() {
                hook(&scratch.0, "before_copy");
            }
        });
        let copy = scratch.0.join("chess.db");
        // The database first, then the journal, whose name is the database name plus `-journal`.
        copy_capped(db, &copy)?;
        copy_capped(&journal, &scratch.0.join("chess.db-journal"))?;
        #[cfg(test)]
        NF2_SCRATCH_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow().as_ref() {
                hook(&scratch.0, "after_copy");
            }
        });
        let conn = Connection::open_with_flags(
            sqlite_path(&copy),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        let mut empty = true;
        for table in ["player", "game"] {
            let present: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )?;
            if present > 0 {
                // `table` is one of two literals above, never caller text.
                let rows: i64 =
                    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })?;
                empty &= rows == 0;
            }
        }
        drop(conn);
        Ok(empty)
    }
    match check(db) {
        Ok(empty) => empty,
        Err(_) => {
            tracing::warn!("NF-2: could not examine a copy of the database; keeping the error");
            false
        }
    }
}

/// The probe, with the one exception of NF-2 (review of store part A rework 1; the SDD 3.9.1
/// note 1 text "the next open completes an interrupted first creation" needs it): a first
/// launch killed inside the switch to WAL can leave a hot rollback journal, which a read-only
/// connection cannot roll back (`SQLITE_READONLY`, mapped to `Unavailable{ReadOnly}` for ever).
/// If the probe answers `ReadOnly`, the directory shows nothing but that cut-off first creation
/// (`DataDir::is_interrupted_first_creation`) **and** a rollback of a private copy leaves a
/// database with no players and no games (FIND-B-06 narrowed, Brian Prascher 2026-11-22;
/// `rollback_copy_holds_no_players_or_games`), the real file is opened read-write once, the
/// journal is rolled back by SQLite (which restores the committed state, an empty database), and
/// the probe runs again. Any other `ReadOnly` stays an error and nothing is written: `chess.db`
/// and `chess.db-journal` are byte for byte as found.
fn open_probe(data_dir: &DataDir, db: &Path) -> Result<Connection, StoreError> {
    match read_only_probe(db) {
        Err(StoreError::Unavailable {
            cause: crate::error::UnavailableCause::ReadOnly,
        }) if data_dir.is_interrupted_first_creation()?
            && rollback_copy_holds_no_players_or_games(db) =>
        {
            let recover = Connection::open_with_flags(
                sqlite_path(db),
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            recover.busy_timeout(BUSY_TIMEOUT)?;
            table_count(&recover)?;
            drop(recover);
            read_only_probe(db)
        }
        other => other,
    }
}

/// How often a first launch that lost a race re-runs the upgrade step (see `open_blocking`).
const FIRST_LAUNCH_ATTEMPTS: usize = 8;

#[cfg(test)]
type Hook = std::cell::RefCell<Option<Box<dyn Fn()>>>;

#[cfg(test)]
type ScratchHook = std::cell::RefCell<Option<Box<dyn Fn(&Path, &str)>>>;

#[cfg(test)]
thread_local! {
    /// Test seam (NF-7, review FIND-R05): runs between the first version read of the probe and
    /// the table count, where a concurrent first launch can commit its schema.
    static BETWEEN_READS: Hook = const { std::cell::RefCell::new(None) };
    /// Test seam (NF4-18): runs before the NF-4 trace scan, where a concurrent first launch may
    /// finish and a journal appear.
    static BEFORE_TRACE_SCAN: Hook = const { std::cell::RefCell::new(None) };
    /// Test seam: lowers the NF-2 copy cap so the boundary sits inside a small file.
    static NF2_CAP_OVERRIDE: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
    /// Test seam: runs with the scratch directory before and after the NF-2 copy.
    static NF2_SCRATCH_HOOK: ScratchHook = const { std::cell::RefCell::new(None) };
    /// Test seam: fail `finish_open` right after `store.id` was written (review FIND-002).
    static FAIL_AFTER_STORE_ID: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The open sequence. **First launch is race-safe and never deletes anything** (review
/// FIND-001, FIND-002; the rule replaces the part A cleanup of `chess.db*`):
///
/// - `chess.db` is created with `create_new` (atomic). The opener that gets `AlreadyExists` is
///   a loser and simply carries on with the existing file; nobody removes a database another
///   opener may already be using.
/// - A first launch that fails leaves an empty `chess.db` (no tables, `user_version` 0) and
///   possibly `store.id` and the empty layout. That is the "interrupted first creation" state
///   that the next open already treats as a first launch, so it never reports `Missing`: the
///   database file is created before `store.id`, and `store.id` is written before the schema.
/// - `store.id` is written atomically (temp file, then `hard_link` without replacing), so a
///   loser never reads a torn file and the first writer wins.
/// - An opener that loses the exclusive migration (`Busy` from the version re-read) repeats the
///   upgrade step, which then finds the winner's schema and does nothing.
pub(crate) fn open_blocking(
    data_dir: &DataDir,
    migrator: &Migrator,
    read_connections: usize,
    now: i64,
    process: &ProcessIdentity,
) -> Result<Opened, StoreError> {
    let ensured = data_dir.ensure_root()?;
    probe_durability(data_dir)?;
    let db = data_dir.db_path();
    // Re-check `db.exists()` after the trace scan: a concurrent first launch creates `chess.db`
    // before it writes `store.id`, so traces seen with the database now present are that race
    // and not a lost store.
    if !db.exists() && data_dir.has_traces_of_a_store()? && !db.exists() {
        return Err(StoreError::Missing);
    }
    match create_new_0600(&db) {
        Ok(file) => drop(file),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(map_io(&e)),
    }
    fault::check("open.after_db_created", &db).map_err(|e| map_io(&e))?;
    // D1-01: scratch files of crashed processes (`.durability-probe-*`, `.store.id.tmp-*`).
    data_dir.remove_stale_scratch(&pid_alive);
    // FIND-RW1-08: and the NF-2 scratch directories (copies of a database) that a killed opener
    // left in the temp directory.
    if let Ok(meta) = fs::metadata(data_dir.root()) {
        use std::os::unix::fs::MetadataExt;
        sweep_stale_nf2_scratch(&std::env::temp_dir(), meta.uid(), &pid_alive);
    }
    // NF-4: a `-wal` with content beside a zero-length `chess.db` is deleted by SQLite itself when
    // the file is opened (`[RAN]` 2026-11-22: even a read-only open), so it is looked at before
    // the probe and not after.
    let wal_before_probe = data_dir.data_traces()?.wal;
    // A zero-length `chess.db` beside a `-wal` with content cannot be a first launch in progress
    // (SQLite writes the header before it creates a WAL `[INF]`), and opening it would delete
    // that WAL: refuse here, before SQLite touches it (NF-4: nothing is written or removed).
    if wal_before_probe && fs::metadata(&db).map_err(|e| map_io(&e))?.len() == 0 {
        tracing::warn!(
            reason = "empty_database_beside_data_traces",
            "refusing to open"
        );
        return Err(StoreError::Missing);
    }
    // Read-only: nothing is written for SchemaTooNew or Corrupt.
    let probe = open_probe(data_dir, &db)?;
    let found = Migrator::read_version(&probe)?;
    #[cfg(test)]
    BETWEEN_READS.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook();
        }
    });
    if found > migrator.latest() {
        return Err(StoreError::SchemaTooNew {
            db: found,
            app: migrator.latest(),
        });
    }
    quick_check(&probe)?;
    // An empty file or one with no tables is a first launch (also an interrupted one); a
    // database with tables and no version is not ours, and is refused before any write.
    let mut fresh = settle_fresh(&probe, migrator, found)?;
    if fresh {
        match nf4_verdict(data_dir, &probe)? {
            Nf4::Proceed => {}
            Nf4::Refuse => {
                tracing::warn!(
                    reason = "empty_database_beside_data_traces",
                    "refusing to open"
                );
                return Err(StoreError::Missing);
            }
            // The schema appeared while the traces were examined: decide again, as an opener
            // that read the schema in the first place.
            Nf4::SchemaAppeared => {
                let again = Migrator::read_version(&probe)?;
                if again > migrator.latest() {
                    return Err(StoreError::SchemaTooNew {
                        db: again,
                        app: migrator.latest(),
                    });
                }
                fresh = settle_fresh(&probe, migrator, again)?;
            }
        }
    }
    drop(probe);
    let attempts = if fresh { FIRST_LAUNCH_ATTEMPTS } else { 1 };
    let mut attempt = 1;
    loop {
        match finish_open(
            data_dir,
            migrator,
            read_connections,
            now,
            fresh,
            ensured.loose_permissions,
            process,
        ) {
            Err(StoreError::Busy) if attempt < attempts => {
                // SQLite does not always run the busy handler while the winner switches the
                // file to WAL, so a short pause before the next attempt (`BUSY` from the WAL
                // switch seen with three processes, 2026-11-10 `[RAN]`).
                std::thread::sleep(Duration::from_millis(
                    5 * u64::try_from(attempt).unwrap_or(1),
                ));
                attempt += 1;
            }
            other => return other,
        }
    }
}

/// Is this a first launch? An empty file or one with no tables is (also an interrupted one); a
/// database with tables and no version is not ours and is refused before any write.
fn settle_fresh(probe: &Connection, migrator: &Migrator, found: u32) -> Result<bool, StoreError> {
    if found != 0 {
        return Ok(false);
    }
    if table_count(probe)? > 0 {
        // The version and the table count are separate reads: a concurrent first launch may
        // have committed its schema in between (found by the 10x loop, run 9, `Corrupt`).
        // Read the version again; tables with a version of 0 are not our database.
        let again = Migrator::read_version(probe)?;
        if again == 0 {
            return Err(StoreError::Corrupt);
        }
        if again > migrator.latest() {
            return Err(StoreError::SchemaTooNew {
                db: again,
                app: migrator.latest(),
            });
        }
        return Ok(false);
    }
    Ok(true)
}

/// The answer of `nf4_verdict`.
enum Nf4 {
    /// No data trace: a first launch (rows 4 and 5 of the table).
    Proceed,
    /// Row 7: empty database beside data traces.
    Refuse,
    /// A schema was committed while the traces were examined (a concurrent first launch).
    SchemaAppeared,
}

/// NF-4 (Brian Prascher 2026-11-22; ruling 2026-11-23, FIND-RW1-01; package section (a) rows 4,
/// 5, 5b, 7): an empty or schemaless `chess.db` beside a **provenance trace** of an earlier
/// store's data (`DataDir::data_traces`: journal content, backups, backup or import files,
/// `chess.db.restoring`) is **not** a first launch: `Store::open` refuses with `Missing` and
/// nothing is written. The SQLite sidecars `-wal` and `-journal` are not traces on their own: a
/// creation cut off inside its first transaction leaves them and holds no data, so they never
/// refuse here (the zero-length-database-beside-a-`-wal` case is refused earlier, before SQLite
/// can delete that WAL). Called after the probe found no schema. The traces are scanned
/// **first** and the schema is read **after**: if a schema has appeared meanwhile (a concurrent
/// first launch finished) the answer is `SchemaAppeared`. There is no waiting. A first launch has
/// no provenance trace: its layout is empty and `store.id` is not a trace (FIND-002 stays: empty
/// `chess.db` plus `store.id` only is a first launch). Nothing here writes, moves or deletes.
fn nf4_verdict(data_dir: &DataDir, probe: &Connection) -> Result<Nf4, StoreError> {
    #[cfg(test)]
    BEFORE_TRACE_SCAN.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook();
        }
    });
    let traces = data_dir.data_traces()?;
    if !traces.provenance {
        return Ok(Nf4::Proceed);
    }
    if Migrator::read_version(probe)? > 0 || table_count(probe)? > 0 {
        return Ok(Nf4::SchemaAppeared);
    }
    Ok(Nf4::Refuse)
}

fn finish_open(
    data_dir: &DataDir,
    migrator: &Migrator,
    read_connections: usize,
    now: i64,
    fresh: bool,
    loose_permissions: bool,
    process: &ProcessIdentity,
) -> Result<Opened, StoreError> {
    data_dir.ensure_layout()?;
    let db = data_dir.db_path();
    if fresh && data_dir.read_store_id()?.is_none() {
        let created = data_dir.write_store_id_once(&StoreId {
            store_uuid: new_store_uuid(),
            created_at: now,
        })?;
        if !created {
            // Another first launch wrote it first: its file stands (NF-3: only a real
            // `store.id` that exists says so; a stale temp name never does).
            data_dir.read_store_id()?.ok_or(StoreError::Corrupt)?;
        }
    }
    fault::check("open.after_store_id", &db).map_err(|e| map_io(&e))?;
    #[cfg(test)]
    if FAIL_AFTER_STORE_ID.with(std::cell::Cell::get) {
        return Err(StoreError::Unavailable {
            cause: crate::error::UnavailableCause::Io(io::ErrorKind::Other),
        });
    }
    let mut write = open_write_connection(&db)?;
    // F-A03: register this process and look at the others before anything is migrated. A
    // store that is at schema version 0 has no registry table yet; its schema is created by the
    // upgrade below and the registration follows it.
    let registry_exists = coord::has_registry(&write)?;
    if registry_exists {
        coord::register(&mut write, process, now, &pid_alive)?;
    }
    let outcome = upgrade_registered(
        &mut write,
        data_dir,
        migrator,
        now,
        process,
        registry_exists,
    );
    let outcome = outcome.inspect_err(|_| {
        // Every error path removes this process's row (SDD section 4, DD-ST-005).
        if registry_exists {
            let _ = coord::deregister(&write, process);
        }
    })?;
    let mut readers = Vec::new();
    for _ in 0..read_connections.max(1) {
        readers.push(open_read_connection(&db)?);
    }
    // The database file itself is durable once the migration committed under synchronous=FULL;
    // the directory entry of a file created now needs its own sync.
    sync_dir(data_dir.root()).map_err(|e| map_io(&e))?;
    Ok(Opened {
        write,
        readers,
        report: OpenReport {
            // The opener whose upgrade took the schema from nothing to version 1.
            first_launch: outcome.from == 0,
            upgraded_from: (outcome.backed_up).then_some(outcome.from),
            schema_version: outcome.to,
            loose_permissions,
        },
    })
}

/// The upgrade with the exclusive guard wired to the process registry (F-A03, SDD 3.11.5): no
/// other live process may exist, or the upgrade returns `Exclusive{Migration}`; the migrator
/// calls the guard before its backup (FIND-010) and again inside its exclusive transaction.
/// A store that had no registry table is registered after its schema exists.
fn upgrade_registered(
    write: &mut Connection,
    data_dir: &DataDir,
    migrator: &Migrator,
    now: i64,
    process: &ProcessIdentity,
    registered: bool,
) -> Result<crate::migrate::MigrateOutcome, StoreError> {
    let own = process.owner();
    let guard = |conn: &Connection| -> Result<(), StoreError> {
        if coord::other_live(conn, &own, now, &pid_alive)?.is_empty() {
            Ok(())
        } else {
            Err(StoreError::Exclusive {
                reason: ExclusiveReason::Migration,
            })
        }
    };
    let outcome = migrator.upgrade(
        write,
        |version| data_dir.pre_upgrade_backup_path(version),
        &guard,
    )?;
    if !registered {
        coord::register(write, process, now, &pid_alive)?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::UnavailableCause;

    fn me() -> ProcessIdentity {
        ProcessIdentity::current("cli", "test")
    }

    fn pragma_int(conn: &Connection, name: &str) -> i64 {
        conn.pragma_query_value(None, name, |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn the_write_connection_has_every_normative_pragma_read_back() {
        let dir = tempfile::TempDir::new().unwrap();
        let conn = open_write_connection(&dir.path().join("chess.db")).unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        assert_eq!(pragma_int(&conn, "synchronous"), 2, "FULL");
        assert_eq!(pragma_int(&conn, "fullfsync"), 1);
        assert_eq!(pragma_int(&conn, "checkpoint_fullfsync"), 1);
        assert_eq!(pragma_int(&conn, "foreign_keys"), 1);
        assert_eq!(pragma_int(&conn, "busy_timeout"), 5000);
    }

    #[test]
    fn the_open_is_refused_when_journal_mode_does_not_become_wal() {
        // An in-memory database answers `memory` to `journal_mode = WAL`.
        let memory = Connection::open_in_memory().unwrap();
        let refused = apply_write_pragmas(&memory);
        assert!(
            matches!(
                refused,
                Err(StoreError::Unavailable {
                    cause: UnavailableCause::Io(io::ErrorKind::Unsupported)
                })
            ),
            "{refused:?}"
        );
    }

    /// Review FIND-002: a first launch that fails after `store.id` was written must not leave a
    /// directory the next open reports as `Missing`.
    #[test]
    fn a_failed_first_launch_after_store_id_is_a_first_launch_again_not_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let data = DataDir::new(dir.path());
        FAIL_AFTER_STORE_ID.with(|f| f.set(true));
        let failed = open_blocking(&data, &Migrator::embedded(), 1, 5, &me());
        FAIL_AFTER_STORE_ID.with(|f| f.set(false));
        assert!(failed.is_err());
        assert!(
            dir.path().join("store.id").exists(),
            "the failure point was reached"
        );
        let first_id = data.read_store_id().unwrap().unwrap();
        let retry = open_blocking(&data, &Migrator::embedded(), 1, 6, &me()).unwrap();
        assert!(retry.report.first_launch, "the retry is the first launch");
        assert_eq!(retry.report.schema_version, 1);
        assert_eq!(
            data.read_store_id().unwrap().unwrap().store_uuid,
            first_id.store_uuid,
            "the identity of the first attempt stands"
        );
        drop(retry);
        let third = open_blocking(&data, &Migrator::embedded(), 1, 7, &me()).unwrap();
        assert!(!third.report.first_launch);
    }

    #[test]
    fn any_sync_error_refuses_the_directory_with_path_and_errno() {
        let dir = tempfile::TempDir::new().unwrap();
        // ENOTSUP 45, ENOTTY 25, EIO 5, ENODEV 19: all refuse (D-109).
        for errno in [45, 25, 5, 19] {
            let result =
                probe_durability_with(dir.path(), &|_| Err(io::Error::from_raw_os_error(errno)));
            match result {
                Err(StoreError::DurabilityUnsupported { path, errno: got }) => {
                    assert_eq!((path.as_path(), got), (dir.path(), errno));
                }
                other => panic!("{other:?}"),
            }
            assert_eq!(
                fs::read_dir(dir.path()).unwrap().count(),
                0,
                "scratch removed"
            );
        }
    }

    #[test]
    fn a_non_regular_file_sync_error_from_the_spike_refuses() {
        // OQ-14 spike: sync_all on /dev/null fails (ENODEV on macOS). Whatever the OS says, the
        // real error of a real sync_all call must refuse, not pass.
        let dir = tempfile::TempDir::new().unwrap();
        let result = probe_durability_with(dir.path(), &|_| {
            File::open("/dev/null").and_then(|f| f.sync_all())
        });
        match (File::open("/dev/null").and_then(|f| f.sync_all()), result) {
            (Err(e), Err(StoreError::DurabilityUnsupported { errno, .. })) => {
                assert_eq!(Some(errno), e.raw_os_error());
            }
            (Ok(()), Ok(())) => {} // a platform where it succeeds: nothing to refuse
            (direct, probed) => panic!("{direct:?} vs {probed:?}"),
        }
    }

    #[test]
    fn scratch_file_errors_stay_unavailable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let denied = probe_durability_with(dir.path(), &|_| Ok(()));
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            matches!(
                denied,
                Err(StoreError::Unavailable {
                    cause: UnavailableCause::Permission
                })
            ),
            "{denied:?}"
        );
        // A missing directory is also a create error, not a sync refusal.
        let missing = probe_durability_with(&dir.path().join("absent"), &|_| {
            Err(io::Error::from_raw_os_error(45))
        });
        assert!(
            matches!(missing, Err(StoreError::Unavailable { .. })),
            "{missing:?}"
        );
    }

    // ---- NF-7: the FIND-R05 window (another process commits the schema between two reads) ----

    fn with_hook<T>(hook: impl Fn() + 'static, body: impl FnOnce() -> T) -> T {
        BETWEEN_READS.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
        let out = body();
        BETWEEN_READS.with(|h| *h.borrow_mut() = None);
        out
    }

    fn open_dir(dir: &tempfile::TempDir) -> Result<Opened, StoreError> {
        open_blocking(
            &DataDir::new(dir.path()),
            &Migrator::embedded(),
            1,
            5,
            &me(),
        )
    }

    /// The winner of a first-launch race commits version 1 between our two reads: the version
    /// is read again, the open carries on as a later opener (not `Corrupt`, not a first launch).
    #[test]
    fn r05_a_schema_committed_between_the_reads_is_a_later_opener_not_corrupt() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = dir.path().join("chess.db");
        let hook_db = db.clone();
        let result = with_hook(
            move || {
                let c = Connection::open(&hook_db).unwrap();
                c.execute_batch(crate::migrate::MIGRATIONS[0].sql).unwrap();
                c.pragma_update(None, "user_version", 1).unwrap();
            },
            || open_dir(&dir),
        );
        let opened = result.unwrap();
        assert!(!opened.report.first_launch, "the winner created the schema");
        assert_eq!(opened.report.schema_version, 1);
    }

    fn with_trace_hook<T>(hook: impl Fn() + 'static, body: impl FnOnce() -> T) -> T {
        BEFORE_TRACE_SCAN.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
        let out = body();
        BEFORE_TRACE_SCAN.with(|h| *h.borrow_mut() = None);
        out
    }

    /// NF4-18: a concurrent first launch finishes (schema committed, a game's journal appears)
    /// while this opener sits between its probe read and the NF-4 trace scan. The re-read finds
    /// the schema and the answer is not `Missing`: an ordinary open of an existing store.
    #[test]
    fn nf4_18_a_schema_committed_before_the_trace_scan_is_not_a_refusal() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let result = with_trace_hook(
            move || {
                let c = Connection::open(root.join("chess.db")).unwrap();
                c.execute_batch(crate::migrate::MIGRATIONS[0].sql).unwrap();
                c.pragma_update(None, "user_version", 1).unwrap();
                fs::create_dir_all(root.join("journal")).unwrap();
                fs::write(
                    root.join("journal/11111111-1111-1111-1111-111111111111.jnl"),
                    b"x",
                )
                .unwrap();
            },
            || open_dir(&dir),
        );
        let opened = result.unwrap();
        assert!(!opened.report.first_launch, "the winner created the schema");
        assert_eq!(opened.report.schema_version, 1);
        // Row 8, not a first launch: `store.id` is not written by this opener (FIND-007).
        assert!(!dir.path().join("store.id").exists());
    }

    /// NF4-18, the control: the same trace without a schema is the refusal (so the hook above
    /// is what makes the difference), and the re-read is what saves the open.
    #[test]
    fn nf4_18_control_the_same_trace_without_a_schema_is_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let result = with_trace_hook(
            move || {
                fs::create_dir_all(root.join("journal")).unwrap();
                fs::write(
                    root.join("journal/11111111-1111-1111-1111-111111111111.jnl"),
                    b"x",
                )
                .unwrap();
            },
            || open_dir(&dir),
        );
        assert!(
            matches!(result, Err(StoreError::Missing)),
            "{:?}",
            result.err()
        );
    }

    /// FIND-RW1-01 (ruling 2026-11-23): a creator that is alive shows a `-wal` or a `-journal`
    /// with content while this opener scans the traces. Neither is a trace on its own: the open
    /// is a first launch at once (no settle wait), whatever the creator does next.
    #[test]
    fn rw1_01_a_sidecar_that_appears_before_the_trace_scan_is_not_a_refusal_and_not_a_wait() {
        for name in ["chess.db-journal", "chess.db-wal"] {
            let dir = tempfile::TempDir::new().unwrap();
            // A non-zero schemaless database (a zero-length one beside a WAL is refused before
            // the probe, by design).
            Connection::open(dir.path().join("chess.db"))
                .unwrap()
                .execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE x (a); DROP TABLE x;")
                .unwrap();
            let appear = dir.path().join(name);
            let started = std::time::Instant::now();
            let result = with_trace_hook(
                move || fs::write(&appear, vec![7u8; 512]).unwrap(),
                || open_dir(&dir),
            );
            let opened = result.unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(opened.report.first_launch, "{name}");
            assert!(
                started.elapsed() < Duration::from_millis(900),
                "{name}: no settle wait, {:?}",
                started.elapsed()
            );
        }
    }

    /// The provenance side of the same hook: a data trace that appears at that moment is a
    /// refusal at once, and nothing is removed.
    #[test]
    fn rw1_01_a_provenance_trace_that_appears_before_the_trace_scan_is_a_refusal() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        let started = std::time::Instant::now();
        let result = with_trace_hook(
            move || fs::write(root.join("chess.db.bak-1"), b"x").unwrap(),
            || open_dir(&dir),
        );
        assert!(
            matches!(result, Err(StoreError::Missing)),
            "{:?}",
            result.err()
        );
        assert!(started.elapsed() < Duration::from_millis(900));
        assert!(dir.path().join("chess.db.bak-1").exists());
    }

    // ---- FIND-RW1-06, FIND-RW1-08, FIND-RW1-12: the NF-2 copy ----

    /// An interrupted first creation: an empty database and a hot rollback journal, as a kill
    /// inside the first schema transaction leaves them. Returns the data directory.
    fn hot_journal_dir() -> tempfile::TempDir {
        let src = tempfile::TempDir::new().unwrap();
        let conn = Connection::open(src.path().join("chess.db")).unwrap();
        conn.execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE x (a); DROP TABLE x;")
            .unwrap();
        conn.execute_batch("PRAGMA cache_size=1; BEGIN; CREATE TABLE zz_inflight (x);")
            .unwrap();
        for _ in 0..50 {
            conn.execute("INSERT INTO zz_inflight VALUES (zeroblob(4000))", [])
                .unwrap();
        }
        let dir = tempfile::TempDir::new().unwrap();
        fs::copy(src.path().join("chess.db"), dir.path().join("chess.db")).unwrap();
        fs::copy(
            src.path().join("chess.db-journal"),
            dir.path().join("chess.db-journal"),
        )
        .unwrap();
        // The journal larger than the database (zero padding after its records is harmless to
        // SQLite), so that either file can be the one that decides.
        let size = len(&dir.path().join("chess.db")) + 4096;
        fs::OpenOptions::new()
            .write(true)
            .open(dir.path().join("chess.db-journal"))
            .unwrap()
            .set_len(size)
            .unwrap();
        dir
    }

    fn len(path: &Path) -> u64 {
        fs::metadata(path).unwrap().len()
    }

    fn with_cap<T>(cap: u64, body: impl FnOnce() -> T) -> T {
        NF2_CAP_OVERRIDE.with(|c| c.set(Some(cap)));
        let out = body();
        NF2_CAP_OVERRIDE.with(|c| c.set(None));
        out
    }

    /// M40, M41: a file exactly at the cap is copied, one byte above is refused, and the rule
    /// holds for the database and for the journal alike.
    #[test]
    fn nf2_the_cap_boundary_is_exact_for_the_database_and_for_the_journal() {
        let dir = hot_journal_dir();
        let db = dir.path().join("chess.db");
        let journal = dir.path().join("chess.db-journal");
        let (db_len, jnl_len) = (len(&db), len(&journal));
        assert!(
            jnl_len > db_len,
            "the fixture: journal larger than database"
        );
        assert!(with_cap(jnl_len, || {
            rollback_copy_holds_no_players_or_games(&db)
        }));
        assert!(!with_cap(jnl_len - 1, || {
            rollback_copy_holds_no_players_or_games(&db)
        }));
        // Database larger: grow it by whole pages of zeros (the header page count still rules).
        let big = db_len + 3 * 4096;
        fs::OpenOptions::new()
            .write(true)
            .open(&db)
            .unwrap()
            .set_len(big)
            .unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&journal)
            .unwrap()
            .set_len(jnl_len.min(big - 1))
            .unwrap();
        let cap = big.max(len(&journal));
        assert!(with_cap(cap, || rollback_copy_holds_no_players_or_games(
            &db
        )));
        assert!(!with_cap(big - 1, || {
            rollback_copy_holds_no_players_or_games(&db)
        }));
    }

    /// FIND-RW1-06: the copy itself is bounded. The journal grows between the size check and
    /// the read (the hook runs in that gap): the copy stops at the cap, answers "cannot tell"
    /// and the scratch directory is gone.
    #[test]
    fn nf2_a_file_that_grows_after_the_size_check_is_not_copied_past_the_cap() {
        let dir = hot_journal_dir();
        let db = dir.path().join("chess.db");
        let journal = dir.path().join("chess.db-journal");
        let cap = len(&db).max(len(&journal));
        let grow = journal.clone();
        NF2_SCRATCH_HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |_, stage| {
                if stage == "before_copy" {
                    let mut file = fs::OpenOptions::new().append(true).open(&grow).unwrap();
                    file.write_all(&[0]).unwrap();
                }
            }));
        });
        let answer = with_cap(cap, || rollback_copy_holds_no_players_or_games(&db));
        NF2_SCRATCH_HOOK.with(|h| *h.borrow_mut() = None);
        assert!(!answer, "a grown file is refused");
        assert_eq!(len(&journal), cap + 1, "the real file was only read");
    }

    /// M44: the scratch directory is `0700` and the copies are `0600` while they exist; the
    /// whole directory is gone afterwards.
    #[test]
    fn nf2_the_scratch_directory_is_private_and_removed() {
        use std::os::unix::fs::PermissionsExt;
        let dir = hot_journal_dir();
        let db = dir.path().join("chess.db");
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = std::rc::Rc::clone(&seen);
        NF2_SCRATCH_HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |scratch, stage| {
                let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
                let mut entry = vec![(stage.to_owned(), mode(scratch))];
                if stage == "after_copy" {
                    entry.push(("db".into(), mode(&scratch.join("chess.db"))));
                    entry.push(("journal".into(), mode(&scratch.join("chess.db-journal"))));
                }
                log.borrow_mut().push((scratch.to_path_buf(), entry));
            }));
        });
        assert!(rollback_copy_holds_no_players_or_games(&db));
        NF2_SCRATCH_HOOK.with(|h| *h.borrow_mut() = None);
        let seen = seen.borrow();
        assert_eq!(seen.len(), 2, "both stages ran");
        assert_eq!(seen[0].1, vec![("before_copy".to_owned(), 0o700)]);
        assert_eq!(
            seen[1].1,
            vec![
                ("after_copy".to_owned(), 0o700),
                ("db".to_owned(), 0o600),
                ("journal".to_owned(), 0o600)
            ]
        );
        assert!(!seen[0].0.exists(), "removed after the check");
    }

    fn scratch_dir(base: &Path, name: &str, mode: u32) -> std::path::PathBuf {
        use std::os::unix::fs::DirBuilderExt;
        let path = base.join(name);
        fs::DirBuilder::new().mode(mode).create(&path).unwrap();
        fs::write(path.join("chess.db"), b"a copy of a database").unwrap();
        path
    }

    /// FIND-RW1-08: a scratch directory of a dead process is swept; every look-alike is left.
    #[test]
    fn nf2_the_sweep_removes_only_private_scratch_of_dead_processes() {
        use std::os::unix::fs::MetadataExt;
        let base = tempfile::TempDir::new().unwrap();
        let uid = fs::metadata(base.path()).unwrap().uid();
        let own = std::process::id();
        let dead = scratch_dir(base.path(), "chess-nf2-4000001-0", 0o700);
        let dead_second = scratch_dir(base.path(), "chess-nf2-4000001-7", 0o700);
        let live = scratch_dir(base.path(), "chess-nf2-4000002-0", 0o700);
        let mine = scratch_dir(base.path(), &format!("chess-nf2-{own}-0"), 0o700);
        let loose = scratch_dir(base.path(), "chess-nf2-4000003-0", 0o755);
        let no_counter = scratch_dir(base.path(), "chess-nf2-4000004", 0o700);
        let bad_pid = scratch_dir(base.path(), "chess-nf2-abc-0", 0o700);
        let other = scratch_dir(base.path(), "other-4000001-0", 0o700);
        let victim = scratch_dir(base.path(), "victim", 0o700);
        std::os::unix::fs::symlink(&victim, base.path().join("chess-nf2-4000005-0")).unwrap();
        fs::write(base.path().join("chess-nf2-4000006-0"), b"a file").unwrap();
        let alive = |pid: u32| pid == 4_000_002;
        assert_eq!(sweep_stale_nf2_scratch(base.path(), uid, &alive), 2);
        assert!(!dead.exists() && !dead_second.exists());
        for kept in [&live, &mine, &loose, &no_counter, &bad_pid, &other, &victim] {
            assert!(kept.join("chess.db").exists(), "{kept:?} stays");
        }
        assert!(base.path().join("chess-nf2-4000005-0").exists(), "symlink");
        assert!(base.path().join("chess-nf2-4000006-0").exists(), "file");
        // Another owner's uid: nothing of ours is touched.
        let again = scratch_dir(base.path(), "chess-nf2-4000007-0", 0o700);
        assert_eq!(sweep_stale_nf2_scratch(base.path(), uid + 1, &alive), 0);
        assert!(again.exists());
        // A missing base is no error.
        assert_eq!(
            sweep_stale_nf2_scratch(&base.path().join("absent"), uid, &alive),
            0
        );
    }

    /// M51: the schema counts as appeared when the version alone is set, and when a table alone
    /// is there; with neither the verdict is the refusal.
    #[test]
    fn nf4_the_schema_has_appeared_for_a_version_alone_or_a_table_alone() {
        let case = |sql: &str| {
            let dir = tempfile::TempDir::new().unwrap();
            let db = dir.path().join("chess.db");
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch("PRAGMA journal_mode=DELETE").unwrap();
            conn.execute_batch(sql).unwrap();
            fs::write(dir.path().join("chess.db.bak-1"), b"x").unwrap();
            let probe = read_only_probe(&db).unwrap();
            let verdict = nf4_verdict(&DataDir::new(dir.path()), &probe).unwrap();
            (verdict, dir)
        };
        let (version_only, _d1) = case("PRAGMA user_version = 1");
        assert!(matches!(version_only, Nf4::SchemaAppeared));
        let (table_only, _d2) = case("CREATE TABLE t (a)");
        assert!(matches!(table_only, Nf4::SchemaAppeared));
        let (neither, _d3) = case("CREATE TABLE t (a); DROP TABLE t");
        assert!(matches!(neither, Nf4::Refuse));
    }

    /// Tables with `user_version` 0 after the re-read are not our database.
    #[test]
    fn r05_tables_with_version_zero_after_the_second_read_are_corrupt() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = dir.path().join("chess.db");
        let result = with_hook(
            move || {
                Connection::open(&db)
                    .unwrap()
                    .execute_batch("CREATE TABLE foreign_table (x)")
                    .unwrap();
            },
            || open_dir(&dir),
        );
        assert!(
            matches!(result, Err(StoreError::Corrupt)),
            "{:?}",
            result.err()
        );
    }

    /// A newer schema committed between the reads is `SchemaTooNew`, not a first launch.
    #[test]
    fn r05_a_newer_schema_committed_between_the_reads_is_schema_too_new() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = dir.path().join("chess.db");
        let result = with_hook(
            move || {
                let c = Connection::open(&db).unwrap();
                c.execute_batch("CREATE TABLE t (x)").unwrap();
                c.pragma_update(None, "user_version", 9).unwrap();
            },
            || open_dir(&dir),
        );
        assert!(
            matches!(result, Err(StoreError::SchemaTooNew { db: 9, app: 1 })),
            "{:?}",
            result.err()
        );
    }

    // ---- NF-7: busy_timeout before the WAL switch and on the probe ----

    /// The write connection waits for a lock that another connection holds while it switches to
    /// WAL: with the timeout set first it succeeds after the holder lets go; with it set after
    /// the switch it would fail `Busy` at once (M16).
    #[test]
    fn the_write_connection_waits_for_a_lock_during_the_wal_switch() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = dir.path().join("chess.db");
        File::create(&db).unwrap();
        let holder = Connection::open(&db).unwrap();
        holder.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let started = std::time::Instant::now();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            holder.execute_batch("COMMIT").unwrap();
        });
        let conn = open_write_connection(&db);
        release.join().unwrap();
        assert!(conn.is_ok(), "{:?}", conn.err());
        assert!(started.elapsed() >= Duration::from_millis(250), "it waited");
    }

    /// The read-only probe waits too (M17, M18): a database locked exclusively for 300 ms does
    /// not make the open fail `Busy`.
    #[test]
    fn the_read_only_probe_and_read_connections_wait_for_a_lock() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = dir.path().join("chess.db");
        {
            let c = open_write_connection(&db).unwrap();
            c.execute_batch(crate::migrate::MIGRATIONS[0].sql).unwrap();
            c.pragma_update(None, "user_version", 1).unwrap();
        }
        let read = open_read_connection(&db).unwrap();
        let busy: i64 = read
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
            .unwrap();
        assert_eq!(busy, 5000, "read connection");
        let holder = Connection::open(&db).unwrap();
        holder.execute_batch("PRAGMA journal_mode=DELETE").unwrap();
        holder.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            holder.execute_batch("COMMIT").unwrap();
        });
        let started = std::time::Instant::now();
        let probe = read_only_probe(&db);
        release.join().unwrap();
        assert!(probe.is_ok(), "{:?}", probe.err());
        assert!(
            started.elapsed() >= Duration::from_millis(250),
            "the probe waited"
        );
        let probe_busy: i64 = probe
            .unwrap()
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
            .unwrap();
        assert_eq!(probe_busy, 5000, "probe connection");
    }

    #[test]
    fn a_passing_probe_is_ok() {
        let dir = tempfile::TempDir::new().unwrap();
        probe_durability_with(dir.path(), &|_| Ok(())).unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn new_store_uuids_differ_and_are_version_4_shaped() {
        let a = new_store_uuid();
        let b = new_store_uuid();
        assert_ne!(a, b);
        assert_eq!(a.get_version_num(), 4);
    }

    #[test]
    fn the_probe_leaves_no_scratch_file_behind() {
        let dir = tempfile::TempDir::new().unwrap();
        let data = DataDir::new(dir.path());
        probe_durability(&data).unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
