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
use crate::journal::format::{HEADER_LEN, JournalRecord, Kind, Payload};
use crate::journal::reader::scan_bytes;

/// `hard_link` then `remove_file`: moves `src` to `dst` and never replaces `dst`.
pub(crate) fn move_no_replace(src: &Path, dst: &Path) -> io::Result<()> {
    fs::hard_link(src, dst)?;
    fs::remove_file(src)
}

/// True when both names are the same **regular** file (same device and inode). A link is never
/// followed (`symlink_metadata`) and a name that is not a regular file is never "the same file":
/// a symbolic link at `done/<uuid>.jnl` that points at the journal must not make a caller delete
/// the journal as "already moved" (FIND-D4R-03). `Err` when either name does not exist.
pub(crate) fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    let (a, b) = (fs::symlink_metadata(a)?, fs::symlink_metadata(b)?);
    Ok(a.is_file() && b.is_file() && a.dev() == b.dev() && a.ino() == b.ino())
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
        match fs::symlink_metadata(&dst) {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// No `.rebase` file: nothing to do.
    Nothing,
    /// A `.rebase` file beside `<uuid>.jnl` was removed: windows (a) and (b) (it will be built
    /// again, same bytes) or (f) (the same file under two names, step 3e finished).
    RemovedStaleRebase,
    /// `<uuid>.jnl` was absent and `.rebase` was complete (it was synced before the database
    /// step): window (c). It was moved into place.
    RenamedIntoPlace,
    /// B-R05: `<uuid>.jnl` was absent and the `.rebase` was **not** complete (it does not scan
    /// clean to its end, its header is not this game's and this store's, or it does not end as
    /// a rebase ends). It is not installed. Its bytes are kept: it was moved, without replacing
    /// anything, to `journal/damaged/<uuid>.<k>.rebase` (`k` the first unused integer).
    MovedAsideIncomplete { damaged: PathBuf },
}

/// B-R05, D-153: does `bytes` hold what `write_rebase_file` writes? The header names this game
/// and this store and the records scan clean to the end of the file (no torn tail, no damage),
/// and the last record is one that a rebase ends with: a terminal record (`GameEnd`, `Abort`,
/// `Voided`; the builder adds no `Clock` after one) or a `Clock` with no running side (the
/// closing `Clock`, which the builder also leaves alone when it already is the last record).
/// A file cut at a record boundary before the closing `Clock` ends in a `Move`, `Start` or
/// the like and is refused here.
///
/// **The shape alone cannot tell a cut file from a whole one** (FIND-D4R-05): a rebase that
/// copies a stopped `Clock` from the middle of the old journal has that shape at the boundary
/// after it, so a file cut there looks like a rebase of a journal that ends in a stopped
/// `Clock`. `journal_seq` is the other witness: step 2 of the rebase stores the new file's last
/// `seq` in `GAME.journal_seq` before the file is moved into place, so a file whose last `seq`
/// is **below** the stored value was cut after the store learned its length. With the row's
/// `journal_seq` given, such a file is incomplete. `None` (no row to ask) leaves the shape test
/// alone.
pub(crate) fn rebase_is_complete(
    bytes: &[u8],
    game: Uuid,
    store: Uuid,
    journal_seq: Option<u64>,
) -> bool {
    let Ok(scan) = scan_bytes(bytes, game, store) else {
        return false;
    };
    if scan.damaged.is_some()
        || scan.torn_tail_dropped
        || scan.intact_end != bytes.len()
        || bytes.len() <= HEADER_LEN
    {
        return false;
    }
    scan.records
        .last()
        .is_some_and(|last| ends_a_rebase(last) && journal_seq.is_none_or(|seq| last.seq >= seq))
}

fn ends_a_rebase(record: &JournalRecord) -> bool {
    matches!(record.kind(), Kind::GameEnd | Kind::Abort | Kind::Voided)
        || matches!(&record.payload, Payload::Clock(clock) if clock.running_side.is_none())
}

/// The file-level repair at startup for the rebase crash windows (SDD 3.11.3, D-110):
/// (a), (b) and (f): `<uuid>.jnl` exists and a `.rebase` exists, so the `.rebase` is removed
/// (a rebuilt one has the same bytes; a link of the same inode is only a second name);
/// (c): `<uuid>.jnl` is absent and `.rebase` exists, so it is moved into place without
/// replacing anything **if it is complete** (B-R05, see `rebase_is_complete`); an incomplete
/// one is moved to `journal/damaged/` with its bytes and is **not** installed, so a game never
/// resumes on half a rebase. The caller holds the game's lock. Window (e) is
/// `move_aside_damaged`.
///
/// **The choice for an incomplete `.rebase` (B-R05):** the bytes are kept, as for a damaged
/// journal (nothing of the game's files is ever deleted), under the name
/// `damaged/<uuid>.<k>.rebase`. The suffix `.rebase` keeps it apart from `<uuid>.<k>.jnl`, the
/// names that CT-ST-088 counts, and says what it is. The game then has no journal file; the
/// "row with no journal" path rebuilds one from the rows (the store is ahead of an empty
/// journal, D-66, so the rebase builds from the store).
pub fn settle_rebase_files(journal_dir: &Path, game: Uuid, store: Uuid) -> io::Result<Settled> {
    settle_rebase_files_at(journal_dir, game, store, None)
}

/// `settle_rebase_files` with the row's `GAME.journal_seq` for the completeness test of a
/// `.rebase` that is about to be installed (see `rebase_is_complete`). The recovery uses this one
/// after it has read the row; the public function above cannot know the row.
pub(crate) fn settle_rebase_files_at(
    journal_dir: &Path,
    game: Uuid,
    store: Uuid,
    journal_seq: Option<u64>,
) -> io::Result<Settled> {
    let jnl = journal_path(journal_dir, game);
    let rebase = rebase_path(journal_dir, game);
    if fs::symlink_metadata(&rebase).is_err() {
        return Ok(Settled::Nothing);
    }
    if fs::symlink_metadata(&jnl).is_ok() {
        fs::remove_file(&rebase)?;
        sync_dir(journal_dir)?;
        return Ok(Settled::RemovedStaleRebase);
    }
    let bytes = fs::read(&rebase)?;
    if rebase_is_complete(&bytes, game, store, journal_seq) {
        move_no_replace(&rebase, &jnl)?;
        sync_dir(journal_dir)?;
        return Ok(Settled::RenamedIntoPlace);
    }
    let dir = damaged_dir(journal_dir);
    crate::datadir::create_dir_0700(&dir)?;
    let mut k = 1u32;
    let aside = loop {
        let dst = dir.join(format!("{}.{k}.rebase", game.hyphenated()));
        match fs::hard_link(&rebase, &dst) {
            Ok(()) => break dst,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // A crash between the link and the removal leaves the same file under both
                // names: finish that move instead of making a second copy.
                if same_file(&rebase, &dst)? {
                    break dst;
                }
                k += 1;
            }
            Err(e) => return Err(e),
        }
    };
    fs::remove_file(&rebase)?;
    sync_dir(journal_dir)?;
    sync_dir(&dir)?;
    Ok(Settled::MovedAsideIncomplete { damaged: aside })
}

/// What `finish_close_final` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseRepair {
    /// `done/<uuid>.jnl` was absent: the file was linked there and the old name removed.
    Moved,
    /// B-R03: `done/<uuid>.jnl` already was the same file (same device and inode): a crash or a
    /// retry after the link. The old name was removed, nothing else changed.
    FinishedEarlierAttempt,
}

/// The move of a finished journal to `journal/done/` (SDD 3.10.1, 3.11.8) as an idempotent
/// repair (B-R03, D-153): `hard_link(<uuid>.jnl, done/<uuid>.jnl)`, the failpoint
/// `journal.close_final.linked`, `remove_file(<uuid>.jnl)`, then both directories are synced.
/// When the link meets an existing `done/<uuid>.jnl` that is the same file, the earlier attempt
/// died between the two calls (the journal exists under two names): the old name is removed
/// and the call succeeds, so a retry never loops on `AlreadyExists`. A `done/` file that is a
/// **different** file is not a retry: it is reported as `AlreadyExists` and neither file is
/// touched (damage, for a person to look at).
pub(crate) fn finish_close_final(path: &Path) -> io::Result<CloseRepair> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let done = dir.join("done");
    crate::datadir::create_dir_0700(&done)?;
    let dst = done.join(name);
    let repair = match fs::hard_link(path, &dst) {
        Ok(()) => {
            fault::check("journal.close_final.linked", &dst)?;
            fs::remove_file(path)?;
            CloseRepair::Moved
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            if same_file(path, &dst)? {
                fs::remove_file(path)?;
                CloseRepair::FinishedEarlierAttempt
            } else {
                return Err(e);
            }
        }
        Err(e) => return Err(e),
    };
    sync_dir(dir)?;
    sync_dir(&done)?;
    Ok(repair)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::journal::format::{
        ClockPayload, ClockText, MovePayload, SeatKind, StartPayload, UuidText, encode_header,
        encode_record,
    };

    #[test]
    fn same_file_is_true_only_for_one_regular_file_under_two_names() {
        let dir = tempfile::TempDir::new().unwrap();
        let (a, b, c) = (
            dir.path().join("a"),
            dir.path().join("b"),
            dir.path().join("c"),
        );
        fs::write(&a, b"x").unwrap();
        fs::hard_link(&a, &b).unwrap();
        fs::write(&c, b"x").unwrap();
        assert!(same_file(&a, &b).unwrap());
        assert!(!same_file(&a, &c).unwrap());
        // A link is never followed and is never "the same file" (FIND-D4R-03).
        let link = dir.path().join("link");
        symlink(&a, &link).unwrap();
        assert!(!same_file(&a, &link).unwrap());
        assert!(!same_file(&link, &a).unwrap());
        let dangling = dir.path().join("dangling");
        symlink(dir.path().join("nowhere"), &dangling).unwrap();
        assert!(!same_file(&a, &dangling).unwrap());
        // A directory is not a regular file.
        assert!(!same_file(dir.path(), dir.path()).unwrap());
        assert!(same_file(&a, &dir.path().join("missing")).is_err());
    }

    /// CT-ST-089 "no-replace": a move onto an existing name fails and changes nothing (F01).
    #[test]
    fn move_no_replace_never_replaces() {
        let dir = tempfile::TempDir::new().unwrap();
        let (src, dst) = (dir.path().join("src"), dir.path().join("dst"));
        fs::write(&src, b"new").unwrap();
        fs::write(&dst, b"old").unwrap();
        let err = move_no_replace(&src, &dst).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&src).unwrap(), b"new");
        assert_eq!(fs::read(&dst).unwrap(), b"old");
        fs::remove_file(&dst).unwrap();
        move_no_replace(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read(&dst).unwrap(), b"new");
    }

    fn start(game: Uuid) -> Payload {
        Payload::Start(StartPayload {
            game: UuidText(game),
            white: UuidText(Uuid::from_u128(1)),
            black: UuidText(Uuid::from_u128(2)),
            white_kind: SeatKind::Human,
            black_kind: SeatKind::Human,
            matchup: "hh".into(),
            mode: "strict".into(),
            custom_start: false,
            start_fen: None,
            clock_spec: ClockText::parse("300+2"),
            rated: true,
            self_play: false,
        })
    }

    fn mv(ply: u32) -> Payload {
        Payload::Move(MovePayload {
            ply,
            uci: format!("u{ply}"),
            think_ms: 1,
            white_ms: Some(1),
            black_ms: Some(1),
            san: format!("s{ply}"),
            fen_after: format!("f{ply}"),
        })
    }

    fn stopped() -> Payload {
        Payload::Clock(ClockPayload {
            white_ms: Some(5),
            black_ms: Some(6),
            running_side: None,
        })
    }

    /// FIND-D4R-05: every byte offset of a rebase that carries a stopped `Clock` in the middle.
    /// With the row's `journal_seq` only the whole file is complete; the shape alone also
    /// accepts the cut right after the middle `Clock`, which is why the witness exists.
    #[test]
    fn only_the_whole_rebase_is_complete_at_every_byte_offset() {
        let (game, store) = (Uuid::from_u128(7), Uuid::from_u128(8));
        let payloads = [start(game), mv(0), stopped(), mv(1), stopped()];
        let mut bytes = encode_header(game, store).to_vec();
        let mut ends = Vec::new();
        for (i, payload) in payloads.into_iter().enumerate() {
            let record = JournalRecord {
                seq: u64::try_from(i).unwrap() + 1,
                wall_ms: 10,
                payload,
            };
            bytes.extend(encode_record(&record).unwrap());
            ends.push(bytes.len());
        }
        let last_seq = 5;
        let mut shape_only = Vec::new();
        for cut in 0..=bytes.len() {
            let part = &bytes[..cut];
            assert_eq!(
                rebase_is_complete(part, game, store, Some(last_seq)),
                cut == bytes.len(),
                "offset {cut}"
            );
            if rebase_is_complete(part, game, store, None) {
                shape_only.push(cut);
            }
        }
        assert_eq!(shape_only, vec![ends[2], ends[4]]);
        // A store that stopped short of the file does not reject it; one ahead of it does.
        assert!(rebase_is_complete(&bytes, game, store, Some(4)));
        assert!(!rebase_is_complete(&bytes, game, store, Some(6)));
        // Another game's or store's header is never complete.
        assert!(!rebase_is_complete(&bytes, Uuid::from_u128(9), store, None));
        assert!(!rebase_is_complete(&bytes, game, Uuid::from_u128(9), None));
    }
}
