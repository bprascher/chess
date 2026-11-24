//! File moves of the journal directory (SDD 3.11.3 dated note 2026-11-09, D-83, D-110).
//!
//! **The no-replace move** is `std::fs::hard_link(src, dst)` followed by
//! `std::fs::remove_file(src)`: `hard_link` onto an existing name fails with `AlreadyExists` and
//! changes nothing, which is the no-replace rule, and std needs no `unsafe`, `libc` or `rustix`
//! for it (D-107, spike OQ-14). Between the two calls the file has two names (link count 2);
//! the crash windows that creates are handled by `move_aside_damaged` (window e) and
//! `settle_rebase_files` (windows a, c, f), and the directory syncs are the caller's.

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::fault;

/// `hard_link` then `remove_file`: moves `src` to `dst` and never replaces `dst`.
pub(crate) fn move_no_replace(src: &Path, dst: &Path) -> io::Result<()> {
    fs::hard_link(src, dst)?;
    fs::remove_file(src)
}

/// True when both names are the same file (same device and inode).
pub(crate) fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    let (a, b) = (fs::metadata(a)?, fs::metadata(b)?);
    Ok(a.dev() == b.dev() && a.ino() == b.ino())
}

pub(crate) fn sync_dir(dir: &Path) -> io::Result<()> {
    fault::sync_dir_named("sync.journal_move_dir", dir)
}

pub(crate) fn journal_path(journal_dir: &Path, game: Uuid) -> PathBuf {
    journal_dir.join(format!("{}.jnl", game.hyphenated()))
}

pub(crate) fn rebase_path(journal_dir: &Path, game: Uuid) -> PathBuf {
    journal_dir.join(format!("{}.jnl.rebase", game.hyphenated()))
}

fn damaged_dir(journal_dir: &Path) -> PathBuf {
    journal_dir.join("damaged")
}

/// What `move_aside_damaged` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedAside {
    /// The name under `journal/damaged/`, `<game_uuid>.<k>.jnl`.
    pub damaged: PathBuf,
    /// `k`.
    pub k: u32,
    /// A previous attempt had already linked the file (window e); only the removal ran.
    pub finished_earlier_attempt: bool,
}

/// Step 3a to 3c of the rebase: moves the damaged `<uuid>.jnl` to `damaged/<uuid>.<k>.jnl`
/// (`k` the first unused integer, starting at 1), then syncs `journal/` and `journal/damaged/`
/// (RR-A-5). **Window (e):** if a `damaged/<uuid>.<k>.jnl` already is the same file as
/// `<uuid>.jnl` (a crash after the link and before the removal), the move is finished by the
/// removal alone and no second damaged file is made; a damaged file of another inode only
/// moves `k` on. The failpoint `journal.rebase.old_linked` sits between the link and the removal.
pub fn move_aside_damaged(journal_dir: &Path, game: Uuid) -> io::Result<MovedAside> {
    let src = journal_path(journal_dir, game);
    let dir = damaged_dir(journal_dir);
    crate::datadir::create_dir_0700(&dir)?;
    let name = |k: u32| dir.join(format!("{}.{k}.jnl", game.hyphenated()));
    let mut k = 1u32;
    let moved = loop {
        let dst = name(k);
        match fs::metadata(&dst) {
            Ok(_) => {
                if same_file(&src, &dst)? {
                    fs::remove_file(&src)?;
                    break MovedAside {
                        damaged: dst,
                        k,
                        finished_earlier_attempt: true,
                    };
                }
                k += 1;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => match fs::hard_link(&src, &dst) {
                Ok(()) => {
                    // 3a done, 3b not yet.
                    fault::check("journal.rebase.old_linked", &dst)?;
                    fs::remove_file(&src)?;
                    break MovedAside {
                        damaged: dst,
                        k,
                        finished_earlier_attempt: false,
                    };
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => k += 1,
                Err(e) => return Err(e),
            },
            Err(e) => return Err(e),
        }
    };
    sync_dir(journal_dir)?;
    sync_dir(&dir)?;
    Ok(moved)
}

/// What `settle_rebase_files` found and did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    /// No `.rebase` file: nothing to do.
    Nothing,
    /// A `.rebase` file beside `<uuid>.jnl` was removed: windows (a) and (b) (it will be built
    /// again, same bytes) or (f) (the same file under two names, step 3e finished).
    RemovedStaleRebase,
    /// `<uuid>.jnl` was absent and `.rebase` was complete (it was synced before the database
    /// step): window (c). It was moved into place.
    RenamedIntoPlace,
}

/// The file-level repair at startup for the rebase crash windows (SDD 3.11.3, D-110):
/// (a), (b) and (f): `<uuid>.jnl` exists and a `.rebase` exists, so the `.rebase` is removed
/// (a rebuilt one has the same bytes; a link of the same inode is only a second name);
/// (c): `<uuid>.jnl` is absent and `.rebase` exists, so it is moved into place without
/// replacing anything. The caller holds the game's lock. Window (e) is `move_aside_damaged`.
pub fn settle_rebase_files(journal_dir: &Path, game: Uuid) -> io::Result<Settled> {
    let jnl = journal_path(journal_dir, game);
    let rebase = rebase_path(journal_dir, game);
    if fs::symlink_metadata(&rebase).is_err() {
        return Ok(Settled::Nothing);
    }
    if fs::symlink_metadata(&jnl).is_ok() {
        fs::remove_file(&rebase)?;
        sync_dir(journal_dir)?;
        Ok(Settled::RemovedStaleRebase)
    } else {
        move_no_replace(&rebase, &jnl)?;
        sync_dir(journal_dir)?;
        Ok(Settled::RenamedIntoPlace)
    }
}
