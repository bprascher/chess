//! Part B: the process registry, the game lease rows and the open-time guard (F-A03), with
//! **two real processes** (the test binary re-runs itself as a child; `tests/common/child.rs`).
//! CT-ST-013 (other-version half), CT-ST-014, CT-ST-015, CT-ST-006 (close-on-exec), FIND-010
//! (guard before backup), NF-1 (concurrent upgraders), D1-01 / NF-3 (stale scratch files),
//! stale-lease takeover, `lease_*` and `register_process` through the public `Store`.
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

use chess_domain::{GameStatus, Matchup, Mode};
use chess_store::journal::LeaseState;
use chess_store::{
    DataDir, ExclusiveReason, JournalError, LeaseBeat, Store, StoreError, StoreOptions,
};
use common::child::{self, Spawned};
use common::{
    block_on, dead_pid, fixture_v1, gid, human, new_game, open_in, options, pid, raw,
    ticking_clock, v2,
};
use rusqlite::{Connection, params};
use tempfile::TempDir;

const ENV_SCENARIO: &str = "CHESS_STORE_B_SCENARIO";
const ENV_DIR: &str = "CHESS_STORE_B_DIR";
const ENV_KIND: &str = "CHESS_STORE_B_KIND";
const ENV_VERSION: &str = "CHESS_STORE_B_VERSION";
const ENV_SCHEMA: &str = "CHESS_STORE_B_SCHEMA";

fn open_as(
    dir: &Path,
    kind: &str,
    version: &str,
    schema2: bool,
) -> Result<chess_store::OpenedStore, StoreError> {
    let mut opts: StoreOptions = options(dir).with_process(kind, version);
    if schema2 {
        opts = opts.with_migrator(v2());
    }
    block_on(Store::open(opts))
}

// ------------------------------------------------------------------------------ the child

/// The child. Scenarios: `hold` (open, report READY, stay until standard input closes),
/// `upgrade` (open when told `go`, report the result), `lease` (create a journal, take the
/// lease row, start a grandchild process, report READY).
#[test]
fn child_entry() {
    let Ok(scenario) = std::env::var(ENV_SCENARIO) else {
        return;
    };
    let dir = std::env::var(ENV_DIR).unwrap();
    let kind = std::env::var(ENV_KIND).unwrap_or_else(|_| "server".into());
    let version = std::env::var(ENV_VERSION).unwrap_or_else(|_| "1.0.0".into());
    let schema2 = std::env::var(ENV_SCHEMA).is_ok_and(|v| v == "2");
    match scenario.as_str() {
        "hold" => {
            let opened = open_as(Path::new(&dir), &kind, &version, schema2).unwrap();
            println!("CHILD READY {}", std::process::id());
            child::wait_for_stdin_eof();
            drop(opened);
        }
        "upgrade" => {
            child::read_stdin_line();
            let result = open_as(Path::new(&dir), &kind, &version, true);
            match result {
                Ok(opened) => {
                    println!(
                        "CHILD RESULT ok upgraded_from={:?}",
                        opened.report.upgraded_from
                    );
                }
                Err(e) => println!("CHILD RESULT err {e:?}"),
            }
        }
        "lease" => {
            let opened = open_as(Path::new(&dir), &kind, &version, false).unwrap();
            let store = opened.store;
            let game = gid(1);
            let writer = store.journal().unwrap().create(&game).unwrap();
            block_on(store.lease_upsert(game, store.owner())).unwrap();
            let mut grandchild = std::process::Command::new("sleep")
                .arg("120")
                .spawn()
                .unwrap();
            println!("CHILD READY {} {}", std::process::id(), grandchild.id());
            child::wait_for_stdin_eof();
            drop(writer);
            let _ = grandchild.kill();
            let _ = grandchild.wait();
        }
        other => panic!("unknown scenario {other}"),
    }
}

fn spawn_child(scenario: &str, dir: &Path, kind: &str, version: &str, schema2: bool) -> Spawned {
    child::spawn(
        "child_entry",
        &[
            (ENV_SCENARIO, scenario),
            (ENV_DIR, dir.to_str().unwrap()),
            (ENV_KIND, kind),
            (ENV_VERSION, version),
            (ENV_SCHEMA, if schema2 { "2" } else { "1" }),
        ],
    )
}

fn holder(dir: &Path, kind: &str, version: &str) -> Spawned {
    let child = spawn_child("hold", dir, kind, version, false);
    child.expect("READY");
    child
}

fn registry(dir: &Path) -> Vec<(String, String)> {
    let conn = Connection::open(dir.join("chess.db")).unwrap();
    let mut st = conn
        .prepare("SELECT owner, build_version FROM process_heartbeat ORDER BY owner")
        .unwrap();
    st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn user_version(dir: &Path) -> i64 {
    Connection::open(dir.join("chess.db"))
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    out.sort();
    out
}

// ---------------------------------------------------------------- CT-ST-014: other version

/// CT-ST-014 (PLAT-5, architecture 7.5): a live process of another build version refuses the
/// start with `OtherVersionRunning{version, kind}`; the refused process leaves no row; the
/// refusal ends the moment that process is killed (its pid is gone, its heartbeat is fresh).
#[test]
fn ct_st_014_a_live_process_of_another_version_refuses_the_open() {
    let dir = fixture_v1();
    let mut other = holder(dir.path(), "server", "9.9.9");
    let refused = open_as(dir.path(), "cli", "1.0.0", false).err().unwrap();
    match refused {
        StoreError::OtherVersionRunning { version, kind } => {
            assert_eq!((version.as_str(), kind.as_str()), ("9.9.9", "server"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        registry(dir.path()),
        vec![(format!("server:{}", other.pid()), "9.9.9".to_owned())],
        "no row for the refused process"
    );
    // The same version is welcome beside it, and both are listed live.
    let alongside = open_as(dir.path(), "cli", "9.9.9", false).unwrap();
    let live = block_on(alongside.store.list_live_processes()).unwrap();
    assert_eq!(live.len(), 2, "{live:?}");
    block_on(alongside.store.deregister_process()).unwrap();
    // `kill -9`: the pid check makes it dead at once; no waiting for the 30 s threshold.
    other.kill();
    let ok = open_as(dir.path(), "cli", "1.0.0", false).unwrap();
    assert_eq!(
        registry(dir.path()).len(),
        1,
        "the dead process's row was deleted"
    );
    drop(ok);
}

/// CT-ST-013, other-version half: a schema newer than the build is `SchemaTooNew` before any
/// registration, even beside a process of another version, and nothing is written.
#[test]
fn ct_st_013_newer_schema_is_refused_before_the_registry_is_touched() {
    let dir = fixture_v1();
    let other = holder(dir.path(), "server", "9.9.9");
    Connection::open(dir.path().join("chess.db"))
        .unwrap()
        .pragma_update(None, "user_version", 2)
        .unwrap();
    let rows_before = registry(dir.path());
    let err = open_as(dir.path(), "cli", "1.0.0", false).err().unwrap();
    assert!(
        matches!(err, StoreError::SchemaTooNew { db: 2, app: 1 }),
        "{err:?}"
    );
    assert_eq!(registry(dir.path()), rows_before);
    drop(other);
}

// ------------------------------------------------- CT-ST-015, FIND-010: alone to migrate

/// CT-ST-015 and FIND-010: an upgrade with another live process returns `Exclusive` **before**
/// the pre-upgrade backup, so the last good `chess.db.bak-v1` stays byte-identical and no temp
/// file appears; the database is not touched; the refusal ends when the other process dies, and
/// then the upgrade replaces the old copy.
#[test]
fn ct_st_015_an_upgrade_needs_no_other_live_process_and_refuses_before_the_backup() {
    let dir = fixture_v1();
    let backup = dir.path().join("chess.db.bak-v1");
    fs::write(&backup, b"last good copy").unwrap();
    let mut other = holder(dir.path(), "server", "1.0.0");
    let listing = names(dir.path());
    let err = open_as(dir.path(), "cli", "1.0.0", true).err().unwrap();
    assert!(
        matches!(
            err,
            StoreError::Exclusive {
                reason: ExclusiveReason::Migration
            }
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(&backup).unwrap(), b"last good copy", "FIND-010");
    assert_eq!(user_version(dir.path()), 1);
    let after: Vec<String> = names(dir.path())
        .into_iter()
        .filter(|n| !n.starts_with("chess.db-"))
        .collect();
    let before: Vec<String> = listing
        .into_iter()
        .filter(|n| !n.starts_with("chess.db-"))
        .collect();
    assert_eq!(after, before, "no temp file, no new file");
    assert_eq!(
        registry(dir.path()).len(),
        1,
        "the refused opener removed its row"
    );
    other.kill();
    let opened = open_as(dir.path(), "cli", "1.0.0", true).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_ne!(
        fs::read(&backup).unwrap(),
        b"last good copy",
        "replaced by a verified copy"
    );
    assert_eq!(user_version(dir.path()), 2);
}

/// A stale row (a crashed process, pid gone) does not stop an upgrade and is deleted.
#[test]
fn a_dead_process_row_does_not_block_an_upgrade() {
    let dir = fixture_v1();
    raw(dir.path())
        .execute(
            "INSERT INTO process_heartbeat VALUES (?1, '1.0.0', 4102444800000)",
            params![format!("server:{}", dead_pid())],
        )
        .unwrap();
    let opened = open_as(dir.path(), "cli", "1.0.0", true).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(registry(dir.path()).len(), 1);
}

/// NF-1 with the guard: several processes upgrade at the same moment. None gets a backup
/// failure from a collision, the database is consistent, and a retry afterwards succeeds.
#[test]
fn nf_1_concurrent_upgraders_do_not_destroy_each_others_backup() {
    for round in 0..6 {
        let dir = fixture_v1();
        let mut kids: Vec<Spawned> = (0..3)
            .map(|_| spawn_child("upgrade", dir.path(), "cli", "1.0.0", true))
            .collect();
        for kid in &mut kids {
            kid.send("go\n");
        }
        let results: Vec<String> = kids.iter().map(|k| k.expect("RESULT")).collect();
        for r in &results {
            assert!(
                r.starts_with("RESULT ok") || r.contains("Exclusive") || r.contains("Busy"),
                "round {round}: {results:?}"
            );
            assert!(
                !r.contains("BackupFailed") && !r.contains("Corrupt"),
                "{results:?}"
            );
        }
        for kid in &mut kids {
            assert!(kid.finish().success());
        }
        // Whatever happened, the store is consistent and the next solo open ends at version 2.
        let opened = open_as(dir.path(), "cli", "1.0.0", true).unwrap();
        assert_eq!(opened.report.schema_version, 2, "{results:?}");
        drop(opened);
        assert_eq!(user_version(dir.path()), 2);
        assert!(
            !names(dir.path())
                .iter()
                .any(|n| n.starts_with("chess.db.tmp-bak")),
            "{:?}",
            names(dir.path())
        );
        if let Ok(copy) = Connection::open(dir.path().join("chess.db.bak-v1")) {
            let v: i64 = copy
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .unwrap();
            assert_eq!(v, 1);
        }
    }
}

/// F-A03: the "PENDING part B" marker of the part A opener is gone.
#[test]
fn the_pending_part_b_marker_is_gone_from_open_rs() {
    let text = include_str!("../src/open.rs");
    assert!(!text.contains("PENDING part B"));
    assert!(
        !text.contains("&|_conn| Ok(())"),
        "the accepting guard is gone"
    );
}

// ----------------------------------------------------------- registry through the Store API

#[test]
fn register_heartbeat_deregister_and_list_through_the_store() {
    let dir = TempDir::new().unwrap();
    let opened = open_as(dir.path(), "desktop", "1.0.0", false).unwrap();
    let store = opened.store;
    let me = format!("desktop:{}", std::process::id());
    assert_eq!(store.owner(), me);
    // The open registered this process (also for a first launch: after the schema exists).
    assert_eq!(registry(dir.path()), vec![(me.clone(), "1.0.0".to_owned())]);
    let live = block_on(store.list_live_processes()).unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!(
        (live[0].kind.as_str(), live[0].pid),
        ("desktop", Some(std::process::id()))
    );
    let first_beat = live[0].heartbeat_at;
    block_on(store.process_heartbeat()).unwrap();
    let later = block_on(store.list_live_processes()).unwrap();
    assert!(
        later[0].heartbeat_at > first_beat,
        "the injected clock ticks"
    );
    assert!(block_on(store.register_process()).unwrap().is_empty());
    block_on(store.deregister_process()).unwrap();
    assert!(block_on(store.list_live_processes()).unwrap().is_empty());
    assert!(registry(dir.path()).is_empty());
    // A heartbeat after deregistration registers again.
    block_on(store.process_heartbeat()).unwrap();
    assert_eq!(registry(dir.path()).len(), 1);
}

#[test]
fn a_process_that_missed_its_heartbeats_for_30_s_is_stale_and_replaced_by_a_fresh_start() {
    let dir = fixture_v1();
    // A live pid (this test's own parent shell is not safe to use; use a real sleeping child).
    let mut sleeper = std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .unwrap();
    raw(dir.path())
        .execute(
            "INSERT INTO process_heartbeat VALUES (?1, '0.0.1', 1000)",
            params![format!("server:{}", sleeper.id())],
        )
        .unwrap();
    // The injected clock is far past 1000 + 30 s: the row is stale although its pid exists.
    let opts = options(dir.path()).with_clock(ticking_clock(1000 + 30_001));
    let opened = block_on(Store::open(opts)).unwrap();
    assert_eq!(registry(dir.path()).len(), 1);
    drop(opened);
    // With the clock at exactly 30 s the row is still live and refuses.
    raw(dir.path())
        .execute("DELETE FROM process_heartbeat", [])
        .unwrap();
    raw(dir.path())
        .execute(
            "INSERT INTO process_heartbeat VALUES (?1, '0.0.1', 1000)",
            params![format!("server:{}", sleeper.id())],
        )
        .unwrap();
    let opts = options(dir.path()).with_clock(ticking_clock(1000 + 30_000));
    let err = block_on(Store::open(opts)).err().unwrap();
    assert!(
        matches!(err, StoreError::OtherVersionRunning { .. }),
        "{err:?}"
    );
    sleeper.kill().unwrap();
    sleeper.wait().unwrap();
}

// ------------------------------------------------------------------------------- leases

fn world() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let store = open_as(dir.path(), "cli", "1.0.0", false).unwrap().store;
    for (n, name) in [(1, "Ann"), (2, "Bob")] {
        block_on(store.register_player(human(n, name))).unwrap();
    }
    block_on(store.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
    (dir, store)
}

#[test]
fn lease_rows_are_created_beaten_taken_over_and_deleted_by_owner() {
    let (dir, store) = world();
    let game = gid(1);
    assert_eq!(block_on(store.lease_owner(game)).unwrap(), None);
    assert_eq!(
        block_on(store.lease_upsert(game, "cli:11".into())).unwrap(),
        None
    );
    let row = block_on(store.lease_owner(game)).unwrap().unwrap();
    assert_eq!(row.owner, "cli:11");
    // The owner's beat refreshes the heartbeat.
    assert_eq!(
        block_on(store.lease_beat(game, "cli:11".into())).unwrap(),
        LeaseBeat::Held
    );
    let beaten = block_on(store.lease_owner(game)).unwrap().unwrap();
    assert!(beaten.heartbeat_at > row.heartbeat_at);
    // Another owner's beat is compare-and-set: it changes nothing and learns the owner.
    assert_eq!(
        block_on(store.lease_beat(game, "server:22".into())).unwrap(),
        LeaseBeat::Lost {
            owner: "cli:11".into()
        }
    );
    assert_eq!(block_on(store.lease_owner(game)).unwrap().unwrap(), beaten);
    // Takeover: the new owner (it holds the journal lock) overwrites the row and learns who had it.
    let previous = block_on(store.lease_upsert(game, "server:22".into()))
        .unwrap()
        .unwrap();
    assert_eq!(previous.owner, "cli:11");
    // The same owner again is not a takeover.
    assert_eq!(
        block_on(store.lease_upsert(game, "server:22".into())).unwrap(),
        None
    );
    assert_eq!(
        block_on(store.lease_beat(game, "cli:11".into())).unwrap(),
        LeaseBeat::Lost {
            owner: "server:22".into()
        }
    );
    // Delete is by owner: the old owner cannot delete the new owner's row.
    assert!(!block_on(store.lease_delete(game, "cli:11".into())).unwrap());
    assert!(block_on(store.lease_owner(game)).unwrap().is_some());
    assert!(block_on(store.lease_delete(game, "server:22".into())).unwrap());
    assert_eq!(block_on(store.lease_owner(game)).unwrap(), None);
    // Row gone: the beat returns the GAME row's state (in progress here: only the row was deleted).
    match block_on(store.lease_beat(game, "server:22".into())).unwrap() {
        LeaseBeat::RowGone {
            status,
            voided_by,
            content_hash,
            ended_at,
        } => {
            assert_eq!(status, GameStatus::InProgress);
            assert_eq!((voided_by, content_hash, ended_at), (None, None, None));
        }
        other => panic!("{other:?}"),
    }
    // A voided row is reported with what the session needs (D-67 (iii), D-69).
    raw(dir.path())
        .execute(
            "UPDATE game SET status = 'voided', ended_at = 777, content_hash = 'sha256:v1:aa', \
             voided_by = (SELECT id FROM player WHERE name = 'Bob'), termination = 'voided by reset' \
             WHERE uuid = ?1",
            [gid(1).as_uuid().hyphenated().to_string()],
        )
        .unwrap();
    match block_on(store.lease_beat(game, "server:22".into())).unwrap() {
        LeaseBeat::RowGone {
            status,
            voided_by,
            content_hash,
            ended_at,
        } => {
            assert_eq!(status, GameStatus::Voided);
            assert_eq!(voided_by, Some(pid(2)));
            assert_eq!(content_hash.as_deref(), Some("sha256:v1:aa"));
            assert_eq!(ended_at.map(|t| t.0), Some(777));
        }
        other => panic!("{other:?}"),
    }
    // Unknown games.
    assert!(matches!(
        block_on(store.lease_upsert(gid(99), "cli:1".into())),
        Err(StoreError::NotFound { .. })
    ));
    assert!(matches!(
        block_on(store.lease_beat(gid(99), "cli:1".into())),
        Err(StoreError::NotFound { .. })
    ));
    assert!(!block_on(store.lease_delete(gid(99), "cli:1".into())).unwrap());
}

/// DAT-4, CT-ST-025/026: a second process sees the game as held; after `kill -9` of the owner
/// the lock is free at once (no waiting), the lease row still names the dead owner, and the new
/// owner takes it over; the old owner's beat says it lost the row.
#[test]
fn stale_lease_takeover_after_kill_9_with_two_processes() {
    let (dir, store) = world();
    let mut owner = spawn_child("lease", dir.path(), "server", "1.0.0", false);
    let ready = owner.expect("READY");
    let grandchild: u32 = ready.split_whitespace().nth(2).unwrap().parse().unwrap();
    let journal = store.journal().unwrap();
    assert_eq!(journal.probe(&gid(1)), LeaseState::Held);
    match journal.create(&gid(1)) {
        Err(JournalError::Locked { .. }) => {}
        other => panic!("{other:?}"),
    }
    let child_owner = format!("server:{}", owner.pid());
    assert_eq!(
        block_on(store.lease_owner(gid(1))).unwrap().unwrap().owner,
        child_owner
    );
    owner.kill();
    assert_eq!(
        journal.probe(&gid(1)),
        LeaseState::Free,
        "the OS released the lock at once"
    );
    // CT-ST-006: the grandchild still runs; had it inherited the descriptor it would hold the lock.
    assert!(
        std::process::Command::new("/bin/kill")
            .arg("-0")
            .arg(grandchild.to_string())
            .status()
            .unwrap()
            .success(),
        "the grandchild is alive"
    );
    // The lease row is stale (display only) and is taken over.
    assert_eq!(
        block_on(store.lease_owner(gid(1))).unwrap().unwrap().owner,
        child_owner
    );
    let me = store.owner();
    let previous = block_on(store.lease_upsert(gid(1), me.clone()))
        .unwrap()
        .unwrap();
    assert_eq!(previous.owner, child_owner);
    assert_eq!(
        block_on(store.lease_beat(gid(1), child_owner)).unwrap(),
        LeaseBeat::Lost { owner: me }
    );
    let _ = std::process::Command::new("/bin/kill")
        .arg(grandchild.to_string())
        .status();
}

// ----------------------------------------------------- D1-01 / NF-3: scratch files

#[test]
fn d1_01_stale_scratch_files_of_dead_processes_are_removed_by_the_next_open() {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path())); // a real store first
    let gone = dead_pid();
    let live = std::process::Command::new("sleep").arg("60").spawn();
    let mut live = live.unwrap();
    let make = |name: String| fs::write(dir.path().join(name), b"x").unwrap();
    make(format!(".durability-probe-{gone}-0"));
    make(format!(".store.id.tmp-{gone}-3"));
    make(format!(".durability-probe-{}-9", live.id())); // a live process: left alone
    make(format!(".store.id.tmp-{}-9", std::process::id())); // this process: left alone
    make(".store.id.tmp-notapid-1".to_owned()); // not ours to judge: left alone
    make(".unrelated".to_owned());
    drop(open_in(dir.path()));
    let left: Vec<String> = names(dir.path())
        .into_iter()
        .filter(|n| n.starts_with('.'))
        .collect();
    assert_eq!(
        left,
        vec![
            format!(".durability-probe-{}-9", live.id()),
            format!(".store.id.tmp-{}-9", std::process::id()),
            ".store.id.tmp-notapid-1".to_owned(),
            ".unrelated".to_owned(),
        ]
    );
    live.kill().unwrap();
    live.wait().unwrap();
}

/// NF-3: a stale temp with this very pid and counter 0 no longer makes a first launch `Corrupt`.
#[test]
fn nf_3_a_stale_store_id_temp_with_a_reused_pid_does_not_break_the_first_launch() {
    let dir = TempDir::new().unwrap();
    for n in 0..8 {
        fs::write(
            dir.path()
                .join(format!(".store.id.tmp-{}-{n}", std::process::id())),
            b"stale",
        )
        .unwrap();
    }
    // Counter values are process-wide, so cover whichever values this process uses next.
    let opened = open_in(dir.path());
    assert!(opened.report.first_launch);
    assert!(DataDir::new(dir.path()).read_store_id().unwrap().is_some());
}

/// M38 (NF-7): the success path leaves no `.store.id.tmp-*` and no probe file.
#[test]
fn a_successful_first_launch_leaves_no_scratch_file() {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    let hidden: Vec<String> = names(dir.path())
        .into_iter()
        .filter(|n| n.starts_with('.'))
        .collect();
    assert!(hidden.is_empty(), "{hidden:?}");
}

/// A second `Store` in the same process is the same owner (one row), and the registry stays
/// valid when it is dropped without deregistering.
#[test]
fn two_stores_in_one_process_share_one_registry_row() {
    let dir = TempDir::new().unwrap();
    let a = open_in(dir.path());
    let b = open_in(dir.path());
    assert_eq!(registry(dir.path()).len(), 1);
    drop((a, b));
    assert_eq!(
        registry(dir.path()).len(),
        1,
        "no deregistration on drop; the pid check cleans it"
    );
}
