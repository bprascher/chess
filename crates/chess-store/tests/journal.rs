//! Part B, the journal through the public API on real files: create, append, the lock, close,
//! the writer rules that need no fault injection (D-82 dropped future, size limit), the file
//! moves (D-110), the rebase builder and the rebase file steps (UT-ST-088, 090, CT-ST-089 file
//! half), CT-ST-083 (untimed clocks). Fault-injection and kill tests are in `journal_faults.rs`
//! and `journal_crash.rs` (feature `test-support`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::fs;
use std::future::Future;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::task::{Context, Waker};

use chess_store::journal::format::{
    HEADER_LEN, JournalRecord, MovePayload, Payload, StartPayload, decode_header, encode_header,
    encode_record,
};
use chess_store::journal::{
    ClockPair, JournalWriter, LeaseState, MoveJournal, RebaseError, RebaseInput, ReconcileInput,
    RunSource, Settled, StoredMove, apply_plan, build_rebased, fold, move_aside_damaged, reconcile,
    reconciled_clocks, scan_bytes, scan_file, settle_rebase_files, swap_in_rebased,
    write_rebase_file,
};
use chess_store::{DataDir, JournalError, WriteCause};
use common::jb;
use common::{block_on, gid, open_in};
use tempfile::TempDir;
use uuid::Uuid;

const STORE: u128 = 0x5701;

fn journal_in(dir: &Path) -> MoveJournal {
    let data = DataDir::new(dir);
    data.ensure().unwrap();
    MoveJournal::new(data, Uuid::from_u128(STORE))
}

fn jnl(dir: &Path, game: u128) -> PathBuf {
    dir.join("journal")
        .join(format!("{}.jnl", Uuid::from_u128(game).hyphenated()))
}

fn scan(dir: &Path, game: u128) -> chess_store::journal::Scan {
    scan_file(
        &jnl(dir, game),
        Uuid::from_u128(game),
        Uuid::from_u128(STORE),
    )
    .unwrap()
}

fn append(w: &JournalWriter, wall: i64, p: Payload) -> Result<u64, JournalError> {
    block_on(w.append(wall, p))
}

// ------------------------------------------------------------------ create, append, lock

#[test]
fn create_writes_the_exact_header_with_mode_0600_and_syncs_a_new_file() {
    let dir = TempDir::new().unwrap();
    let j = journal_in(dir.path());
    let w = j.create(&gid(7)).unwrap();
    let path = jnl(dir.path(), 7);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        bytes,
        encode_header(Uuid::from_u128(7), Uuid::from_u128(STORE)).to_vec()
    );
    assert_eq!(bytes.len(), HEADER_LEN);
    let header = decode_header(&bytes).unwrap();
    assert_eq!(
        (header.version, header.game, header.store),
        (1, Uuid::from_u128(7), Uuid::from_u128(STORE))
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!((w.next_seq(), w.acked_len()), (1, 42));
    assert_eq!(w.path(), path);
}

/// D3 ruling 2026-11-24: the writer refuses a record that the format refuses (an end time or a
/// `Start` time at or below zero, a `matchup` text that is not the code of the kinds): an error,
/// no byte written, the sequence number not used up, and the next good record is accepted.
#[test]
fn the_writer_refuses_a_record_the_format_refuses_and_writes_nothing() {
    use chess_store::journal::format::{AbortPayload, SeatKind};
    let dir = TempDir::new().unwrap();
    let j = journal_in(dir.path());
    let w = j.create(&gid(1)).unwrap();
    assert_eq!(append(&w, 1_000, jb::start(1)).unwrap(), 1);
    let before = fs::read(jnl(dir.path(), 1)).unwrap();
    let refused = JournalError::WriteFailed {
        cause: WriteCause::Io(std::io::ErrorKind::InvalidInput),
    };
    let abort = |ended_at| {
        Payload::Abort(AbortPayload {
            ended_at,
            content_hash: "h".into(),
        })
    };
    assert_eq!(append(&w, 1_001, abort(0)).unwrap_err(), refused);
    assert_eq!(append(&w, 1_001, abort(-9)).unwrap_err(), refused);
    assert_eq!(
        append(&w, 0, jb::start(2)).unwrap_err(),
        refused,
        "Start.wall_ms 0"
    );
    let Payload::Start(mut wrong) = jb::start(3) else {
        panic!()
    };
    wrong.white_kind = SeatKind::Computer;
    assert_eq!(
        append(&w, 1_002, Payload::Start(wrong)).unwrap_err(),
        refused,
        "matchup hh with a computer seat"
    );
    assert_eq!(
        fs::read(jnl(dir.path(), 1)).unwrap(),
        before,
        "no byte written"
    );
    assert_eq!(w.next_seq(), 2, "no sequence number used up");
    assert_eq!(append(&w, 1_003, abort(5)).unwrap(), 2);
}

#[test]
fn a_second_create_of_a_held_journal_is_locked_and_of_a_free_one_is_an_error_that_replaces_nothing()
{
    let dir = TempDir::new().unwrap();
    let j = journal_in(dir.path());
    let w = j.create(&gid(1)).unwrap();
    assert_eq!(
        j.create(&gid(1)).unwrap_err(),
        JournalError::Locked { owner_kind: None }
    );
    assert_eq!(j.probe(&gid(1)), LeaseState::Held);
    drop(w);
    assert_eq!(
        j.probe(&gid(1)),
        LeaseState::Free,
        "the lock goes with the descriptor"
    );
    let before = fs::read(jnl(dir.path(), 1)).unwrap();
    match j.create(&gid(1)) {
        Err(JournalError::WriteFailed {
            cause: WriteCause::Io(std::io::ErrorKind::AlreadyExists),
        }) => {}
        other => panic!("{other:?}"),
    }
    assert_eq!(
        fs::read(jnl(dir.path(), 1)).unwrap(),
        before,
        "never replaced"
    );
    assert_eq!(j.probe(&gid(404)), LeaseState::Free, "no file is free");
}

#[test]
fn append_assigns_sequence_numbers_and_the_file_is_exactly_header_plus_records() {
    let dir = TempDir::new().unwrap();
    let j = journal_in(dir.path());
    let w = j.create(&gid(3)).unwrap();
    let payloads = [jb::start(3), jb::mv(0), jb::mv(1), jb::clock(5, 6)];
    let mut expected = encode_header(Uuid::from_u128(3), Uuid::from_u128(STORE)).to_vec();
    for (i, p) in payloads.iter().enumerate() {
        let seq = append(&w, 1000 + i64::try_from(i).unwrap(), p.clone()).unwrap();
        assert_eq!(seq, u64::try_from(i).unwrap() + 1);
        expected.extend(
            encode_record(&JournalRecord {
                seq,
                wall_ms: 1000 + i64::try_from(i).unwrap(),
                payload: p.clone(),
            })
            .unwrap(),
        );
        assert_eq!(w.acked_len(), u64::try_from(expected.len()).unwrap());
    }
    assert_eq!(fs::read(jnl(dir.path(), 3)).unwrap(), expected);
    assert_eq!(w.next_seq(), 5);
    let s = scan(dir.path(), 3);
    assert_eq!(s.records.len(), 4);
    assert!(s.damaged.is_none() && !s.torn_tail_dropped);
    assert_eq!(s.records[2].wall_ms, 1002);
    // The blocking variant is the same code path.
    assert_eq!(w.append_blocking(9, jb::mv(2)).unwrap(), 5);
}

/// CT-ST-083 (writer, scan and reconcile halves): an untimed game's null clocks survive.
#[test]
fn ct_st_083_untimed_null_clocks_survive_append_scan_and_reconcile() {
    let dir = TempDir::new().unwrap();
    let j = journal_in(dir.path());
    let w = j.create(&gid(4)).unwrap();
    append(&w, 1, jb::start(4)).unwrap();
    append(&w, 2, jb::untimed_move(0)).unwrap();
    append(
        &w,
        3,
        Payload::Clock(chess_store::journal::format::ClockPayload {
            white_ms: None,
            black_ms: None,
            running_side: None,
        }),
    )
    .unwrap();
    let text = String::from_utf8_lossy(&fs::read(jnl(dir.path(), 4)).unwrap()).into_owned();
    assert!(text.contains("\"white_ms\":null"));
    let s = scan(dir.path(), 4);
    let f = fold(&s.records);
    assert_eq!(f.moves[&1].clocks, None);
    assert_eq!(f.clocks, Some(None));
    let plan = reconcile(&ReconcileInput {
        journal: &f,
        stored: &[],
        scan_damaged: false,
        db_seq: 0,
        last_intact_seq: 3,
        stored_clocks: None,
    });
    assert_eq!(plan.rows_to_write.len(), 1);
    assert_eq!(
        plan.clocks, None,
        "null equals the stored null: nothing to set"
    );
}

#[test]
fn a_body_over_the_limit_is_refused_leaves_the_file_alone_and_the_writer_usable() {
    let dir = TempDir::new().unwrap();
    let w = journal_in(dir.path()).create(&gid(5)).unwrap();
    append(&w, 1, jb::start(5)).unwrap();
    let before = fs::read(jnl(dir.path(), 5)).unwrap();
    let big = Payload::Move(MovePayload {
        ply: 0,
        uci: "x".into(),
        think_ms: 0,
        white_ms: None,
        black_ms: None,
        san: "s".repeat(70_000),
        fen_after: "f".into(),
    });
    assert_eq!(
        append(&w, 2, big).unwrap_err(),
        JournalError::WriteFailed {
            cause: WriteCause::TooLarge
        }
    );
    assert_eq!(fs::read(jnl(dir.path(), 5)).unwrap(), before);
    assert!(!w.is_poisoned());
    assert_eq!(
        append(&w, 3, jb::mv(0)).unwrap(),
        2,
        "the seq was not consumed"
    );
}

/// A half-null clock pair is refused by the writer (SF-14), file untouched.
#[test]
fn a_record_the_reader_would_refuse_is_never_written() {
    let dir = TempDir::new().unwrap();
    let w = journal_in(dir.path()).create(&gid(6)).unwrap();
    let before = fs::read(jnl(dir.path(), 6)).unwrap();
    let half = Payload::Clock(chess_store::journal::format::ClockPayload {
        white_ms: Some(1),
        black_ms: None,
        running_side: None,
    });
    assert!(matches!(
        append(&w, 1, half),
        Err(JournalError::WriteFailed { .. })
    ));
    assert_eq!(fs::read(jnl(dir.path(), 6)).unwrap(), before);
    assert!(!w.is_poisoned());
}

/// D-82 (3): a caller that stops waiting does not know whether the record is durable, so the
/// writer is poisoned; the task still ran to completion; every later append is refused and
/// touches nothing.
#[test]
fn a_dropped_append_future_poisons_the_writer_but_the_work_completes() {
    let dir = TempDir::new().unwrap();
    let w = journal_in(dir.path()).create(&gid(8)).unwrap();
    append(&w, 1, jb::start(8)).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let _enter = rt.enter();
    let mut fut = Box::pin(w.append(2, jb::mv(0)));
    let mut cx = Context::from_waker(Waker::noop());
    let _ = fut.as_mut().poll(&mut cx); // submits the blocking task
    drop(fut);
    assert!(w.is_poisoned());
    // Wait for the task to finish writing, then look at the file.
    let mut tries = 0;
    while w.next_seq() == 2 && tries < 200 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        tries += 1;
    }
    let on_disk = scan(dir.path(), 8);
    assert!(!on_disk.records.is_empty());
    let size = fs::metadata(jnl(dir.path(), 8)).unwrap().len();
    let later = rt.block_on(w.append(3, jb::mv(1)));
    assert_eq!(
        later.unwrap_err(),
        JournalError::WriteFailed {
            cause: WriteCause::Poisoned
        }
    );
    assert_eq!(
        fs::metadata(jnl(dir.path(), 8)).unwrap().len(),
        size,
        "nothing written after poison"
    );
    assert_eq!(
        w.append_blocking(4, jb::mv(1)).unwrap_err(),
        JournalError::WriteFailed {
            cause: WriteCause::Poisoned
        }
    );
}

#[test]
fn an_append_future_that_completes_does_not_poison() {
    let dir = TempDir::new().unwrap();
    let w = journal_in(dir.path()).create(&gid(9)).unwrap();
    append(&w, 1, jb::start(9)).unwrap();
    assert!(!w.is_poisoned());
}

// -------------------------------------------------------------------------- close_final

#[test]
fn close_final_moves_the_file_to_done_without_replacing_and_releases_the_lock() {
    let dir = TempDir::new().unwrap();
    let j = journal_in(dir.path());
    let w = j.create(&gid(10)).unwrap();
    append(&w, 1, jb::start(10)).unwrap();
    let bytes = fs::read(jnl(dir.path(), 10)).unwrap();
    w.close_final().unwrap();
    let done = dir
        .path()
        .join("journal/done")
        .join(format!("{}.jnl", Uuid::from_u128(10).hyphenated()));
    assert_eq!(fs::read(&done).unwrap(), bytes);
    assert!(!jnl(dir.path(), 10).exists());
    assert_eq!(j.probe(&gid(10)), LeaseState::Free);
    assert_eq!(fs::metadata(&done).unwrap().nlink(), 1, "one name");
    // A file already in done/ is never replaced: the close fails and the journal stays.
    let w = j.create(&gid(11)).unwrap();
    let blocker = dir
        .path()
        .join("journal/done")
        .join(format!("{}.jnl", Uuid::from_u128(11).hyphenated()));
    fs::write(&blocker, b"previous").unwrap();
    assert!(matches!(
        w.close_final(),
        Err(JournalError::WriteFailed { .. })
    ));
    assert_eq!(fs::read(&blocker).unwrap(), b"previous");
    assert!(jnl(dir.path(), 11).exists());
}

// --------------------------------------------------------------------- the move helper

#[test]
fn the_no_replace_move_never_overwrites_and_changes_nothing_on_failure() {
    let dir = TempDir::new().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    fs::write(&a, b"A").unwrap();
    fs::write(&b, b"B").unwrap();
    // `hard_link` onto an existing name is the no-replace rule (D-110).
    assert_eq!(
        fs::hard_link(&a, &b).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        (fs::read(&a).unwrap(), fs::read(&b).unwrap()),
        (b"A".to_vec(), b"B".to_vec())
    );
}

fn damaged_setup(dir: &Path, game: u128) -> Vec<u8> {
    let j = journal_in(dir);
    let w = j.create(&gid(game)).unwrap();
    append(&w, 1, jb::start(game)).unwrap();
    for ply in 0..4 {
        append(&w, 10 + i64::from(ply), jb::mv(ply)).unwrap();
    }
    drop(w);
    // Damage the middle: flip a byte in record 3 (seq 3 = journal ply 1... any record body).
    let path = jnl(dir, game);
    let mut bytes = fs::read(&path).unwrap();
    let s = scan_bytes(&bytes, Uuid::from_u128(game), Uuid::from_u128(STORE)).unwrap();
    assert_eq!(s.records.len(), 5);
    let first = HEADER_LEN
        + s.records[..2]
            .iter()
            .map(|r| encode_record(r).unwrap().len())
            .sum::<usize>();
    bytes[first + 14] ^= 0xFF;
    fs::write(&path, &bytes).unwrap();
    bytes
}

#[test]
fn move_aside_picks_the_first_unused_k_and_never_replaces_a_damaged_file() {
    let dir = TempDir::new().unwrap();
    let original = damaged_setup(dir.path(), 20);
    let jdir = dir.path().join("journal");
    let damaged = jdir.join("damaged");
    fs::create_dir(&damaged).unwrap();
    let name = |k: u32| damaged.join(format!("{}.{k}.jnl", Uuid::from_u128(20).hyphenated()));
    fs::write(name(1), b"an earlier damaged journal").unwrap();
    let moved = move_aside_damaged(&jdir, Uuid::from_u128(20)).unwrap();
    assert_eq!((moved.k, moved.finished_earlier_attempt), (2, false));
    assert_eq!(
        fs::read(name(1)).unwrap(),
        b"an earlier damaged journal",
        "never replaced"
    );
    assert_eq!(fs::read(name(2)).unwrap(), original);
    assert!(!jnl(dir.path(), 20).exists());
    assert_eq!(fs::metadata(name(2)).unwrap().nlink(), 1);
}

/// Window (e): the damaged file exists under two names (a crash after 3a, before 3b). The mover
/// finishes with the removal alone and makes no second `damaged/` file; a `damaged/` file of
/// another inode only moves `k` on.
#[test]
fn window_e_a_file_with_two_names_is_finished_by_the_removal_alone() {
    let dir = TempDir::new().unwrap();
    let original = damaged_setup(dir.path(), 21);
    let jdir = dir.path().join("journal");
    let damaged = jdir.join("damaged");
    fs::create_dir(&damaged).unwrap();
    let name = |k: u32| damaged.join(format!("{}.{k}.jnl", Uuid::from_u128(21).hyphenated()));
    fs::write(name(1), b"another inode").unwrap();
    fs::hard_link(jnl(dir.path(), 21), name(2)).unwrap(); // 3a done, 3b not
    assert_eq!(fs::metadata(name(2)).unwrap().nlink(), 2);
    let moved = move_aside_damaged(&jdir, Uuid::from_u128(21)).unwrap();
    assert_eq!((moved.k, moved.finished_earlier_attempt), (2, true));
    assert!(!jnl(dir.path(), 21).exists());
    let in_damaged: Vec<_> = fs::read_dir(&damaged)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(in_damaged.len(), 2, "no third file: {in_damaged:?}");
    assert_eq!(fs::read(name(2)).unwrap(), original);
    assert_eq!(fs::metadata(name(2)).unwrap().nlink(), 1);
}

/// Windows (a), (b), (c), (f) at file level.
#[test]
fn settle_rebase_files_handles_the_crash_windows() {
    let dir = TempDir::new().unwrap();
    let jdir = dir.path().join("journal");
    fs::create_dir_all(&jdir).unwrap();
    let game = Uuid::from_u128(30);
    let jnl = jdir.join(format!("{}.jnl", game.hyphenated()));
    let rebase = jdir.join(format!("{}.jnl.rebase", game.hyphenated()));
    assert_eq!(
        settle_rebase_files(&jdir, game, Uuid::from_u128(STORE)).unwrap(),
        Settled::Nothing
    );
    // (a)/(b): a .rebase beside a (damaged) journal is stale and removed; the journal stays.
    fs::write(&jnl, b"damaged").unwrap();
    fs::write(&rebase, b"stale").unwrap();
    assert_eq!(
        settle_rebase_files(&jdir, game, Uuid::from_u128(STORE)).unwrap(),
        Settled::RemovedStaleRebase
    );
    assert!(!rebase.exists());
    assert_eq!(fs::read(&jnl).unwrap(), b"damaged");
    // (f): the same file under two names: the .rebase name goes, the journal stays whole.
    fs::write(&jnl, b"rebased").unwrap();
    fs::hard_link(&jnl, &rebase).unwrap();
    assert_eq!(
        settle_rebase_files(&jdir, game, Uuid::from_u128(STORE)).unwrap(),
        Settled::RemovedStaleRebase
    );
    assert_eq!(fs::read(&jnl).unwrap(), b"rebased");
    assert_eq!(fs::metadata(&jnl).unwrap().nlink(), 1);
    // (c): no journal, a complete .rebase: moved into place, never replacing. "Complete" is
    // checked (B-R05): a real rebase file, built the way the builder builds one.
    let complete = {
        let mut bytes = encode_header(game, Uuid::from_u128(STORE)).to_vec();
        for record in [
            JournalRecord {
                seq: 1,
                wall_ms: 5,
                payload: jb::start(30),
            },
            JournalRecord {
                seq: 2,
                wall_ms: 5,
                payload: jb::clock(1, 2),
            },
        ] {
            bytes.extend(encode_record(&record).unwrap());
        }
        bytes
    };
    fs::remove_file(&jnl).unwrap();
    fs::write(&rebase, &complete).unwrap();
    assert_eq!(
        settle_rebase_files(&jdir, game, Uuid::from_u128(STORE)).unwrap(),
        Settled::RenamedIntoPlace
    );
    assert_eq!(fs::read(&jnl).unwrap(), complete);
    assert!(!rebase.exists());
}

/// B-R05: an incomplete `.rebase` with no journal is not installed. Its bytes are kept under
/// `damaged/<uuid>.<k>.rebase`; no `<uuid>.jnl` appears.
#[test]
fn b_r05_an_incomplete_rebase_is_moved_aside_and_never_installed() {
    let dir = TempDir::new().unwrap();
    let jdir = dir.path().join("journal");
    fs::create_dir_all(&jdir).unwrap();
    let game = Uuid::from_u128(31);
    let jnl = jdir.join(format!("{}.jnl", game.hyphenated()));
    let rebase = jdir.join(format!("{}.jnl.rebase", game.hyphenated()));
    let store = Uuid::from_u128(STORE);
    let mut good = encode_header(game, store).to_vec();
    for record in [
        JournalRecord {
            seq: 1,
            wall_ms: 5,
            payload: jb::start(31),
        },
        JournalRecord {
            seq: 2,
            wall_ms: 6,
            payload: jb::mv(0),
        },
        JournalRecord {
            seq: 3,
            wall_ms: 6,
            payload: jb::clock(1, 2),
        },
    ] {
        good.extend(encode_record(&record).unwrap());
    }
    let cut_at_record = {
        // Cut exactly before the closing Clock: every record is whole, but the file does not
        // end the way a rebase ends.
        let last = encode_record(&JournalRecord {
            seq: 3,
            wall_ms: 6,
            payload: jb::clock(1, 2),
        })
        .unwrap()
        .len();
        good[..good.len() - last].to_vec()
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("garbage", b"complete".to_vec()),
        ("header only", good[..HEADER_LEN].to_vec()),
        ("torn", good[..good.len() - 3].to_vec()),
        ("whole records, no closing clock", cut_at_record),
        ("another store", {
            let mut other = good.clone();
            other[..HEADER_LEN].copy_from_slice(&encode_header(game, Uuid::from_u128(1)));
            other
        }),
    ];
    for (n, (name, bytes)) in cases.iter().enumerate() {
        fs::write(&rebase, bytes).unwrap();
        let settled = settle_rebase_files(&jdir, game, store).unwrap();
        let k = n + 1;
        let aside = jdir
            .join("damaged")
            .join(format!("{}.{k}.rebase", game.hyphenated()));
        assert_eq!(
            settled,
            Settled::MovedAsideIncomplete {
                damaged: aside.clone()
            },
            "{name}"
        );
        assert!(!jnl.exists(), "{name}: nothing was installed");
        assert!(!rebase.exists(), "{name}");
        assert_eq!(&fs::read(&aside).unwrap(), bytes, "{name}: bytes kept");
    }
    // The complete one is installed.
    fs::write(&rebase, &good).unwrap();
    assert_eq!(
        settle_rebase_files(&jdir, game, store).unwrap(),
        Settled::RenamedIntoPlace
    );
    assert_eq!(fs::read(&jnl).unwrap(), good);
}

// ------------------------------------------------------------------ rebase builder

fn stored(ply: u32, played_at: Option<i64>) -> StoredMove {
    let j = ply - 1;
    StoredMove {
        ply,
        uci: format!("u{j}"),
        san: format!("s{j}"),
        fen_after: format!("fen{j}"),
        think_ms: Some(10 + u64::from(j)),
        clocks: Some((300_000 - u64::from(j), 300_000 + u64::from(j))),
        played_at,
    }
}

struct Built {
    old: Vec<u8>,
    scan: chess_store::journal::Scan,
    rebased: chess_store::journal::Rebased,
}

/// A journal of Start + moves for journal plies 0 and 1 intact (seq 1..=3), the rest damaged;
/// the store holds plies 1..=5 (the journal plies 0..=4).
fn build_with_store_longer(dir: &Path, game: u128) -> Built {
    let old = damaged_setup(dir, game);
    let scan = scan_bytes(&old, Uuid::from_u128(game), Uuid::from_u128(STORE)).unwrap();
    assert!(scan.damaged.is_some());
    assert_eq!(scan.records.len(), 2, "Start and one Move are intact");
    let folded = fold(&scan.records);
    let rows: Vec<StoredMove> = (1..=5)
        .map(|p| {
            stored(
                p,
                match p {
                    1 => Some(10),
                    4 => None,
                    _ => Some(500 + i64::from(p)),
                },
            )
        })
        .collect();
    let plan = reconcile(&ReconcileInput {
        journal: &folded,
        stored: &rows,
        scan_damaged: true,
        db_seq: 6,
        last_intact_seq: scan.last_seq(),
        stored_clocks: Some((7, 8)),
    });
    assert_eq!(plan.source, RunSource::Store);
    let clocks = reconciled_clocks(&plan, Some((7, 8)));
    let rebased = build_rebased(&RebaseInput {
        game: Uuid::from_u128(game),
        store: Uuid::from_u128(STORE),
        intact_bytes: &old[HEADER_LEN..scan.intact_end],
        intact: &scan.records,
        source: plan.source,
        journal_last_ply: u32::try_from(folded.moves.len()).unwrap(),
        stored: &rows,
        clocks,
        start: None,
    })
    .unwrap();
    Built { old, scan, rebased }
}

/// UT-ST-088: byte-for-byte copy of the intact run, synthesized `Move` records only for the
/// plies the store holds beyond it, `seq` continuing, `wall_ms` from `played_at` or the
/// previous record when NULL, a closing `Clock`, deterministic, and a reconcile after the
/// rebase gives an empty plan.
#[test]
fn ut_st_088_the_rebase_builder_copies_synthesizes_and_is_deterministic() {
    let dir = TempDir::new().unwrap();
    let b = build_with_store_longer(dir.path(), 40);
    let bytes = &b.rebased.bytes;
    assert_eq!(
        &bytes[..HEADER_LEN],
        &encode_header(Uuid::from_u128(40), Uuid::from_u128(STORE))[..]
    );
    let copied = &b.old[HEADER_LEN..b.scan.intact_end];
    assert_eq!(
        &bytes[HEADER_LEN..HEADER_LEN + copied.len()],
        copied,
        "byte for byte"
    );
    let s = scan_bytes(bytes, Uuid::from_u128(40), Uuid::from_u128(STORE)).unwrap();
    assert!(s.damaged.is_none() && !s.torn_tail_dropped);
    let seqs: Vec<u64> = s.records.iter().map(|r| r.seq).collect();
    assert_eq!(
        seqs,
        vec![1, 2, 3, 4, 5, 6, 7],
        "continuous: Start, Move, 4 synthesized, Clock"
    );
    assert_eq!(s.records[0].wall_ms, 1);
    assert_eq!(s.records[1].wall_ms, 10);
    // Synthesized plies 2 to 5 are journal plies 1 to 4 with the row's values.
    for (rec, ply) in s.records[2..6].iter().zip(2u32..) {
        let Payload::Move(m) = &rec.payload else {
            panic!("{rec:?}")
        };
        assert_eq!(m.ply, ply - 1, "journal ply = MOVE.ply - 1 (D-92)");
        assert_eq!(
            (m.uci.as_str(), m.san.as_str()),
            (
                format!("u{}", ply - 1).as_str(),
                format!("s{}", ply - 1).as_str()
            )
        );
        assert_eq!(m.think_ms, 10 + u64::from(ply - 1));
    }
    assert_eq!(s.records[2].wall_ms, 502, "played_at of the row");
    assert_eq!(s.records[3].wall_ms, 503);
    assert_eq!(
        s.records[4].wall_ms, 503,
        "NULL played_at: the previous record's wall_ms"
    );
    assert_eq!(s.records[5].wall_ms, 505);
    let Payload::Clock(c) = &s.records[6].payload else {
        panic!()
    };
    assert_eq!(
        (c.white_ms, c.black_ms, c.running_side),
        (Some(7), Some(8), None),
        "the stored clocks (D-66 case)"
    );
    assert_eq!(s.records[6].wall_ms, 505);
    assert_eq!(b.rebased.last_seq, 7);
    // Deterministic.
    let again = build_with_store_longer(TempDir::new().unwrap().path(), 40);
    assert_eq!(again.rebased, b.rebased);
    // After the rebase: db_seq = last_intact_seq = N and reconcile gives an empty plan.
    let folded = fold(&s.records);
    let rows: Vec<StoredMove> = (1..=5).map(|p| stored(p, None)).collect();
    let plan = reconcile(&ReconcileInput {
        journal: &folded,
        stored: &rows,
        scan_damaged: false,
        db_seq: 7,
        last_intact_seq: 7,
        stored_clocks: Some((7, 8)),
    });
    assert!(plan.is_empty(), "{plan:?}");
    let _ = apply_plan(&rows, None, &plan);
}

/// When the journal wins at some ply no record is synthesized (only the closing `Clock`).
#[test]
fn the_builder_synthesizes_nothing_when_the_journal_wins() {
    let dir = TempDir::new().unwrap();
    let old = damaged_setup(dir.path(), 41);
    let scan = scan_bytes(&old, Uuid::from_u128(41), Uuid::from_u128(STORE)).unwrap();
    let rebased = build_rebased(&RebaseInput {
        game: Uuid::from_u128(41),
        store: Uuid::from_u128(STORE),
        intact_bytes: &old[HEADER_LEN..scan.intact_end],
        intact: &scan.records,
        source: RunSource::Journal,
        journal_last_ply: 1,
        stored: &[stored(1, None), stored(2, None), stored(3, None)],
        clocks: None,
        start: None,
    })
    .unwrap();
    let s = scan_bytes(&rebased.bytes, Uuid::from_u128(41), Uuid::from_u128(STORE)).unwrap();
    assert_eq!(s.records.len(), 3, "Start, Move, Clock");
    let Payload::Clock(c) = &s.records[2].payload else {
        panic!()
    };
    assert_eq!(
        (c.white_ms, c.black_ms),
        (None, None),
        "reconciled clocks, untimed"
    );
}

/// FIND-RW1-12 (M36): `RunSource::Both` means the journal and the store agree, so the builder
/// synthesizes nothing even when stored rows lie beyond the journal's last ply; only `Store` does.
#[test]
fn the_builder_synthesizes_only_for_a_store_source_not_for_both() {
    let dir = TempDir::new().unwrap();
    let old = damaged_setup(dir.path(), 43);
    let scan = scan_bytes(&old, Uuid::from_u128(43), Uuid::from_u128(STORE)).unwrap();
    let rows = [stored(1, None), stored(2, None), stored(3, None)];
    let build = |source| {
        build_rebased(&RebaseInput {
            game: Uuid::from_u128(43),
            store: Uuid::from_u128(STORE),
            intact_bytes: &old[HEADER_LEN..scan.intact_end],
            intact: &scan.records,
            source,
            journal_last_ply: 1,
            stored: &rows,
            clocks: None,
            start: None,
        })
        .unwrap()
    };
    let count = |rebased: &chess_store::journal::Rebased| {
        scan_bytes(&rebased.bytes, Uuid::from_u128(43), Uuid::from_u128(STORE))
            .unwrap()
            .records
            .len()
    };
    assert_eq!(count(&build(RunSource::Both)), 3, "Start, Move, Clock");
    assert_eq!(count(&build(RunSource::Journal)), 3);
    assert_eq!(
        count(&build(RunSource::Store)),
        5,
        "two synthesized Moves (plies 2, 3), plus Clock"
    );
}

/// UT-ST-090: the intact run holds no `Start`: the file starts with the synthesized one.
#[test]
fn ut_st_090_an_empty_intact_run_starts_with_the_synthesized_start() {
    let rows: Vec<StoredMove> = (1..=3)
        .map(|p| stored(p, Some(900 + i64::from(p))))
        .collect();
    let input = |start: Option<(StartPayload, i64)>| RebaseInput {
        game: Uuid::from_u128(42),
        store: Uuid::from_u128(STORE),
        intact_bytes: &[],
        intact: &[],
        source: RunSource::Store,
        journal_last_ply: 0,
        stored: &rows,
        clocks: Some((1, 2)),
        start,
    };
    assert_eq!(
        build_rebased(&input(None)).unwrap_err(),
        RebaseError::MissingStart
    );
    let Payload::Start(start) = jb::start(42) else {
        panic!()
    };
    let rebased = build_rebased(&input(Some((start.clone(), 777)))).unwrap();
    assert_eq!(
        rebased,
        build_rebased(&input(Some((start, 777)))).unwrap(),
        "deterministic"
    );
    let s = scan_bytes(&rebased.bytes, Uuid::from_u128(42), Uuid::from_u128(STORE)).unwrap();
    let seqs: Vec<u64> = s.records.iter().map(|r| r.seq).collect();
    assert_eq!(
        seqs,
        vec![1, 2, 3, 4, 5],
        "Start at 1, moves from 2, then the Clock"
    );
    assert_eq!(s.records[0].wall_ms, 777, "GAME.started_at");
    assert!(matches!(s.records[0].payload, Payload::Start(_)));
    assert_eq!(s.records[1].wall_ms, 901);
}

// ------------------------------------------------------------------- rebase file steps

/// The whole file half of a rebase with no kill: the old file lands in `damaged/` byte for
/// byte, the new one is in place and scans clean, no `.rebase` remains, no file has two names,
/// the lock is held by the returned writer, and it continues with `seq = N + 1`.
#[test]
fn the_rebase_file_steps_swap_the_new_journal_in_and_keep_the_old_one() {
    let dir = TempDir::new().unwrap();
    let b = build_with_store_longer(dir.path(), 50);
    let jdir = dir.path().join("journal");
    let game = Uuid::from_u128(50);
    let file = write_rebase_file(&jdir, game, &b.rebased).unwrap();
    let rebase = jdir.join(format!("{}.jnl.rebase", game.hyphenated()));
    assert_eq!(fs::read(&rebase).unwrap(), b.rebased.bytes);
    assert_eq!(
        fs::metadata(&rebase).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let writer = swap_in_rebased(&jdir, game, file).unwrap();
    assert!(!rebase.exists());
    assert_eq!(fs::read(jnl(dir.path(), 50)).unwrap(), b.rebased.bytes);
    assert_eq!(fs::metadata(jnl(dir.path(), 50)).unwrap().nlink(), 1);
    let damaged: Vec<_> = fs::read_dir(jdir.join("damaged"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(damaged.len(), 1);
    assert_eq!(fs::read(&damaged[0]).unwrap(), b.old);
    assert_eq!(
        fs::metadata(jdir.join("damaged"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert!(
        damaged[0]
            .to_string_lossy()
            .ends_with(&format!("{}.1.jnl", game.hyphenated()))
    );
    // The flock on the rebase file's inode carries over to the new name.
    assert_eq!(
        MoveJournal::new(DataDir::new(dir.path()), Uuid::from_u128(STORE)).probe(&gid(50)),
        LeaseState::Held
    );
    assert_eq!(
        (writer.next_seq(), writer.acked_len()),
        (8, u64::try_from(b.rebased.bytes.len()).unwrap())
    );
    assert_eq!(
        append(&writer, 9, jb::mv(5)).unwrap(),
        8,
        "seq is continuous"
    );
    assert!(scan(dir.path(), 50).damaged.is_none());
    drop(writer);
    assert_eq!(
        MoveJournal::new(DataDir::new(dir.path()), Uuid::from_u128(STORE)).probe(&gid(50)),
        LeaseState::Free
    );
}

/// The rebase file never replaces: a `.rebase` that exists is an error and is left alone.
#[test]
fn write_rebase_file_refuses_an_existing_rebase_and_leaves_it() {
    let dir = TempDir::new().unwrap();
    let b = build_with_store_longer(dir.path(), 51);
    let jdir = dir.path().join("journal");
    let rebase = jdir.join(format!("{}.jnl.rebase", Uuid::from_u128(51).hyphenated()));
    fs::write(&rebase, b"someone else's").unwrap();
    assert!(write_rebase_file(&jdir, Uuid::from_u128(51), &b.rebased).is_err());
    assert_eq!(fs::read(&rebase).unwrap(), b"someone else's");
}

#[test]
fn a_store_without_store_id_gives_no_journal() {
    let dir = TempDir::new().unwrap();
    let store = open_in(dir.path()).store;
    assert!(store.journal().is_ok());
    fs::remove_file(dir.path().join("store.id")).unwrap();
    let reopened = open_in(dir.path()).store;
    assert!(matches!(
        reopened.journal(),
        Err(chess_store::StoreError::Corrupt)
    ));
    let _: ClockPair = None;
}

// ------------------------------------------------- rebase rework 2026-11-22 (CH-11 B, B-11 B)

mod rework {
    use super::*;
    use chess_store::journal::format::{
        AbortPayload, ClockPayload, EndReason, GameEndPayload, Side, VoidReason, VoidedPayload,
        crc32,
    };

    fn terminal(kind: &str) -> Payload {
        match kind {
            "GameEnd" => Payload::GameEnd(GameEndPayload {
                result: "1-0".to_owned(),
                reason: EndReason::Resignation,
                ended_at: 9,
                eco: None,
                opening_name: None,
                analysis_used: false,
                content_hash: "h".to_owned(),
                white_time_used_ms: 1,
                black_time_used_ms: 2,
                total_active_ms: 3,
            }),
            "Abort" => Payload::Abort(AbortPayload {
                ended_at: 9,
                content_hash: "h".to_owned(),
            }),
            _ => Payload::Voided(VoidedPayload {
                reason: VoidReason::Unreadable,
                reset_player: None,
                content_hash: "h".to_owned(),
                ended_at: 9,
            }),
        }
    }

    fn file_of(game: u128, records: &[Vec<u8>]) -> Vec<u8> {
        let mut out = encode_header(Uuid::from_u128(game), Uuid::from_u128(STORE)).to_vec();
        for r in records {
            out.extend(r);
        }
        out
    }

    fn rec(seq: u64, payload: Payload) -> Vec<u8> {
        encode_record(&JournalRecord {
            seq,
            wall_ms: 100 + i64::try_from(seq).unwrap(),
            payload,
        })
        .unwrap()
    }

    fn rebase_of(
        game: u128,
        old: &[u8],
        source: RunSource,
        stored_rows: &[StoredMove],
        last_ply: u32,
    ) -> chess_store::journal::Rebased {
        let scan = scan_bytes(old, Uuid::from_u128(game), Uuid::from_u128(STORE)).unwrap();
        build_rebased(&RebaseInput {
            game: Uuid::from_u128(game),
            store: Uuid::from_u128(STORE),
            intact_bytes: &old[HEADER_LEN..scan.intact_end],
            intact: &scan.records,
            source,
            journal_last_ply: last_ply,
            stored: stored_rows,
            clocks: Some((5, 6)),
            start: None,
        })
        .unwrap()
    }

    /// CH-11 B (Brian, 2026-11-22): the builder omits the closing `Clock` when the last record of
    /// the rebased file is terminal (`GameEnd`, `Abort`, `Voided`). The file is then the old
    /// intact run byte for byte.
    #[test]
    fn ch11_no_closing_clock_after_a_terminal_last_record() {
        for kind in ["GameEnd", "Abort", "Voided"] {
            let old = file_of(
                60,
                &[
                    rec(1, jb::start(60)),
                    rec(2, jb::mv(0)),
                    rec(3, terminal(kind)),
                ],
            );
            let rebased = rebase_of(60, &old, RunSource::Journal, &[], 1);
            assert_eq!(rebased.bytes, old, "{kind}: unchanged, no Clock");
            assert_eq!(rebased.last_seq, 3, "{kind}");
            let s =
                scan_bytes(&rebased.bytes, Uuid::from_u128(60), Uuid::from_u128(STORE)).unwrap();
            assert!(
                s.records
                    .iter()
                    .all(|r| !matches!(r.payload, Payload::Clock(_))),
                "{kind}"
            );
        }
    }

    /// CH-11 B boundary: a non-terminal last record still gets the closing `Clock`, a terminal
    /// record that is not last (`Move` after it, synthesized from the store) does too, and a
    /// `Clock` that was already the last record is kept and not doubled.
    #[test]
    fn ch11_the_closing_clock_stays_when_the_last_record_is_not_terminal() {
        let old = file_of(61, &[rec(1, jb::start(61)), rec(2, jb::mv(0))]);
        let rebased = rebase_of(61, &old, RunSource::Journal, &[], 1);
        assert_eq!(rebased.last_seq, 3);
        assert_eq!(
            rebased.bytes.len(),
            old.len() + rec(3, jb::clock(5, 6)).len()
        );
        let s = scan_bytes(&rebased.bytes, Uuid::from_u128(61), Uuid::from_u128(STORE)).unwrap();
        assert!(matches!(s.records[2].payload, Payload::Clock(_)));

        // Terminal first, then a stored row beyond the journal: the last record is a Move, so
        // the Clock closes the file.
        let old = file_of(
            62,
            &[
                rec(1, jb::start(62)),
                rec(2, jb::mv(0)),
                rec(3, terminal("GameEnd")),
            ],
        );
        let rebased = rebase_of(
            62,
            &old,
            RunSource::Store,
            &[stored(1, None), stored(2, None), stored(3, None)],
            1,
        );
        let s = scan_bytes(&rebased.bytes, Uuid::from_u128(62), Uuid::from_u128(STORE)).unwrap();
        let kinds: Vec<_> = s.records.iter().map(|r| r.kind()).collect();
        assert_eq!(
            kinds.last().copied(),
            Some(chess_store::journal::format::Kind::Clock)
        );
        assert_eq!(
            s.records.len(),
            6,
            "Start, Move, GameEnd, 2 synthesized, Clock"
        );
    }

    /// Spec 3.3 (e), observed: rebasing an already rebased file is idempotent when it ends with a
    /// terminal record (no `Clock` is added), and **not** idempotent otherwise: each rebase adds
    /// one more closing `Clock` (FIND-ST-RW-04, reported to the architect; not changed here, the
    /// ruling is only about the terminal case).
    #[test]
    fn rebase_of_a_rebased_file_is_idempotent_after_a_terminal_record_and_after_its_own_clock() {
        let terminal = file_of(
            64,
            &[
                rec(1, jb::start(64)),
                rec(2, jb::mv(0)),
                rec(3, terminal("GameEnd")),
            ],
        );
        let once = rebase_of(64, &terminal, RunSource::Journal, &[], 1);
        let twice = rebase_of(64, &once.bytes, RunSource::Journal, &[], 1);
        assert_eq!(once.bytes, twice.bytes, "terminal: idempotent");

        // FIND-RW1-03, FIND-ST-RW-04, spec 3.3 (e): a file that ends with a Move gets one closing
        // Clock, and every later rebase of the result is byte-equal to the first.
        let open = file_of(65, &[rec(1, jb::start(65)), rec(2, jb::mv(0))]);
        let once = rebase_of(65, &open, RunSource::Journal, &[], 1);
        assert_eq!(once.last_seq, 3);
        let twice = rebase_of(65, &once.bytes, RunSource::Journal, &[], 1);
        let thrice = rebase_of(65, &twice.bytes, RunSource::Journal, &[], 1);
        assert_eq!(once, twice, "second rebase equals the first, byte for byte");
        assert_eq!(twice, thrice);
        assert_eq!(twice.last_seq, 3, "no second Clock");

        // A Clock that is last but is not the closing one (other values, or a running side) is
        // not it: the reconciled clocks are written once, and then the file is stable.
        for last in [
            jb::clock(7, 8),
            Payload::Clock(ClockPayload {
                white_ms: Some(5),
                black_ms: Some(6),
                running_side: Some(Side::White),
            }),
        ] {
            let file = file_of(66, &[rec(1, jb::start(66)), rec(2, last)]);
            let once = rebase_of(66, &file, RunSource::Journal, &[], 1);
            assert_eq!(once.last_seq, 3, "a closing Clock is appended");
            let twice = rebase_of(66, &once.bytes, RunSource::Journal, &[], 1);
            assert_eq!(once, twice);
        }
        // The closing Clock already last: kept as it is, nothing appended.
        let file = file_of(67, &[rec(1, jb::start(67)), rec(2, jb::clock(5, 6))]);
        let once = rebase_of(67, &file, RunSource::Journal, &[], 1);
        assert_eq!(once.bytes, file);
    }

    /// FIND-RW1-10: `intact_bytes` and `intact` must describe one intact run; every way they can
    /// disagree is refused, and the agreeing case builds.
    #[test]
    fn rebase_input_with_bytes_and_records_that_disagree_is_refused() {
        let old = file_of(
            68,
            &[rec(1, jb::start(68)), rec(2, jb::mv(0)), rec(3, jb::mv(1))],
        );
        let scan = scan_bytes(&old, Uuid::from_u128(68), Uuid::from_u128(STORE)).unwrap();
        let bytes = &old[HEADER_LEN..scan.intact_end];
        let build = |intact_bytes: &[u8], intact: &[JournalRecord]| {
            build_rebased(&RebaseInput {
                game: Uuid::from_u128(68),
                store: Uuid::from_u128(STORE),
                intact_bytes,
                intact,
                source: RunSource::Journal,
                journal_last_ply: 2,
                stored: &[],
                clocks: Some((5, 6)),
                start: None,
            })
        };
        assert!(build(bytes, &scan.records).is_ok(), "the agreeing case");
        let refused = Err(RebaseError::InconsistentIntact);
        // Empty bytes, one decoded record (the reviewer's case: last_seq 2, file rescans as 0).
        assert_eq!(build(&[], &scan.records[..1]), refused);
        // Bytes, no records (it would have taken the `start` path and dropped the bytes).
        assert_eq!(build(bytes, &[]), refused);
        // Fewer records than bytes, and more records than bytes.
        assert_eq!(build(bytes, &scan.records[..2]), refused);
        let mut longer = scan.records.clone();
        longer.push(scan.records[2].clone());
        assert_eq!(build(bytes, &longer), refused);
        // The same count with a different record.
        let mut altered = scan.records.clone();
        altered[2].wall_ms += 1;
        assert_eq!(build(bytes, &altered), refused);
        // A torn tail inside the bytes, and a flipped byte inside them.
        assert_eq!(build(&bytes[..bytes.len() - 1], &scan.records), refused);
        assert_eq!(
            build(&bytes[..bytes.len() - 1], &scan.records[..2]),
            refused
        );
        let mut flipped = bytes.to_vec();
        flipped[bytes.len() / 2] ^= 0xff;
        assert_eq!(build(&flipped, &scan.records), refused);
    }

    /// FIND-B-11 B (the test the architect noted missing): a CRC-valid record that carries
    /// fields this build does not know survives a rebase byte for byte, still scans as a good
    /// record, and the file steps put the same bytes on disk. The builder re-encodes nothing of
    /// the intact run.
    #[test]
    fn unknown_fields_survive_a_rebase_verbatim() {
        let json = br#"{"to_ply":1,"future_field":{"a":[1,2,3]},"z":null}"#;
        let mut body = 2u64.to_le_bytes().to_vec();
        body.push(3);
        body.extend(7i64.to_le_bytes());
        body.extend(json);
        let len = u32::try_from(body.len()).unwrap();
        let mut covered = len.to_le_bytes().to_vec();
        covered.extend(&body);
        let mut frame = vec![0xA5, 0x5A];
        frame.extend(len.to_le_bytes());
        frame.extend(crc32(&covered).to_le_bytes());
        frame.extend(body);

        let old = file_of(
            63,
            &[rec(1, jb::start(63)), frame.clone(), rec(3, jb::mv(0))],
        );
        let scan = scan_bytes(&old, Uuid::from_u128(63), Uuid::from_u128(STORE)).unwrap();
        assert_eq!(scan.records.len(), 3, "tolerant decode reads it");
        let rebased = rebase_of(63, &old, RunSource::Journal, &[], 1);
        assert!(
            rebased
                .bytes
                .windows(frame.len())
                .any(|w| w == frame.as_slice()),
            "the frame with the unknown fields is in the new file unchanged"
        );
        assert_eq!(
            &rebased.bytes[HEADER_LEN..HEADER_LEN + (old.len() - HEADER_LEN)],
            &old[HEADER_LEN..],
            "the whole intact run is byte for byte"
        );
        let again =
            scan_bytes(&rebased.bytes, Uuid::from_u128(63), Uuid::from_u128(STORE)).unwrap();
        assert_eq!(&again.records[..3], &scan.records[..]);
        assert!(again.damaged.is_none() && !again.torn_tail_dropped);

        // And on disk, through the file steps.
        let dir = TempDir::new().unwrap();
        let jdir = dir.path().join("journal");
        fs::create_dir_all(&jdir).unwrap();
        let game = Uuid::from_u128(63);
        let file = write_rebase_file(&jdir, game, &rebased).unwrap();
        let _writer = swap_in_rebased(&jdir, game, file).unwrap();
        assert_eq!(fs::read(jnl(dir.path(), 63)).unwrap(), rebased.bytes);
    }
}
