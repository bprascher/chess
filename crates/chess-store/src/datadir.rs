//! DD-ST-004: the on-disk layout, permissions and directory resolution (architecture 7.1,
//! D-36; SDD 3.5.1). The data directory is created `0700`; files are created `0600`. An
//! existing directory with looser permissions is reported and never `chmod`ed: it is the
//! user's (SDD section 4, DD-ST-004).

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{StoreError, map_io};

/// Mode of every directory the store creates.
pub const DIR_MODE: u32 = 0o700;
/// Mode of every file the store creates.
pub const FILE_MODE: u32 = 0o600;

/// The data directory (D-36). A cheap, clonable path wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDir {
    root: PathBuf,
}

/// What `ensure` found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnsureReport {
    /// The directory did not exist and was created.
    pub created: bool,
    /// The directory exists and has group or other permission bits. Never changed.
    pub loose_permissions: bool,
}

/// `store.id`: written when a store is created (architecture 7.1, Q51; SDD 3.5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoreId {
    pub store_uuid: Uuid,
    /// Creation time, Unix epoch milliseconds.
    pub created_at: i64,
}

impl DataDir {
    pub fn new(root: impl Into<PathBuf>) -> DataDir {
        DataDir { root: root.into() }
    }

    /// Precedence of D-36 and CLI-4: (1) the `--data-dir` flag, (2) `CHESS_DATA_DIR`, (3) the
    /// default `<home>/Library/Application Support/Chess`. An empty environment value counts as
    /// unset. Returns `None` only when no source gives a directory (no flag, no env, no home).
    /// Pure: the caller passes the values it read, so tests need no process environment.
    pub fn resolve(
        flag: Option<&Path>,
        env_value: Option<&OsStr>,
        home: Option<&Path>,
    ) -> Option<DataDir> {
        if let Some(flag) = flag.filter(|p| !p.as_os_str().is_empty()) {
            return Some(DataDir::new(flag));
        }
        if let Some(env) = env_value.filter(|v| !v.is_empty()) {
            return Some(DataDir::new(env));
        }
        home.filter(|h| !h.as_os_str().is_empty())
            .map(|h| DataDir::new(h.join("Library/Application Support/Chess")))
    }

    /// `resolve` with the process environment (`CHESS_DATA_DIR`, `HOME`).
    pub fn resolve_from_env(flag: Option<&Path>) -> Option<DataDir> {
        let env_value = std::env::var_os("CHESS_DATA_DIR");
        let home = std::env::var_os("HOME").map(PathBuf::from);
        DataDir::resolve(flag, env_value.as_deref(), home.as_deref())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db_path(&self) -> PathBuf {
        self.root.join("chess.db")
    }

    pub fn store_id_path(&self) -> PathBuf {
        self.root.join("store.id")
    }

    pub fn journal_dir(&self) -> PathBuf {
        self.root.join("journal")
    }

    pub fn journal_done_dir(&self) -> PathBuf {
        self.journal_dir().join("done")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    /// `chess.db.bak-v<N>`: the pre-upgrade copy of schema version `N`, never pruned.
    pub fn pre_upgrade_backup_path(&self, version: u32) -> PathBuf {
        self.root.join(format!("chess.db.bak-v{version}"))
    }

    /// Creates the directory (mode `0700`) if it is missing and reports what it found. An existing
    /// directory is never `chmod`ed.
    pub fn ensure_root(&self) -> Result<EnsureReport, StoreError> {
        let existed = match fs::metadata(&self.root) {
            Ok(meta) if meta.is_dir() => true,
            Ok(_) => {
                return Err(StoreError::Unavailable {
                    cause: crate::error::UnavailableCause::Io(io::ErrorKind::NotADirectory),
                });
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => false,
            Err(e) => return Err(map_io(&e)),
        };
        if !existed {
            create_dir_0700(&self.root).map_err(|e| map_io(&e))?;
        }
        let mode = fs::metadata(&self.root).map_err(|e| map_io(&e))?.mode();
        let loose = mode & 0o077 != 0 && existed;
        if loose {
            tracing::warn!(
                mode = format_args!("{:o}", mode & 0o777),
                "data directory permissions are looser than 0700; left unchanged"
            );
        }
        Ok(EnsureReport {
            created: !existed,
            loose_permissions: loose,
        })
    }

    /// Creates the part-A layout inside an existing root: `journal/`, `journal/done/` and
    /// `backups/`, mode `0700`. Other directories (`journal/damaged/`, `journal/unrestored/`,
    /// `missing-<date>/`) are created by their owners when needed.
    pub fn ensure_layout(&self) -> Result<(), StoreError> {
        for dir in [
            self.journal_dir(),
            self.journal_done_dir(),
            self.backups_dir(),
        ] {
            create_dir_0700(&dir).map_err(|e| map_io(&e))?;
        }
        Ok(())
    }

    /// `ensure_root` then `ensure_layout`.
    pub fn ensure(&self) -> Result<EnsureReport, StoreError> {
        let report = self.ensure_root()?;
        self.ensure_layout()?;
        Ok(report)
    }

    /// Reads `store.id`; `Ok(None)` when absent, `Corrupt` when it does not parse.
    pub fn read_store_id(&self) -> Result<Option<StoreId>, StoreError> {
        match fs::read(self.store_id_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| StoreError::Corrupt),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(map_io(&e)),
        }
    }

    /// Writes `store.id` (`0600`) atomically and without replacing: the content goes to a
    /// temp file in the directory, is synced, and is then `hard_link`ed to `store.id`, so a
    /// reader never sees a torn file and the first writer wins. An existing `store.id` is
    /// `Unavailable{Io(AlreadyExists)}` and is left as it was. The format is unchanged
    /// (review FIND-007).
    pub fn write_store_id(&self, id: &StoreId) -> Result<(), StoreError> {
        if self.write_store_id_once(id)? {
            Ok(())
        } else {
            Err(StoreError::Unavailable {
                cause: crate::error::UnavailableCause::Io(io::ErrorKind::AlreadyExists),
            })
        }
    }

    /// `write_store_id` with the two meanings of "already exists" told apart (NF-3): `Ok(true)`
    /// this call created `store.id`; `Ok(false)` `store.id` was already there (the first writer
    /// wins and its file stands). A temp name that is already taken is a stale file of a crashed
    /// process that had this pid (the name holds this pid and a counter that only this process
    /// advances), so it is removed and the next name is used; it never reads as "store.id
    /// exists".
    pub(crate) fn write_store_id_once(&self, id: &StoreId) -> Result<bool, StoreError> {
        static TEMPS: AtomicU64 = AtomicU64::new(0);
        let bytes = serde_json::to_vec(id).map_err(|_| StoreError::Internal)?;
        let mut attempts = 0;
        let (temp, mut file) = loop {
            let temp = self.root.join(format!(
                ".store.id.tmp-{}-{}",
                std::process::id(),
                TEMPS.fetch_add(1, Ordering::Relaxed)
            ));
            match create_new_0600(&temp) {
                Ok(file) => break (temp, file),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempts < 16 => {
                    attempts += 1;
                    let _ = fs::remove_file(&temp);
                }
                Err(e) => return Err(map_io(&e)),
            }
        };
        let written = file
            .write_all(&bytes)
            .and_then(|()| crate::fault::sync_file("sync.store_id_file", &temp, &file));
        drop(file);
        let linked = written.and_then(|()| fs::hard_link(&temp, self.store_id_path()));
        let _ = fs::remove_file(&temp);
        match linked {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
            Err(e) => return Err(map_io(&e)),
        }
        crate::fault::sync_dir_named("sync.store_id_dir", &self.root).map_err(|e| map_io(&e))?;
        Ok(true)
    }

    /// D1-01 (store part D1 evidence): removes scratch files that a crashed process left in the
    /// data directory, `.durability-probe-<pid>-<n>` and `.store.id.tmp-<pid>-<n>`, when their
    /// pid no longer exists. A file of this process or of a live process may be in use and is
    /// left. Best effort: errors are ignored. Returns the number removed.
    pub(crate) fn remove_stale_scratch(&self, alive: &dyn Fn(u32) -> bool) -> usize {
        const PREFIXES: [&str; 2] = [".durability-probe-", ".store.id.tmp-"];
        let Ok(entries) = fs::read_dir(&self.root) else {
            return 0;
        };
        let own = std::process::id();
        let mut verdicts = std::collections::HashMap::new();
        let mut removed = 0;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(pid) = PREFIXES
                .iter()
                .find_map(|prefix| name.strip_prefix(prefix))
                .and_then(|rest| rest.split('-').next())
                .and_then(|pid| pid.parse::<u32>().ok())
            else {
                continue;
            };
            if pid == own {
                continue;
            }
            let live = *verdicts.entry(pid).or_insert_with(|| alive(pid));
            if !live && fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
        removed
    }

    /// NF-2 (narrowed, FIND-B-06): true when the directory shows the state of a first launch that
    /// was cut off inside the switch to WAL, and nothing else: a non-empty rollback journal
    /// `chess.db-journal` and **no** provenance trace (`data_traces`: nothing in `journal/` or
    /// `backups/`, no `chess.db.bak*`, `chess.db.tmp-bak-v*`, `chess.db.pre-import-*` or
    /// `chess.db.restoring`) and no content in `chess.db-wal`. This is conditions B to F of the
    /// rule; the database must also hold no players and no games (G, checked by the opener on a
    /// copy) before the journal is rolled back through a read-write connection.
    pub(crate) fn is_interrupted_first_creation(&self) -> Result<bool, StoreError> {
        let traces = self.data_traces()?;
        Ok(traces.hot_journal && !traces.provenance && !traces.wal)
    }

    /// NF-4 and NF-2, two classes (ruling 2026-11-23, FIND-RW1-01; package section (a)).
    ///
    /// **Provenance traces** (`provenance`): traces of an earlier store's data that a first
    /// creation can never produce, and that make an **empty** `chess.db` suspicious: content in
    /// `journal/` (including `done/`, `damaged/`, `unrestored/`), a non-empty `backups/`, a
    /// `chess.db.bak*`, `chess.db.tmp-bak-v*`, `chess.db.pre-import-*` or `chess.db.restoring`.
    ///
    /// **SQLite sidecars** (`wal`, `hot_journal`: a `chess.db-wal` or `chess.db-journal` with
    /// content) are reported as facts and are **not** traces on their own: a creation cut off
    /// inside its first transaction leaves them, and they hold no user data. The opener judges
    /// them by what SQLite reads (the probe) and, for a zero-length `chess.db` beside a `-wal`,
    /// refuses before SQLite deletes that WAL (`open_blocking`).
    ///
    /// Benign names (`.DS_Store`, `._*`, zero-length sidecars, `-shm`, `store.id`, `logs/`,
    /// `config.toml`, `missing-*`, scratch names) are neither. Read-only.
    pub(crate) fn data_traces(&self) -> Result<DataTraces, StoreError> {
        let mut traces = DataTraces::default();
        for entry in fs::read_dir(&self.root).map_err(|e| map_io(&e))? {
            let entry = entry.map_err(|e| map_io(&e))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let has_content = |entry: &fs::DirEntry| content_of(entry.metadata().map(|m| m.len()));
            if name.starts_with("chess.db.bak")
                || name.starts_with("chess.db.tmp-bak-v")
                || name.starts_with("chess.db.pre-import-")
                || name == "chess.db.restoring"
            {
                traces.provenance = true;
            } else if name == "chess.db-wal" {
                traces.wal = has_content(&entry);
            } else if name == "chess.db-journal" {
                traces.hot_journal = has_content(&entry);
            }
        }
        traces.provenance = traces.provenance
            || dir_has_entries(&self.backups_dir())?
            || journal_has_content(&self.journal_dir())?;
        Ok(traces)
    }

    /// True when anything other than an empty layout of an earlier store is present: the
    /// signals of "StoreMissing" (Q51, architecture 7.1): `store.id`, any `chess.db*` file
    /// (WAL, backups), a non-empty `backups/` or a non-empty `journal/`.
    pub fn has_traces_of_a_store(&self) -> Result<bool, StoreError> {
        if self.store_id_path().exists() {
            return Ok(true);
        }
        let entries = fs::read_dir(&self.root).map_err(|e| map_io(&e))?;
        for entry in entries {
            let entry = entry.map_err(|e| map_io(&e))?;
            if entry.file_name().to_string_lossy().starts_with("chess.db") {
                return Ok(true);
            }
        }
        Ok(dir_has_entries(&self.backups_dir())? || journal_has_content(&self.journal_dir())?)
    }
}

/// Does a file of this length (or of a length that could not be read) count as having content?
/// A file that cannot be examined is treated as having content: a trace is never ignored
/// because of an I/O error.
fn content_of(len: io::Result<u64>) -> bool {
    len.map_or(true, |len| len > 0)
}

/// What `DataDir::data_traces` found (NF-4, NF-2).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DataTraces {
    /// A provenance trace: journal content, backups, backup or import files (see `data_traces`).
    pub provenance: bool,
    /// A `chess.db-wal` with content. A sidecar, not a trace on its own.
    pub wal: bool,
    /// A `chess.db-journal` with content. A sidecar, not a trace on its own.
    pub hot_journal: bool,
}

fn journal_has_content(journal: &Path) -> Result<bool, StoreError> {
    // `journal/done/` exists in the fresh layout; only a file or a non-empty subdirectory
    // counts as a trace.
    let entries = match fs::read_dir(journal) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(map_io(&e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| map_io(&e))?;
        let path = entry.path();
        if is_os_metadata(&entry.file_name()) {
            continue;
        }
        if path.is_dir() {
            if dir_has_entries(&path)? {
                return Ok(true);
            }
        } else {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Finder and AppleDouble files (`.DS_Store`, `._*`) appear when a user browses the folder and
/// are not traces of a store (review FIND-008).
fn is_os_metadata(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    name == ".DS_Store" || name.starts_with("._")
}

fn dir_has_entries(dir: &Path) -> Result<bool, StoreError> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                if !is_os_metadata(&entry.map_err(|e| map_io(&e))?.file_name()) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(map_io(&e)),
    }
}

/// Creates `path` and missing parents with mode `0700`. An existing directory is left alone.
pub(crate) fn create_dir_0700(path: &Path) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(path)
}

/// A path as SQLite must see it (review FIND-009). The bundled SQLite is built with
/// `SQLITE_USE_URI` (libsqlite3-sys 0.39.1 `build.rs:184` `[OBS]`), so a name that starts with
/// `file:` is read as a URI whatever the open flags say. A relative path gets a `./` prefix, so
/// it never starts with `file:`; an absolute path starts with `/` and is left alone.
pub(crate) fn sqlite_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(".").join(path)
    }
}

/// Creates a new file with mode `0600`; fails if it exists. `OpenOptions` in std sets
/// close-on-exec (SDD 3.11.6; checked by a test in `open.rs` is not possible without a child
/// process, so part B's CT-ST-006 carries it `[UNV]`).
pub(crate) fn create_new_0600(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(path)
}

/// `File::sync_all` is `fcntl(F_FULLFSYNC)` on Apple targets, with no fallback to `fsync`
/// (OQ-14 spike, spike-oq14-fsync-flock.md section 3.2). Directory sync is the same call on a
/// directory handle.
pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FIND-RW1-12 (M60): an unreadable length counts as content; zero does not.
    #[test]
    fn a_file_whose_length_cannot_be_read_counts_as_content() {
        assert!(content_of(Err(io::Error::from(
            io::ErrorKind::PermissionDenied
        ))));
        assert!(content_of(Ok(1)));
        assert!(!content_of(Ok(0)));
    }

    /// D1-01: the number of scratch files removed is returned (a pid that is gone, either
    /// prefix); this process's and live pids' files are kept.
    #[test]
    fn remove_stale_scratch_counts_what_it_removed() {
        let dir = tempfile::TempDir::new().unwrap();
        let data = DataDir::new(dir.path());
        let own = std::process::id();
        for name in [
            ".durability-probe-4000001-0".to_owned(),
            ".store.id.tmp-4000001-1".to_owned(),
            ".store.id.tmp-4000002-1".to_owned(),
            format!(".durability-probe-{own}-0"),
            "unrelated".to_owned(),
        ] {
            fs::write(dir.path().join(name), b"x").unwrap();
        }
        let removed = data.remove_stale_scratch(&|pid| pid == 4_000_002);
        assert_eq!(removed, 2);
        assert!(
            dir.path().join(".store.id.tmp-4000002-1").exists(),
            "live pid"
        );
        assert!(
            dir.path()
                .join(format!(".durability-probe-{own}-0"))
                .exists()
        );
        assert!(dir.path().join("unrelated").exists());
    }
}
