//! Part B failure injection on the journal writer and the file steps (feature `test-support`):
//! CT-ST-001, CT-ST-002, UT-ST-087 (D-82), the create and rebase sync seams, CT-ST-089 (sync
//! order). The kill tests are in `journal_crash.rs`. Empty without the feature.
#![cfg(feature = "test-support")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chess_store::fault::{FaultGuard, FaultPlan};
use chess_store::journal::format::{HEADER_LEN, JournalRecord, Payload, encode_record};
use chess_store::journal::{
    JournalWriter, MoveJournal, RebaseInput, ReconcileInput, RunSource, build_rebased, fold,
    reconcile, scan_file, swap_in_rebased, write_rebase_file,
};
use chess_store::{DataDir, JournalError, WriteCause};
use common::jb;
use common::{block_on, gid};
use tempfile::TempDir;
use uuid::Uuid;

const STORE: u128 = 0x5701;

fn setup(game: u128) -> (TempDir, MoveJournal, JournalWriter) {
    let dir = TempDir::new().unwrap();
    let data = DataDir::new(dir.path());
    data.ensure().unwrap();
    let j = MoveJournal::new(data, Uuid::from_u128(STORE));
    let w = j.create(&gid(game)).unwrap();
    (dir, j, w)
}

fn path(dir: &Path, game: u128) -> PathBuf {
    dir.join("journal")
        .join(format!("{}.jnl", Uuid::from_u128(game).hyphenated()))
}

fn scan(dir: &Path, game: u128) -> chess_store::journal::Scan {
    scan_file(
        &path(dir, game),
        Uuid::from_u128(game),
        Uuid::from_u128(STORE),
    )
    .unwrap()
}

fn install(dir: &Path, spec: &str) -> FaultGuard {
    FaultPlan::parse(spec).unwrap().install(dir).unwrap()
}

fn append(w: &JournalWriter, p: Payload) -> Result<u64, JournalError> {
    block_on(w.append(1, p))
}

fn seqs(dir: &Path, game: u128) -> Vec<u64> {
    scan(dir, game).records.iter().map(|r| r.seq).collect()
}

/// CT-ST-001: the failpoints of an append are reached in order once each per record; the
/// record is not acknowledged before the full sync (a delay at `journal.before_fsync` is inside
/// the call), and a failing sync fails the append.
#[test]
fn ct_st_001_the_append_passes_every_failpoint_and_returns_after_the_sync() {
    let (dir, _j, w) = setup(1);
    let guard = install(dir.path(), "journal.before_fsync=delay:200");
    let started = Instant::now();
    append(&w, jb::start(1)).unwrap();
    assert!(
        started.elapsed() >= Duration::from_millis(200),
        "returned only after the sync step"
    );
    for name in [
        "journal.before_write",
        "journal.partial_write",
        "journal.after_write",
        "journal.before_fsync",
        "journal.fsync_error",
        "journal.after_fsync",
    ] {
        assert_eq!(guard.hits(name), 1, "{name}");
    }
    // One plan per directory (FIND-D1R-02): the first is finished before the next is installed.
    drop(guard);
    // The sync call is in the path: ENOTSUP and ENOTTY from `F_FULLFSYNC` are `WriteFailed`
    // and nothing the reader accepts is written (D-71, last sentence).
    for errno in [45, 25, 5] {
        let guard = install(dir.path(), &format!("journal.fsync_error=errno:{errno}"));
        let err = append(&w, jb::mv(0)).unwrap_err();
        assert!(
            matches!(
                err,
                JournalError::WriteFailed {
                    cause: WriteCause::Io(_)
                }
            ),
            "{err:?}"
        );
        assert_eq!(guard.fired("journal.fsync_error"), 1);
        drop(guard);
        assert_eq!(seqs(dir.path(), 1), vec![1], "errno {errno}");
        assert!(!w.is_poisoned());
    }
}

/// CT-ST-002: disk full and read-only injected into the append.
#[test]
fn ct_st_002_disk_full_and_read_only_are_write_failed_and_nothing_is_accepted() {
    let (dir, _j, w) = setup(2);
    append(&w, jb::start(2)).unwrap();
    let length = fs::metadata(path(dir.path(), 2)).unwrap().len();
    for (point, errno, cause) in [
        ("journal.before_write", 28, WriteCause::DiskFull),
        ("journal.partial_write", 28, WriteCause::DiskFull),
        ("journal.after_write", 30, WriteCause::ReadOnly),
        ("journal.fsync_error", 30, WriteCause::ReadOnly),
    ] {
        let guard = install(dir.path(), &format!("{point}=errno:{errno}"));
        let err = append(&w, jb::mv(0)).unwrap_err();
        assert_eq!(err, JournalError::WriteFailed { cause }, "{point}");
        drop(guard);
        assert_eq!(
            fs::metadata(path(dir.path(), 2)).unwrap().len(),
            length,
            "{point}: as before"
        );
        assert_eq!(seqs(dir.path(), 2), vec![1]);
    }
    assert_eq!(
        append(&w, jb::mv(0)).unwrap(),
        2,
        "the writer works again; seq 2 was never used"
    );
}

/// UT-ST-087 (1): `journal.partial_write` (the write stops half-way), then a successful append:
/// the file holds exactly the acknowledged records, no damage, no torn tail.
#[test]
fn ut_st_087_a_partial_write_is_cleaned_up_and_the_next_append_reuses_the_seq() {
    let (dir, _j, w) = setup(3);
    append(&w, jb::start(3)).unwrap();
    append(&w, jb::mv(0)).unwrap();
    let acked = w.acked_len();
    let guard = install(dir.path(), "journal.partial_write=errno:5@0!");
    let err = append(&w, jb::mv(1)).unwrap_err();
    assert!(
        matches!(
            err,
            JournalError::WriteFailed {
                cause: WriteCause::Io(_)
            }
        ),
        "{err:?}"
    );
    assert_eq!(guard.fired("journal.partial_write"), 1);
    assert_eq!(
        fs::metadata(path(dir.path(), 3)).unwrap().len(),
        acked,
        "truncated to acked_len"
    );
    assert!(!w.is_poisoned());
    assert_eq!(w.next_seq(), 3);
    assert_eq!(
        append(&w, jb::mv(1)).unwrap(),
        3,
        "the failed record's seq is reused"
    );
    let s = scan(dir.path(), 3);
    assert_eq!(
        s.records.iter().map(|r| r.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(s.damaged.is_none() && !s.torn_tail_dropped);
    // The record written after the failure sits at the old acked_len: the explicit seek after
    // `set_len` (without it the descriptor would still be past the cut and leave a hole).
    let expected_len = acked
        + u64::try_from(
            encode_record(&JournalRecord {
                seq: 3,
                wall_ms: 1,
                payload: jb::mv(1),
            })
            .unwrap()
            .len(),
        )
        .unwrap();
    assert_eq!(
        fs::metadata(path(dir.path(), 3)).unwrap().len(),
        expected_len
    );
}

/// UT-ST-087 (2): `journal.fsync_error` with a successful cleanup.
#[test]
fn ut_st_087_a_failed_sync_with_a_successful_cleanup_leaves_the_file_as_before() {
    let (dir, _j, w) = setup(4);
    append(&w, jb::start(4)).unwrap();
    let acked = w.acked_len();
    let _guard = install(dir.path(), "journal.fsync_error=errno:5@0!");
    assert!(matches!(
        append(&w, jb::mv(0)),
        Err(JournalError::WriteFailed { .. })
    ));
    assert_eq!(fs::metadata(path(dir.path(), 4)).unwrap().len(), acked);
    assert_eq!(w.next_seq(), 2);
    assert_eq!(append(&w, jb::mv(0)).unwrap(), 2);
    assert_eq!(seqs(dir.path(), 4), vec![1, 2]);
}

/// UT-ST-087 (3): `journal.truncate_error` after a failed write: the writer is poisoned, every
/// later append is `WriteFailed{Poisoned}`, the file is not touched again, and the next owner's
/// scan applies the torn-tail rule.
#[test]
fn ut_st_087_a_failed_cleanup_poisons_the_writer() {
    let (dir, _j, w) = setup(5);
    append(&w, jb::start(5)).unwrap();
    let acked = w.acked_len();
    let guard = install(
        dir.path(),
        "journal.partial_write=errno:5@0!,journal.truncate_error=errno:5",
    );
    let err = append(&w, jb::mv(0)).unwrap_err();
    assert!(
        matches!(
            err,
            JournalError::WriteFailed {
                cause: WriteCause::Io(_)
            }
        ),
        "{err:?}"
    );
    assert_eq!(guard.fired("journal.truncate_error"), 1);
    assert!(w.is_poisoned());
    let torn = fs::read(path(dir.path(), 5)).unwrap();
    assert!(
        u64::try_from(torn.len()).unwrap() > acked,
        "half a record is still there"
    );
    for _ in 0..3 {
        assert_eq!(
            append(&w, jb::mv(0)).unwrap_err(),
            JournalError::WriteFailed {
                cause: WriteCause::Poisoned
            }
        );
    }
    assert_eq!(
        fs::read(path(dir.path(), 5)).unwrap(),
        torn,
        "not touched again"
    );
    assert_eq!(
        guard.hits("journal.before_write"),
        1,
        "later appends do not reach the failpoints"
    );
    // The next owner's scan drops the torn tail silently and keeps the acknowledged record.
    let s = scan(dir.path(), 5);
    assert_eq!(s.records.len(), 1);
    assert!(s.torn_tail_dropped && s.damaged.is_none());
    assert_eq!(u64::try_from(s.intact_end).unwrap(), acked);
}

/// A failure at `before_write` and at `after_fsync` (the record may be durable but the caller
/// was told it failed): the first leaves the file as before; the second also truncates, because
/// the ack was not given.
#[test]
fn errors_at_the_outer_failpoints_are_cleaned_up_too() {
    let (dir, _j, w) = setup(6);
    append(&w, jb::start(6)).unwrap();
    for point in [
        "journal.before_write",
        "journal.after_write",
        "journal.before_fsync",
        "journal.after_fsync",
    ] {
        let guard = install(dir.path(), &format!("{point}=errno:5@0!"));
        assert!(append(&w, jb::mv(0)).is_err(), "{point}");
        assert_eq!(guard.fired(point), 1);
        drop(guard);
        assert_eq!(seqs(dir.path(), 6), vec![1], "{point}");
        assert!(!w.is_poisoned(), "{point}");
    }
    assert_eq!(append(&w, jb::mv(0)).unwrap(), 2);
}

/// A header that cannot be synced leaves no header-only file behind (STATE-ST-01).
#[test]
fn a_failed_create_leaves_no_file() {
    let dir = TempDir::new().unwrap();
    let data = DataDir::new(dir.path());
    data.ensure().unwrap();
    let j = MoveJournal::new(data, Uuid::from_u128(STORE));
    for point in ["sync.journal_create_file", "sync.journal_create_dir"] {
        let guard = install(dir.path(), &format!("{point}=errno:5"));
        assert!(
            matches!(j.create(&gid(9)), Err(JournalError::WriteFailed { .. })),
            "{point}"
        );
        assert_eq!(guard.fired(point), 1);
        drop(guard);
        assert!(!path(dir.path(), 9).exists(), "{point}");
    }
    j.create(&gid(9)).unwrap();
}

#[test]
fn a_failed_directory_sync_in_close_final_is_an_error() {
    let (dir, _j, w) = setup(10);
    append(&w, jb::start(10)).unwrap();
    let guard = install(dir.path(), "sync.journal_move_dir=errno:5");
    assert!(matches!(
        w.close_final(),
        Err(JournalError::WriteFailed { .. })
    ));
    assert_eq!(
        guard.fired("sync.journal_move_dir"),
        1,
        "the first directory sync fails the close"
    );
    drop(guard);
    // The second directory sync (journal/done/) fails the close as well.
    let (dir, _j, w) = setup(12);
    append(&w, jb::start(12)).unwrap();
    let guard = install(dir.path(), "sync.journal_move_dir=errno:5@1!");
    assert!(matches!(
        w.close_final(),
        Err(JournalError::WriteFailed { .. })
    ));
    assert_eq!(guard.hits("sync.journal_move_dir"), 2);
}

// ------------------------------------------------------------------- CT-ST-089 sync order

fn rebase_fixture(dir: &Path, game: u128) -> chess_store::journal::Rebased {
    let j = MoveJournal::new(DataDir::new(dir), Uuid::from_u128(STORE));
    let w = j.create(&gid(game)).unwrap();
    for (i, p) in [jb::start(game), jb::mv(0), jb::mv(1), jb::mv(2)]
        .into_iter()
        .enumerate()
    {
        append(&w, p).unwrap();
        let _ = i;
    }
    drop(w);
    let p = path(dir, game);
    let mut bytes = fs::read(&p).unwrap();
    let s = chess_store::journal::scan_bytes(&bytes, Uuid::from_u128(game), Uuid::from_u128(STORE))
        .unwrap();
    let first = HEADER_LEN + encode_record(&s.records[0]).unwrap().len();
    bytes[first + 14] ^= 0xFF; // damage record 2
    fs::write(&p, &bytes).unwrap();
    let s = chess_store::journal::scan_bytes(&bytes, Uuid::from_u128(game), Uuid::from_u128(STORE))
        .unwrap();
    assert!(s.damaged.is_some());
    let folded = fold(&s.records);
    let plan = reconcile(&ReconcileInput {
        journal: &folded,
        stored: &[],
        scan_damaged: true,
        db_seq: 0,
        last_intact_seq: 1,
        stored_clocks: None,
    });
    build_rebased(&RebaseInput {
        game: Uuid::from_u128(game),
        store: Uuid::from_u128(STORE),
        intact_bytes: &bytes[HEADER_LEN..s.intact_end],
        intact: &s.records,
        source: RunSource::Journal,
        journal_last_ply: 0,
        stored: &[],
        clocks: plan.clocks.unwrap_or(None),
        start: None,
    })
    .unwrap()
}

/// CT-ST-089 (RR-A-5): both directories are synced **before** the new file is linked into
/// place. Failing the second directory sync of the swap (the sync of `journal/damaged/`) leaves
/// the state of window (c): no `<uuid>.jnl`, a complete `.rebase`, the old file in `damaged/`;
/// `new_linked` is never reached. A full swap does four directory syncs (one for the rebase
/// file, two after the first move, one after the second) and one file sync.
#[test]
fn ct_st_089_both_directories_are_synced_before_the_second_move() {
    let dir = TempDir::new().unwrap();
    DataDir::new(dir.path()).ensure().unwrap();
    let rebased = rebase_fixture(dir.path(), 60);
    let jdir = dir.path().join("journal");
    let game = Uuid::from_u128(60);
    // Sync number 0 is the rebase file's directory; number 1 is the sync of journal/ after the
    // first move; number 2 is journal/damaged/.
    let guard = install(
        dir.path(),
        "sync.journal_move_dir=errno:5@2!,journal.rebase.new_linked=errno:5",
    );
    let file = write_rebase_file(&jdir, game, &rebased).unwrap();
    assert!(swap_in_rebased(&jdir, game, file).is_err());
    assert_eq!(
        guard.hits("journal.rebase.new_linked"),
        0,
        "the second move never started"
    );
    assert!(!path(dir.path(), 60).exists(), "window (c): no journal");
    assert!(
        jdir.join(format!("{game}.jnl.rebase", game = game.hyphenated()))
            .exists()
    );
    assert_eq!(fs::read_dir(jdir.join("damaged")).unwrap().count(), 1);
    drop(guard);
    // The startup path completes it by moving the .rebase into place.
    assert_eq!(
        chess_store::journal::settle_rebase_files(&jdir, game, Uuid::from_u128(STORE)).unwrap(),
        chess_store::journal::Settled::RenamedIntoPlace
    );
    assert!(scan(dir.path(), 60).damaged.is_none());
}

#[test]
fn a_full_swap_makes_four_directory_syncs_and_one_file_sync() {
    let dir = TempDir::new().unwrap();
    DataDir::new(dir.path()).ensure().unwrap();
    let rebased = rebase_fixture(dir.path(), 61);
    let jdir = dir.path().join("journal");
    let game = Uuid::from_u128(61);
    let guard = install(dir.path(), "journal.rebase.built=delay:0");
    let file = write_rebase_file(&jdir, game, &rebased).unwrap();
    swap_in_rebased(&jdir, game, file).unwrap();
    assert_eq!(guard.hits("sync.journal_move_dir"), 4);
    assert_eq!(guard.hits("sync.journal_rebase_file"), 1);
    for name in [
        "journal.rebase.built",
        "journal.rebase.old_linked",
        "journal.rebase.old_aside",
        "journal.rebase.new_linked",
        "journal.rebase.new_in_place",
    ] {
        assert_eq!(guard.hits(name), 1, "{name}");
    }
}

/// A failed rebase file write removes the `.rebase` (nothing half-built is left for the swap).
#[test]
fn a_failed_rebase_file_write_removes_the_rebase() {
    let dir = TempDir::new().unwrap();
    DataDir::new(dir.path()).ensure().unwrap();
    let rebased = rebase_fixture(dir.path(), 62);
    let jdir = dir.path().join("journal");
    let game = Uuid::from_u128(62);
    for point in ["sync.journal_rebase_file", "journal.rebase.built"] {
        let guard = install(dir.path(), &format!("{point}=errno:5"));
        assert!(write_rebase_file(&jdir, game, &rebased).is_err(), "{point}");
        drop(guard);
        assert!(
            !jdir
                .join(format!("{}.jnl.rebase", game.hyphenated()))
                .exists(),
            "{point}"
        );
    }
}
