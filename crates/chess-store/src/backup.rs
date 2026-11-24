//! Part A of DD-ST-020: the verified copy that the pre-upgrade backup needs (SDD 3.11.8,
//! "Writing a backup" and the dated note of 2026-11-09, D-108). The automatic backup, pruning,
//! the 24 h rule and the collection of `journal/done/` are part C (chess-a0r.3.8) and extend
//! this module; they are not here.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::datadir::{create_new_0600, sqlite_path};
use crate::error::BackupCause;
use crate::fault;

/// The temp-name stem for a pre-upgrade copy (D-108): `chess.db.bak-v<N>` becomes
/// `chess.db.tmp-bak-v<N>`. A name that does not follow that pattern (tests pass other names)
/// gets the suffix `.tmp-bak`. Neither form matches `chess.db.bak-v*`, so a temp file is never
/// taken for a backup.
fn temp_stem(target: &Path) -> Option<String> {
    let name = target.file_name()?.to_str()?;
    Some(if name.contains(".bak-v") {
        name.replacen(".bak-v", ".tmp-bak-v", 1)
    } else {
        format!("{name}.tmp-bak")
    })
}

/// This process's temp name: the stem, `-`, the pid (NF-1, review of store part A rework 1).
/// Two processes that upgrade at the same time each write their own temp file, so neither can
/// destroy the other's attempt; the rename over the target keeps the D-108 semantics.
pub(crate) fn temp_name_for(target: &Path) -> Option<PathBuf> {
    let stem = temp_stem(target)?;
    Some(target.with_file_name(format!("{stem}-{}", std::process::id())))
}

/// Removes temp files that a crash left: the part A name without a pid, and `<stem>-<pid>` of
/// a pid that no longer exists. A temp file of a live process (another upgrader) and this
/// process's own are left to their owner. Errors are ignored: a leftover is clutter, not harm.
fn sweep_stale_temps(target: &Path) {
    let (Some(stem), Some(dir)) = (temp_stem(target), target.parent()) else {
        return;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let own = std::process::id();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let stale = if name == stem {
            true
        } else if let Some(pid) = name
            .strip_prefix(&format!("{stem}-"))
            .and_then(|rest| rest.parse::<u32>().ok())
        {
            pid != own && !crate::proc::pid_alive(pid)
        } else {
            false
        };
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// D-108: replaces `target` with a verified copy of `source`, atomically. Order: remove a stale
/// temp file; write and verify the copy at the temp name (`write_verified_copy`, which syncs
/// the file); `rename` it over `target`; sync the directory. Only the rename touches the old
/// copy, so a failed attempt leaves it byte-identical and removes the temp file. A crash
/// before the rename leaves the old copy and a stale temp file, which the next attempt removes.
pub(crate) fn write_replacing_copy(source: &Connection, target: &Path) -> Result<(), BackupCause> {
    let temp = temp_name_for(target).ok_or(BackupCause::Io(io::ErrorKind::InvalidInput))?;
    sweep_stale_temps(target);
    match fs::remove_file(&temp) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(BackupCause::Io(e.kind())),
    }
    write_verified_copy(source, &temp)?;
    // Failpoint `backup.before_rename` (D-108, CT-ST-090): a kill here leaves the old copy and a
    // stale temp file. A failure is handled like a failed rename (the temp file is removed).
    if let Err(e) =
        fault::check("backup.before_rename", target).and_then(|()| fs::rename(&temp, target))
    {
        let _ = fs::remove_file(&temp);
        return Err(BackupCause::Io(e.kind()));
    }
    if let Some(dir) = target.parent() {
        fault::sync_dir_named("sync.backup_rename_dir", dir)
            .map_err(|e| BackupCause::Io(e.kind()))?;
    }
    Ok(())
}

/// Writes `VACUUM INTO target` from `source`, then proves the copy: it opens read-only, its
/// `PRAGMA quick_check` is `ok` and its `user_version` equals the source's. The target is created
/// empty with mode `0600` first (`VACUUM INTO` needs a target that is absent or empty) and an
/// existing target is never overwritten. On any failure after creation the partial file is
/// removed. The file and its directory are fully synced before returning.
pub(crate) fn write_verified_copy(source: &Connection, target: &Path) -> Result<(), BackupCause> {
    let safe = sqlite_path(target);
    let target_text = safe
        .to_str()
        .ok_or(BackupCause::Io(io::ErrorKind::InvalidInput))?;
    match create_new_0600(target) {
        Ok(file) => drop(file),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Err(BackupCause::TargetExists);
        }
        Err(e) => return Err(BackupCause::Io(e.kind())),
    }
    let outcome = copy_and_verify(source, target, target_text);
    if outcome.is_err() {
        // Best effort: a partial file must not be mistaken for a backup.
        let _ = fs::remove_file(target);
    }
    outcome
}

fn copy_and_verify(
    source: &Connection,
    target: &Path,
    target_text: &str,
) -> Result<(), BackupCause> {
    source
        .execute("VACUUM INTO ?1", [target_text])
        .map_err(|_| BackupCause::CopyFailed)?;
    let expected: i64 = source
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| BackupCause::VerifyFailed)?;
    verify_copy(target, expected)?;
    File::open(target)
        .and_then(|f| fault::sync_file("sync.backup_copy_file", target, &f))
        .map_err(|e| BackupCause::Io(e.kind()))?;
    if let Some(dir) = target.parent() {
        fault::sync_dir_named("sync.backup_copy_dir", dir)
            .map_err(|e| BackupCause::Io(e.kind()))?;
    }
    Ok(())
}

/// The copy opens read-only, its `PRAGMA quick_check` is `ok` and its `user_version` is
/// `expected` (`VACUUM INTO` carries it over).
fn verify_copy(target: &Path, expected: i64) -> Result<(), BackupCause> {
    // Failpoint `backup.verify` (CT-ST-090: "a copy that fails verification (injected)").
    fault::check("backup.verify", target).map_err(|_| BackupCause::VerifyFailed)?;
    let copy = Connection::open_with_flags(sqlite_path(target), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| BackupCause::VerifyFailed)?;
    let check: String = copy
        .pragma_query_value(None, "quick_check", |row| row.get(0))
        .map_err(|_| BackupCause::VerifyFailed)?;
    let version: i64 = copy
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| BackupCause::VerifyFailed)?;
    if check == "ok" && version == expected {
        Ok(())
    } else {
        Err(BackupCause::VerifyFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// The temp file this process uses for `chess.db.bak-v<n>` in `dir`.
    fn temp_in(dir: &TempDir, n: u32) -> PathBuf {
        temp_name_for(&dir.path().join(format!("chess.db.bak-v{n}"))).unwrap()
    }

    /// A pid that existed and is gone.
    fn dead_pid() -> u32 {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        pid
    }

    fn source_with_version(dir: &TempDir, version: i64) -> Connection {
        let conn = Connection::open(dir.path().join("src.db")).unwrap();
        conn.execute_batch("CREATE TABLE t (x); INSERT INTO t VALUES (1);")
            .unwrap();
        conn.pragma_update(None, "user_version", version).unwrap();
        conn
    }

    #[test]
    fn a_good_copy_is_written_0600_and_verified() {
        let dir = TempDir::new().unwrap();
        let source = source_with_version(&dir, 3);
        let target = dir.path().join("copy.db");
        write_verified_copy(&source, &target).unwrap();
        verify_copy(&target, 3).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn the_temp_name_never_matches_the_backup_pattern() {
        let pid = std::process::id();
        let t = temp_name_for(Path::new("/d/chess.db.bak-v7")).unwrap();
        assert_eq!(t, Path::new(&format!("/d/chess.db.tmp-bak-v7-{pid}")));
        let other = temp_name_for(Path::new("/d/good.bak")).unwrap();
        assert_eq!(other, Path::new(&format!("/d/good.bak.tmp-bak-{pid}")));
        for p in [t, other] {
            assert!(
                !p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("chess.db.bak-v")
            );
        }
        assert_eq!(temp_name_for(Path::new("/")), None);
    }

    #[test]
    fn replacing_swaps_in_a_verified_copy_and_leaves_no_temp_file() {
        let dir = TempDir::new().unwrap();
        let source = source_with_version(&dir, 3);
        let target = dir.path().join("chess.db.bak-v3");
        fs::write(&target, b"old copy").unwrap();
        // Stale temps: the part A name, this process's own, and one of a pid that is gone.
        let gone = dead_pid();
        let stale = [
            "chess.db.tmp-bak-v3".to_owned(),
            format!("chess.db.tmp-bak-v3-{}", std::process::id()),
            format!("chess.db.tmp-bak-v3-{gone}"),
        ];
        for name in &stale {
            fs::write(dir.path().join(name), b"stale").unwrap();
        }
        write_replacing_copy(&source, &target).unwrap();
        verify_copy(&target, 3).unwrap();
        for name in &stale {
            assert!(!dir.path().join(name).exists(), "{name}");
        }
    }

    /// NF-1: the temp file of a **live** other process (a concurrent upgrader) is not swept.
    #[test]
    fn a_live_processes_temp_file_is_left_alone() {
        let dir = TempDir::new().unwrap();
        let source = source_with_version(&dir, 3);
        let target = dir.path().join("chess.db.bak-v3");
        let mut other = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .unwrap();
        let theirs = dir
            .path()
            .join(format!("chess.db.tmp-bak-v3-{}", other.id()));
        fs::write(&theirs, b"in progress").unwrap();
        write_replacing_copy(&source, &target).unwrap();
        assert_eq!(fs::read(&theirs).unwrap(), b"in progress");
        other.kill().unwrap();
        other.wait().unwrap();
    }

    #[test]
    fn a_failed_replacement_keeps_the_old_copy_byte_identical_and_removes_the_temp() {
        let dir = TempDir::new().unwrap();
        let mut source = source_with_version(&dir, 1);
        let tx = source.transaction().unwrap(); // VACUUM INTO refuses inside a transaction
        let target = dir.path().join("chess.db.bak-v1");
        fs::write(&target, b"last good copy").unwrap();
        assert_eq!(
            write_replacing_copy(&tx, &target),
            Err(BackupCause::CopyFailed)
        );
        assert_eq!(fs::read(&target).unwrap(), b"last good copy");
        assert!(!temp_in(&dir, 1).exists());
    }

    #[test]
    fn a_failed_rename_removes_the_temp_file_and_reports_io() {
        let dir = TempDir::new().unwrap();
        let source = source_with_version(&dir, 1);
        // A non-empty directory at the target: rename of a file onto it fails.
        let target = dir.path().join("chess.db.bak-v1");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("x"), b"x").unwrap();
        assert!(matches!(
            write_replacing_copy(&source, &target),
            Err(BackupCause::Io(_))
        ));
        assert!(!temp_in(&dir, 1).exists());
        assert!(target.join("x").exists());
    }

    #[test]
    fn write_verified_copy_itself_never_overwrites() {
        let dir = TempDir::new().unwrap();
        let source = source_with_version(&dir, 1);
        let target = dir.path().join("copy.db");
        fs::write(&target, b"keep me").unwrap();
        assert_eq!(
            write_verified_copy(&source, &target),
            Err(BackupCause::TargetExists)
        );
        assert_eq!(fs::read(&target).unwrap(), b"keep me");
    }

    #[test]
    fn verification_rejects_a_file_that_is_not_a_database() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("junk.db");
        fs::write(&target, vec![0xABu8; 8192]).unwrap();
        assert_eq!(verify_copy(&target, 0), Err(BackupCause::VerifyFailed));
    }

    #[test]
    fn verification_rejects_a_version_mismatch_and_a_missing_file() {
        let dir = TempDir::new().unwrap();
        let source = source_with_version(&dir, 5);
        let target = dir.path().join("copy.db");
        write_verified_copy(&source, &target).unwrap();
        assert_eq!(verify_copy(&target, 4), Err(BackupCause::VerifyFailed));
        assert_eq!(
            verify_copy(&dir.path().join("absent.db"), 5),
            Err(BackupCause::VerifyFailed)
        );
    }

    #[test]
    fn a_failed_copy_removes_the_partial_file() {
        let dir = TempDir::new().unwrap();
        // A source that cannot VACUUM INTO: inside an open transaction (SQLite refuses).
        let mut source = source_with_version(&dir, 1);
        let tx = source.transaction().unwrap();
        let target = dir.path().join("copy.db");
        assert_eq!(
            write_verified_copy(&tx, &target),
            Err(BackupCause::CopyFailed)
        );
        assert!(!target.exists(), "the partial file is removed");
    }
}
