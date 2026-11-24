//! First launch under concurrency and after failures (review FIND-001, FIND-002, FIND-008).
//! Threads share one process; the child-process test runs this same test binary again
//! (`CHESS_STORE_CHILD_DIR` set) so that two or three real processes open one directory.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Barrier};

use chess_store::{Store, StoreError};
use common::{block_on, human, options, try_open};
use tempfile::TempDir;

const CHILD_ENV: &str = "CHESS_STORE_CHILD_DIR";
const CHILD_ID_ENV: &str = "CHESS_STORE_CHILD_ID";

fn db_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with("chess.db"))
        .collect();
    names.sort();
    names
}

/// Opens, registers one player named by `id`, returns the store (kept alive by the caller) or
/// the typed error.
fn open_and_write(dir: &Path, id: u128) -> Result<chess_store::OpenedStore, StoreError> {
    let opened = block_on(Store::open(options(dir)))?;
    block_on(
        opened
            .store
            .register_player(human(id, &format!("Racer {id}"))),
    )?;
    Ok(opened)
}

/// FIND-001: N concurrent first launches, 200 rounds. Every opener succeeds or fails with a
/// typed error; `chess.db` exists exactly once; every write that was acknowledged is there
/// after all openers are gone; exactly one opener reports a first launch.
#[test]
fn ct_st_004_concurrent_first_launches_never_delete_the_winners_database_200_rounds() {
    const ROUNDS: u128 = 200;
    const OPENERS: u128 = 4;
    for round in 0..ROUNDS {
        let dir = TempDir::new().unwrap();
        let barrier = Arc::new(Barrier::new(OPENERS as usize));
        let handles: Vec<_> = (0..OPENERS)
            .map(|n| {
                let path = dir.path().to_path_buf();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    open_and_write(&path, round * 100 + n + 1)
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let mut acknowledged = 0usize;
        let mut firsts = 0usize;
        for r in &results {
            match r {
                Ok(o) => {
                    acknowledged += 1;
                    firsts += usize::from(o.report.first_launch);
                }
                // A loser of the first-launch race repeats its upgrade step, so nobody is turned
                // away: any error here is a defect.
                Err(e) => panic!("round {round}: {e:?}"),
            }
        }
        assert_eq!(acknowledged, OPENERS as usize, "round {round}");
        assert_eq!(firsts, 1, "round {round}: exactly one first launch");
        assert!(
            db_files(dir.path()).contains(&"chess.db".to_owned()),
            "round {round}: the database is gone: {:?}",
            db_files(dir.path())
        );
        drop(results);
        let again = try_open(dir.path()).unwrap();
        assert!(!again.report.first_launch);
        let players = block_on(again.store.list_players(None, true)).unwrap();
        assert_eq!(players.len(), acknowledged, "round {round}: lost writes");
        assert!(dir.path().join("store.id").exists());
    }
}

/// The child half of the process test: does nothing unless the parent set the environment.
#[test]
fn child_opener() {
    let Ok(dir) = std::env::var(CHILD_ENV) else {
        return;
    };
    let id: u128 = std::env::var(CHILD_ID_ENV).unwrap().parse().unwrap();
    match open_and_write(Path::new(&dir), id) {
        Ok(o) => {
            // Exit status carries the answer: 0 ok, 10 ok and first launch.
            let code = if o.report.first_launch { 10 } else { 0 };
            drop(o);
            std::process::exit(code);
        }
        Err(StoreError::Busy) => std::process::exit(20),
        Err(_) => std::process::exit(30),
    }
}

/// FIND-001 with real processes: three children, 60 rounds (a spawn of the test binary costs
/// far more than a thread).
#[test]
fn ct_st_004_three_processes_opening_a_fresh_directory_lose_nothing_60_rounds() {
    let exe = std::env::current_exe().unwrap();
    for round in 0..60u128 {
        let dir = TempDir::new().unwrap();
        let children: Vec<_> = (0..3u128)
            .map(|n| {
                Command::new(&exe)
                    .args(["--exact", "child_opener", "--nocapture", "--test-threads=1"])
                    .env(CHILD_ENV, dir.path())
                    .env(CHILD_ID_ENV, (round * 10 + n + 1).to_string())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .unwrap()
            })
            .collect();
        let codes: Vec<i32> = children
            .into_iter()
            .map(|mut c| c.wait().unwrap().code().unwrap())
            .collect();
        assert!(
            codes.iter().all(|c| [0, 10].contains(c)),
            "round {round}: {codes:?}"
        );
        let ok = codes.iter().filter(|c| **c != 20).count();
        assert!(ok >= 1, "round {round}: {codes:?}");
        assert_eq!(codes.iter().filter(|c| **c == 10).count(), 1, "{codes:?}");
        let again = try_open(dir.path()).unwrap();
        let players = block_on(again.store.list_players(None, true)).unwrap();
        assert_eq!(players.len(), ok, "round {round}: lost writes {codes:?}");
    }
}

/// FIND-008: Finder metadata is not a trace of a lost store.
#[test]
fn ds_store_files_are_not_traces_of_a_lost_store() {
    let dir = TempDir::new().unwrap();
    drop(try_open(dir.path()).unwrap());
    for name in ["chess.db", "chess.db-wal", "chess.db-shm", "store.id"] {
        let _ = fs::remove_file(dir.path().join(name));
    }
    fs::write(dir.path().join("backups/.DS_Store"), b"x").unwrap();
    fs::write(dir.path().join("journal/.DS_Store"), b"x").unwrap();
    fs::write(dir.path().join("journal/done/.DS_Store"), b"x").unwrap();
    fs::write(dir.path().join("backups/._chess-x.db"), b"x").unwrap();
    let opened = try_open(dir.path()).unwrap();
    assert!(opened.report.first_launch);
    // A real file in backups/ still is a trace.
    drop(opened);
    for name in ["chess.db", "chess.db-wal", "chess.db-shm", "store.id"] {
        let _ = fs::remove_file(dir.path().join(name));
    }
    fs::write(dir.path().join("backups/chess-2026-11-01.db"), b"x").unwrap();
    assert!(matches!(try_open(dir.path()), Err(StoreError::Missing)));
}

const KILL_ENV: &str = "CHESS_STORE_KILL_DIR";

/// The child half of the kill test: says `ready`, then does a first launch. The parent kills it
/// at a random instant.
#[test]
fn child_first_launch_for_kill() {
    let Ok(dir) = std::env::var(KILL_ENV) else {
        return;
    };
    println!("CHILD ready");
    let _ = block_on(Store::open(options(Path::new(&dir))));
}

/// A xorshift64 generator: a seed on the failure message reproduces the delays (not the
/// scheduling).
fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// FIND-RW1-01 (ruling 2026-11-23): a first launch killed with SIGKILL at a random instant inside
/// `Store::open` on a fresh directory is never refused afterwards. No data ever existed, so
/// the next open is a first launch (or an ordinary open when the kill came after the schema
/// commit); `Missing` is the defect. `CHESS_STORE_KILL_ROUNDS` overrides the 500 rounds (FIND-F2-07, ruling 2026-11-24; the cost
/// is about four times the 120 rounds it replaces, see step2-store-B-evidence.md).
#[test]
fn rw1_01_a_first_launch_killed_at_a_random_instant_is_never_missing_afterwards() {
    let rounds: u32 = std::env::var("CHESS_STORE_KILL_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    // How long an open takes here: the kill delay is drawn from 0 to 1.5 times that.
    let timings: Vec<u128> = (0..5)
        .map(|_| {
            let d = TempDir::new().unwrap();
            let t = std::time::Instant::now();
            drop(try_open(d.path()).unwrap());
            t.elapsed().as_micros()
        })
        .collect();
    let typical = timings.iter().copied().max().unwrap().max(2_000);
    let seed = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            & 0xffff_ffff_ffff,
    )
    .unwrap()
        | 1;
    let mut state = seed;
    let (mut first, mut later, mut left_sidecar) = (0u32, 0u32, 0u32);
    for round in 0..rounds {
        let dir = TempDir::new().unwrap();
        let mut child = common::child::spawn(
            "child_first_launch_for_kill",
            &[(KILL_ENV, dir.path().to_str().unwrap())],
        );
        child.expect("ready");
        let delay = next_random(&mut state) % u64::try_from(typical * 3 / 2).unwrap();
        std::thread::sleep(std::time::Duration::from_micros(delay));
        child.kill();
        let sidecar = |name: &str| fs::metadata(dir.path().join(name)).is_ok_and(|m| m.len() > 0);
        left_sidecar += u32::from(sidecar("chess.db-wal") || sidecar("chess.db-journal"));
        match try_open(dir.path()) {
            Ok(opened) => {
                if opened.report.first_launch {
                    first += 1;
                } else {
                    later += 1;
                }
            }
            Err(e) => panic!(
                "round {round} (seed {seed}, delay {delay} us): the reopen after a kill is {e:?}"
            ),
        }
    }
    eprintln!(
        "kill rounds {rounds}, seed {seed}, open takes up to {typical} us: reopened as first launch {first}, \
         as a later open {later}, rounds that left a non-empty -wal or -journal {left_sidecar}"
    );
}
