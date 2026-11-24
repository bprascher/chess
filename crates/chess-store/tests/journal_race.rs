//! B-R04 (review of part B, FIND-B-R04; D4 status): `MoveJournal::create` against a concurrent
//! `probe` of the same name. `create` makes the file and then locks it; a `probe` that takes the
//! lock in between used to make `create` return `Locked` for a file it had just made (and remove
//! it). The fix: a `WouldBlock` on the lock of a file this call just created is retried for a
//! short grace period, and the file is not removed when the holder stays. This is the binary the
//! task calls "the race binary"; it runs many rounds, with a prober thread and, in the second
//! test, with two real processes.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chess_store::journal::LeaseState;
use chess_store::journal::MoveJournal;
use chess_store::{DataDir, JournalError};
use common::child;
use common::gid;
use tempfile::TempDir;
use uuid::Uuid;

const STORE: u128 = 0x5701;
const ENV_DIR: &str = "CHESS_STORE_RACE_DIR";
const ENV_FROM: &str = "CHESS_STORE_RACE_FROM";
const ENV_ROUNDS: &str = "CHESS_STORE_RACE_ROUNDS";

fn rounds() -> u128 {
    std::env::var("CHESS_STORE_RACE_ROUNDS_TOTAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000)
}

fn setup() -> (TempDir, MoveJournal) {
    let dir = TempDir::new().unwrap();
    let data = DataDir::new(dir.path());
    data.ensure().unwrap();
    (dir, MoveJournal::new(data, Uuid::from_u128(STORE)))
}

/// 3 000 creates, each racing a prober thread that locks and unlocks the same name in a loop:
/// none may be refused, none may lose its file, and the writer holds the lock afterwards.
#[test]
fn b_r04_create_is_never_refused_by_a_concurrent_probe() {
    let (_dir, journal) = setup();
    let mut spurious = 0u32;
    for n in 1..=rounds() {
        let game = gid(n);
        let stop = Arc::new(AtomicBool::new(false));
        let prober = {
            let journal = journal.clone();
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let _ = journal.probe(&game);
                }
            })
        };
        let created = journal.create(&game);
        stop.store(true, Ordering::Relaxed);
        prober.join().unwrap();
        match created {
            Ok(writer) => {
                assert_eq!(journal.probe(&game), LeaseState::Held);
                assert!(writer.path().exists());
            }
            Err(JournalError::Locked { .. }) => spurious += 1,
            Err(other) => panic!("round {n}: {other:?}"),
        }
    }
    assert_eq!(spurious, 0, "spurious Locked from create");
}

#[test]
fn race_child_entry() {
    let Ok(dir) = std::env::var(ENV_DIR) else {
        return;
    };
    let from: u128 = std::env::var(ENV_FROM).unwrap().parse().unwrap();
    let count: u128 = std::env::var(ENV_ROUNDS).unwrap().parse().unwrap();
    let data = DataDir::new(&dir);
    let journal = MoveJournal::new(data, Uuid::from_u128(STORE));
    // The child only probes the names the parent creates, as fast as it can, until its
    // standard input closes.
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            child::wait_for_stdin_eof();
            stop.store(true, Ordering::Relaxed);
        })
    };
    println!("CHILD READY");
    while !stop.load(Ordering::Relaxed) {
        for n in from..from + count {
            let _ = journal.probe(&gid(n));
        }
    }
    watcher.join().unwrap();
}

/// The same race across two processes: a child process probes the names while this one creates.
#[test]
fn b_r04_create_is_never_refused_by_a_probing_process() {
    let (dir, journal) = setup();
    let count = 400u128;
    let mut probing = child::spawn(
        "race_child_entry",
        &[
            (ENV_DIR, dir.path().to_str().unwrap()),
            (ENV_FROM, "10000"),
            (ENV_ROUNDS, &count.to_string()),
        ],
    );
    probing.expect("READY");
    let mut spurious = 0u32;
    for n in 10_000..10_000 + count {
        match journal.create(&gid(n)) {
            Ok(_writer) => {}
            Err(JournalError::Locked { .. }) => spurious += 1,
            Err(other) => panic!("{other:?}"),
        }
    }
    probing.finish();
    assert_eq!(spurious, 0, "spurious Locked from create across processes");
}

/// A holder that stays (D4 verification, mutant 60): a thread takes the lock of every name the
/// moment it appears and keeps it until `create` has answered. When `create` loses that race it
/// reports `Locked`, and **the file stays** (it belongs to the holder; `create` used to remove
/// it). The race is won by the holder only some of the time, so the test runs rounds until it
/// has seen the outcome a number of times, and fails when it never does.
#[test]
fn b_r04_a_create_that_loses_to_a_holder_reports_locked_and_leaves_the_file() {
    let (_dir, journal) = setup();
    let wanted = 5u32;
    let mut lost = 0u32;
    for n in 50_000..50_000 + 4_000u128 {
        let game = gid(n);
        let path = journal.path(&game);
        let done = Arc::new(AtomicBool::new(false));
        let holder = {
            let (path, done) = (path.clone(), Arc::clone(&done));
            std::thread::spawn(move || {
                let mut held = None;
                while !done.load(Ordering::Relaxed) {
                    if held.is_none()
                        && let Ok(file) = std::fs::File::open(&path)
                        && file.try_lock().is_ok()
                    {
                        held = Some(file);
                    }
                }
                held.is_some()
            })
        };
        let created = journal.create(&game);
        done.store(true, Ordering::Relaxed);
        let holder_won = holder.join().unwrap();
        match created {
            Ok(_writer) => {}
            Err(JournalError::Locked { .. }) => {
                assert!(holder_won, "Locked without a holder");
                assert!(path.exists(), "round {n}: create removed a holder's file");
                lost += 1;
            }
            Err(other) => panic!("round {n}: {other:?}"),
        }
        if lost >= wanted {
            break;
        }
    }
    assert!(lost >= wanted, "the holder never won the race: {lost}");
}
