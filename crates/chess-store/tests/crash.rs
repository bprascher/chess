//! Part D1: the kill-at-failpoint harness and the crash tests that use it (acceptance (3),
//! CT-ST-090 kill-before-rename; kill points of the first launch and the upgrade).
//!
//! How it works: the test binary re-runs itself (`current_exe`, `--exact crash_child_entry`)
//! with a scenario, a directory and a plan in environment variables. The child installs the plan
//! with `FaultPlan::parse`, runs the scenario against the real `Store::open`, and at a `Kill`
//! failpoint prints `FAILPOINT-KILL <name>` and waits. The parent reads that line, calls
//! `Child::kill` (SIGKILL), waits, and reports the failpoint the child stopped at. Then the
//! parent opens the directory itself. Without the feature this file is empty.
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
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use chess_store::fault::{FaultPlan, KILL_MARKER};
use chess_store::{DataDir, MIGRATIONS, Migration, Migrator, Store, StoreId};
use common::{block_on, options};
use rusqlite::Connection;
use tempfile::TempDir;

const ENV_SCENARIO: &str = "CHESS_STORE_CRASH_SCENARIO";
const ENV_DIR: &str = "CHESS_STORE_CRASH_DIR";
const ENV_PLAN: &str = "CHESS_STORE_CRASH_PLAN";

fn v2() -> Migrator {
    Migrator::with(&[
        MIGRATIONS[0],
        Migration {
            version: 2,
            sql: "CREATE TABLE note (id INTEGER PRIMARY KEY, body TEXT NOT NULL);",
        },
    ])
    .unwrap()
}

/// A store at schema version 1 with its identity file.
fn fixture_v1() -> TempDir {
    let dir = TempDir::new().unwrap();
    let conn = Connection::open(dir.path().join("chess.db")).unwrap();
    conn.execute_batch(include_str!("fixtures/schema-v1.sql"))
        .unwrap();
    drop(conn);
    DataDir::new(dir.path())
        .write_store_id(&StoreId {
            store_uuid: uuid::Uuid::from_u128(5),
            created_at: 1,
        })
        .unwrap();
    dir
}

fn user_version(db: &Path) -> i64 {
    Connection::open(db)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

// ------------------------------------------------------------------------------ the child

/// Runs only when the parent set the scenario variable; otherwise it passes at once, so a
/// plain `cargo test` is not affected.
#[test]
fn crash_child_entry() {
    let Ok(scenario) = std::env::var(ENV_SCENARIO) else {
        return;
    };
    let dir = std::env::var(ENV_DIR).expect("directory");
    let plan = FaultPlan::parse(&std::env::var(ENV_PLAN).expect("plan")).expect("plan syntax");
    let _guard = plan.install(Path::new(&dir)).expect("registered names");
    let result = match scenario.as_str() {
        "first_launch" => block_on(Store::open(options(Path::new(&dir)))).map(|_| ()),
        "upgrade_v2" => {
            block_on(Store::open(options(Path::new(&dir)).with_migrator(v2()))).map(|_| ())
        }
        other => panic!("unknown scenario {other}"),
    };
    // Reaching here means no `Kill` fired. A refusal from an injected error is a normal end.
    assert!(result.is_ok(), "scenario ended with {result:?}");
}

// ----------------------------------------------------------------------------- the parent

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The child printed the marker and the parent killed it; the failpoint it stopped at.
    Killed { at: String },
    /// The child ran to its end (no `Kill` fired); whether it succeeded.
    Finished { success: bool },
}

/// Runs `scenario` in a child with `plan` installed for `dir`, kills it at the first `Kill`
/// failpoint and reports where it died (acceptance (3)).
fn run_child(scenario: &str, dir: &Path, plan: &str) -> Outcome {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "crash_child_entry",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ENV_SCENARIO, scenario)
        .env(ENV_DIR, dir)
        .env(ENV_PLAN, plan)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(name) = line.strip_prefix(&format!("{KILL_MARKER} ")) {
                let _ = tx.send(name.to_owned());
                return;
            }
        }
        // EOF: the child ended without the marker; the sender drops.
    });
    match rx.recv_timeout(Duration::from_secs(60)) {
        Ok(at) => {
            child.kill().unwrap();
            let status = child.wait().unwrap();
            assert!(!status.success(), "a killed child does not succeed");
            Outcome::Killed { at }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Outcome::Finished {
            success: child.wait().unwrap().success(),
        },
        Err(mpsc::RecvTimeoutError::Timeout) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the child neither hit a failpoint nor finished in 60 s");
        }
    }
}

fn killed_at(name: &str) -> Outcome {
    Outcome::Killed {
        at: name.to_owned(),
    }
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

// ------------------------------------------------------------------ harness behaviour

#[test]
fn the_harness_reports_the_failpoint_the_child_died_at() {
    let dir = TempDir::new().unwrap();
    let outcome = run_child("first_launch", dir.path(), "open.after_db_created=kill");
    assert_eq!(outcome, killed_at("open.after_db_created"));
}

#[test]
fn the_harness_reports_a_child_that_never_reached_its_failpoint() {
    // A first launch never reaches `backup.before_rename`.
    let dir = TempDir::new().unwrap();
    let outcome = run_child("first_launch", dir.path(), "backup.before_rename=kill");
    assert_eq!(outcome, Outcome::Finished { success: true });
    assert_eq!(user_version(&dir.path().join("chess.db")), 1);
}

#[test]
fn skip_lets_the_first_hits_pass_in_the_child() {
    // `sync.store_id_dir` is hit once per first launch; with @1 the kill never fires.
    let dir = TempDir::new().unwrap();
    let outcome = run_child("first_launch", dir.path(), "sync.store_id_dir=kill@1");
    assert_eq!(outcome, Outcome::Finished { success: true });
}

#[test]
fn a_child_that_fails_for_another_reason_is_reported_as_not_successful() {
    // An injected error that the scenario cannot absorb: the open fails, the child's assert
    // fails, and the harness says so (it is not mistaken for a kill).
    let dir = TempDir::new().unwrap();
    let outcome = run_child("first_launch", dir.path(), "sync.store_id_file=errno:5");
    assert_eq!(outcome, Outcome::Finished { success: false });
}

// --------------------------------------------------------------------- CT-ST-090 kill

/// CT-ST-090: a kill at `backup.before_rename` leaves the previous backup byte-identical and a
/// stale temp file; the retry removes the temp file and succeeds with a verified copy.
#[test]
fn ct_st_090_a_kill_before_the_rename_keeps_the_old_copy_and_the_retry_succeeds() {
    let dir = fixture_v1();
    let backup = dir.path().join("chess.db.bak-v1");
    fs::write(&backup, b"previous backup, not a database").unwrap();
    let before = fs::read(&backup).unwrap();

    let outcome = run_child("upgrade_v2", dir.path(), "backup.before_rename=kill");
    assert_eq!(outcome, killed_at("backup.before_rename"));
    // NF-1: the temp name carries the pid of the process that wrote it (the child, now dead).
    let temps: Vec<_> = files(dir.path())
        .into_iter()
        .filter(|n| n.starts_with("chess.db.tmp-bak-v1-"))
        .collect();
    assert_eq!(temps.len(), 1, "{temps:?}");
    let temp = dir.path().join(&temps[0]);

    // The previous copy is untouched; the stale temp file is a complete verified copy that is
    // not named like a backup; the upgrade did not start.
    assert_eq!(fs::read(&backup).unwrap(), before);
    assert!(temp.exists(), "the stale temp file stays after a kill");
    assert_eq!(user_version(&temp), 1);
    assert_eq!(user_version(&dir.path().join("chess.db")), 1);
    let backups: Vec<_> = files(dir.path())
        .into_iter()
        .filter(|n| n.starts_with("chess.db.bak-v"))
        .collect();
    assert_eq!(
        backups,
        ["chess.db.bak-v1"],
        "the temp file is not a backup"
    );

    // The parent opens the directory itself: the next run removes the temp file, replaces the
    // old copy with a verified one and upgrades.
    let opened = block_on(Store::open(options(dir.path()).with_migrator(v2()))).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(opened.report.schema_version, 2);
    drop(opened);
    assert!(!temp.exists(), "the next run removes the stale temp file");
    assert_ne!(fs::read(&backup).unwrap(), before);
    assert_eq!(user_version(&backup), 1);
}

#[test]
fn a_kill_after_the_new_backup_and_before_the_upgrade_leaves_a_directory_that_upgrades() {
    let dir = fixture_v1();
    let outcome = run_child("upgrade_v2", dir.path(), "migrate.after_backup=kill");
    assert_eq!(outcome, killed_at("migrate.after_backup"));
    let backup = dir.path().join("chess.db.bak-v1");
    assert_eq!(user_version(&backup), 1, "the new copy is in place");
    assert_eq!(user_version(&dir.path().join("chess.db")), 1);
    let opened = block_on(Store::open(options(dir.path()).with_migrator(v2()))).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(opened.report.schema_version, 2);
}

// ------------------------------------------------------------- first-launch kill points

#[test]
fn a_kill_after_chess_db_is_created_leaves_a_directory_that_opens_as_a_first_launch() {
    let dir = TempDir::new().unwrap();
    let outcome = run_child("first_launch", dir.path(), "open.after_db_created=kill");
    assert_eq!(outcome, killed_at("open.after_db_created"));
    assert_eq!(
        fs::metadata(dir.path().join("chess.db")).unwrap().len(),
        0,
        "an empty database file"
    );
    assert!(!dir.path().join("store.id").exists());
    let opened = block_on(Store::open(options(dir.path()))).unwrap();
    assert!(opened.report.first_launch);
    assert_eq!(opened.report.schema_version, 1);
}

#[test]
fn a_kill_after_store_id_keeps_the_identity_and_opens_as_a_first_launch() {
    let dir = TempDir::new().unwrap();
    let outcome = run_child("first_launch", dir.path(), "open.after_store_id=kill");
    assert_eq!(outcome, killed_at("open.after_store_id"));
    let data = DataDir::new(dir.path());
    let first = data.read_store_id().unwrap().unwrap();
    let opened = block_on(Store::open(options(dir.path()))).unwrap();
    assert!(
        opened.report.first_launch,
        "no schema yet, so still a first launch"
    );
    assert_eq!(
        data.read_store_id().unwrap().unwrap().store_uuid,
        first.store_uuid
    );
}

#[test]
fn a_kill_at_every_sync_of_the_first_launch_leaves_a_directory_that_opens() {
    for name in ["sync.probe", "sync.store_id_file", "sync.store_id_dir"] {
        let dir = TempDir::new().unwrap();
        let outcome = run_child("first_launch", dir.path(), &format!("{name}=kill"));
        assert_eq!(outcome, killed_at(name), "{name}");
        let opened = block_on(Store::open(options(dir.path()))).unwrap();
        assert_eq!(opened.report.schema_version, 1, "{name}");
        // FIND-D1-01 (pinned, not changed): a process killed inside the probe or the
        // `store.id` write leaves its scratch file behind, and no later open removes it. The
        // open itself is not affected. Anything else left over would be a new defect.
        drop(opened);
        for leftover in files(dir.path()) {
            let known = leftover.starts_with(".durability-probe-")
                || leftover.starts_with(".store.id.tmp-")
                || matches!(
                    leftover.as_str(),
                    "chess.db"
                        | "chess.db-wal"
                        | "chess.db-shm"
                        | "store.id"
                        | "journal"
                        | "backups"
                );
            assert!(known, "{name}: unexpected leftover {leftover}");
        }
    }
}
