//! Part D1: `FaultPlan` itself (acceptance (2)), CT-ST-085 through the real `Store::open`
//! (acceptance (4)), and the injection tests that close the sync and verify survivors of the
//! part A review (FIND-013: M16, M17, M22, M30, M31, M35). Without the feature this file is
//! empty.
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
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use chess_store::fault::{self, FaultPlan};
use chess_store::{
    BackupCause, DataDir, MIGRATIONS, Migration, Migrator, Store, StoreError, StoreId,
    UnavailableCause,
};
use common::{block_on, options};
use rusqlite::Connection;
use tempfile::TempDir;

const ENOTSUP: i32 = 45;
const ENOTTY: i32 = 25;
const EIO: i32 = 5;

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

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

fn open_v2(dir: &Path) -> Result<chess_store::OpenedStore, StoreError> {
    block_on(Store::open(options(dir).with_migrator(v2())))
}

// ------------------------------------------------------------------ the registry (2)

/// The 13 names of SDD section 4 (DD-ST-024), copied literally, and the three of Addendum K.
const SDD_NAMES: [&str; 13] = [
    "journal.before_write",
    "journal.after_write",
    "journal.before_fsync",
    "journal.after_fsync",
    "apply.before_commit",
    "apply.after_commit",
    "finalize.after_journal_end",
    "finalize.before_commit",
    "finalize.after_commit_before_rename",
    "reset.after_commit_before_voided_record",
    "backup.after_copy",
    "restore.after_replay",
    "restore.before_rename",
];
const NEW_NAMES: [&str; 3] = [
    "backup.before_rename",
    "journal.rebase.old_linked",
    "journal.rebase.new_linked",
];

#[test]
fn every_sdd_failpoint_and_the_three_new_names_is_registered_and_reachable() {
    let registered: Vec<&str> = fault::all().collect();
    let dir = TempDir::new().unwrap();
    for name in SDD_NAMES.iter().chain(&NEW_NAMES) {
        assert!(registered.contains(name), "{name} is not registered");
        // Armed with an errno, reached through the public trigger: the error comes out.
        let guard = FaultPlan::new()
            .errno(name, EIO)
            .install(dir.path())
            .unwrap();
        let reached = fault::trigger(name, &dir.path().join("x"));
        assert_eq!(
            reached.unwrap_err().raw_os_error(),
            Some(EIO),
            "{name} did not fire"
        );
        assert_eq!((guard.hits(name), guard.fired(name)), (1, 1), "{name}");
    }
    let mut unique = registered.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), registered.len(), "a name is registered twice");
}

#[test]
fn every_registered_name_accepts_every_action_kind() {
    let dir = TempDir::new().unwrap();
    let ctx = dir.path().join("x");
    for name in fault::all() {
        let guard = FaultPlan::new()
            .fail(name, io::ErrorKind::PermissionDenied)
            .install(dir.path())
            .unwrap();
        assert_eq!(
            fault::trigger(name, &ctx).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        drop(guard);
        let guard = FaultPlan::new()
            .delay(name, Duration::ZERO)
            .install(dir.path())
            .unwrap();
        fault::trigger(name, &ctx).unwrap();
        assert_eq!(guard.fired(name), 1);
    }
}

#[test]
fn an_unknown_failpoint_is_refused_at_install_and_at_trigger() {
    let dir = TempDir::new().unwrap();
    let refused = FaultPlan::new()
        .errno("journal.before_wirte", EIO)
        .install(dir.path());
    assert_eq!(
        refused.unwrap_err().to_string(),
        "unknown failpoint \"journal.before_wirte\""
    );
    assert_eq!(
        fault::trigger("nope", dir.path()).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn a_plan_applies_only_under_its_directory_and_only_while_the_guard_lives() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let guard = FaultPlan::new()
        .errno("backup.verify", EIO)
        .install(a.path())
        .unwrap();
    assert!(fault::trigger("backup.verify", &a.path().join("f")).is_err());
    assert!(
        fault::trigger("backup.verify", &b.path().join("f")).is_ok(),
        "another directory is not affected"
    );
    assert_eq!(
        guard.hits("backup.verify"),
        1,
        "the other directory did not count"
    );
    drop(guard);
    assert!(fault::trigger("backup.verify", &a.path().join("f")).is_ok());
}

#[test]
fn a_nested_directory_plan_overrides_the_outer_one() {
    let outer = TempDir::new().unwrap();
    let inner = outer.path().join("inner");
    let _o = FaultPlan::new()
        .errno("backup.verify", EIO)
        .install(outer.path())
        .unwrap();
    let _i = FaultPlan::new().install(&inner).unwrap();
    assert!(fault::trigger("backup.verify", &inner.join("f")).is_ok());
    assert!(fault::trigger("backup.verify", &outer.path().join("f")).is_err());
}

#[test]
fn skip_and_once_count_hits() {
    let dir = TempDir::new().unwrap();
    let ctx = dir.path().join("f");
    let guard = FaultPlan::new()
        .errno("apply.before_commit", EIO)
        .skip(2)
        .once()
        .install(dir.path())
        .unwrap();
    assert!(fault::trigger("apply.before_commit", &ctx).is_ok());
    assert!(fault::trigger("apply.before_commit", &ctx).is_ok());
    assert!(fault::trigger("apply.before_commit", &ctx).is_err());
    assert!(
        fault::trigger("apply.before_commit", &ctx).is_ok(),
        "once: the rule is gone"
    );
    assert_eq!(
        (
            guard.hits("apply.before_commit"),
            guard.fired("apply.before_commit")
        ),
        (4, 1)
    );
}

#[test]
fn a_rule_without_once_fires_on_every_hit() {
    let dir = TempDir::new().unwrap();
    let guard = FaultPlan::new()
        .fail("apply.after_commit", io::ErrorKind::TimedOut)
        .install(dir.path())
        .unwrap();
    for _ in 0..3 {
        assert!(fault::trigger("apply.after_commit", dir.path()).is_err());
    }
    assert_eq!(guard.fired("apply.after_commit"), 3);
    assert_eq!(guard.fired("apply.before_commit"), 0);
}

#[test]
fn delay_sleeps_and_continues() {
    let dir = TempDir::new().unwrap();
    let _g = FaultPlan::new()
        .delay("journal.after_fsync", Duration::from_millis(60))
        .install(dir.path())
        .unwrap();
    let start = Instant::now();
    fault::trigger("journal.after_fsync", dir.path()).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(60));
}

#[test]
fn the_text_form_parses_every_action_and_rejects_bad_input() {
    let dir = TempDir::new().unwrap();
    let guard =
        FaultPlan::parse("sync.probe=errno:45@1,backup.verify=delay:0,restore.before_rename=kill")
            .unwrap()
            .install(dir.path())
            .unwrap();
    assert!(
        fault::trigger("sync.probe", dir.path()).is_ok(),
        "skipped once"
    );
    assert_eq!(
        fault::trigger("sync.probe", dir.path())
            .unwrap_err()
            .raw_os_error(),
        Some(ENOTSUP)
    );
    fault::trigger("backup.verify", dir.path()).unwrap();
    assert_eq!(guard.fired("backup.verify"), 1);
    // `restore.before_rename=kill` is armed but not triggered here (a kill needs a child).
    assert_eq!(guard.hits("restore.before_rename"), 0);
    for bad in [
        "x",
        "a=b",
        "a=errno:x",
        "a=delay:x",
        "a=kill@x",
        "a=errno",
        "a=boom:1",
    ] {
        assert!(FaultPlan::parse(bad).is_err(), "{bad}");
    }
    // An empty plan is valid. Its own directory: a second plan for `dir`, whose guard is still
    // alive above, is refused since FIND-D1R-02.
    let other = TempDir::new().unwrap();
    assert!(FaultPlan::parse("").unwrap().install(other.path()).is_ok());
}

/// FIND-D1R-03: the once-suffix `!` and its order with `@n` are pinned by D1's own suite. Without
/// `!` the rule fires on every hit; with it, once; `@1!` passes one hit, fires one, then is gone.
#[test]
fn the_once_suffix_makes_a_parsed_rule_fire_once_after_its_skip() {
    let hits = |spec: &str, calls: usize| -> Vec<bool> {
        let dir = TempDir::new().unwrap();
        let _guard = FaultPlan::parse(spec).unwrap().install(dir.path()).unwrap();
        (0..calls)
            .map(|_| fault::trigger("apply.before_commit", dir.path()).is_err())
            .collect()
    };
    assert_eq!(
        hits("apply.before_commit=errno:5", 4),
        [true, true, true, true]
    );
    assert_eq!(
        hits("apply.before_commit=errno:5!", 4),
        [true, false, false, false]
    );
    assert_eq!(
        hits("apply.before_commit=errno:5@1!", 4),
        [false, true, false, false]
    );
    assert_eq!(
        hits("apply.before_commit=errno:5@2", 4),
        [false, false, true, true]
    );
    // The suffix belongs to its own rule only.
    let dir = TempDir::new().unwrap();
    let guard = FaultPlan::parse("apply.before_commit=errno:5!,apply.after_commit=errno:5")
        .unwrap()
        .install(dir.path())
        .unwrap();
    for _ in 0..3 {
        let _ = fault::trigger("apply.before_commit", dir.path());
        let _ = fault::trigger("apply.after_commit", dir.path());
    }
    assert_eq!(guard.fired("apply.before_commit"), 1);
    assert_eq!(guard.fired("apply.after_commit"), 3);
    // A bad number after `!`-less or `!`-ful forms stays an error.
    assert!(FaultPlan::parse("a=errno:5!x").is_err());
    assert!(FaultPlan::parse("a=errno:5!@1").is_err());
}

/// FIND-D1R-02: a root with `..` after a component that does not exist can never match a context
/// and is refused at install; a `..` after existing components resolves and fires.
#[test]
fn a_root_with_a_parent_step_after_a_missing_component_is_refused_at_install() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("real")).unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    let bad = dir.path().join("real/nope/../sub");
    let refused = FaultPlan::new().errno("backup.verify", EIO).install(&bad);
    assert!(
        matches!(
            refused,
            Err(fault::InstallError::UnresolvableRoot(ref root)) if *root == bad
        ),
        "{refused:?}"
    );
    // `..` after a missing component deeper in the path, and as the very last component.
    for bad in ["a/b/../..", "nope/..", "x/y/z/../w"] {
        assert!(
            matches!(
                FaultPlan::new().install(&dir.path().join(bad)),
                Err(fault::InstallError::UnresolvableRoot(_))
            ),
            "{bad}"
        );
    }
    // After existing components `..` resolves, and the plan fires for the resolved directory.
    let good = dir.path().join("real/../sub");
    let guard = FaultPlan::new()
        .errno("backup.verify", EIO)
        .install(&good)
        .unwrap();
    assert!(fault::trigger("backup.verify", &dir.path().join("sub/f")).is_err());
    assert_eq!(guard.fired("backup.verify"), 1);
    // A missing root without `..` is allowed (the nested-directory test relies on it).
    assert!(FaultPlan::new().install(&dir.path().join("later")).is_ok());
}

/// FIND-D1R-02: a second plan for a root that already has one is refused, whatever the spelling
/// of the root; after the first guard is dropped the root is free again.
#[test]
fn a_second_plan_for_the_same_root_is_refused_until_the_first_guard_drops() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    let first = FaultPlan::new()
        .errno("backup.verify", EIO)
        .install(dir.path())
        .unwrap();
    for spelling in [dir.path().to_path_buf(), dir.path().join("sub/..")] {
        let second = FaultPlan::new()
            .errno("backup.verify", ENOTSUP)
            .install(&spelling);
        assert!(
            matches!(second, Err(fault::InstallError::DuplicateRoot(_))),
            "{spelling:?}: {second:?}"
        );
    }
    // The refused plan changed nothing: the first plan still fires with its own errno.
    let err = fault::trigger("backup.verify", &dir.path().join("f")).unwrap_err();
    assert_eq!(err.raw_os_error(), Some(EIO));
    assert_eq!(first.fired("backup.verify"), 1);
    // A nested root is not a duplicate.
    assert!(FaultPlan::new().install(&dir.path().join("sub")).is_ok());
    drop(first);
    assert!(FaultPlan::new().install(dir.path()).is_ok());
}

/// Every name that the part A code calls is reached by the real code paths (a zero delay
/// counts the call without changing the outcome). The names of later parts are reached only
/// through `trigger` (see the registry test).
#[test]
fn the_wired_failpoints_are_reached_by_the_real_open_and_upgrade() {
    const WIRED: [&str; 9] = [
        "sync.probe",
        "sync.store_id_file",
        "sync.store_id_dir",
        "open.after_db_created",
        "open.after_store_id",
        "sync.backup_copy_file",
        "sync.backup_copy_dir",
        "sync.backup_rename_dir",
        "backup.before_rename",
    ];
    const WIRED_MORE: [&str; 2] = ["backup.verify", "migrate.after_backup"];
    let delay_all = |dir: &Path| {
        let mut plan = FaultPlan::new();
        for name in WIRED.iter().chain(&WIRED_MORE) {
            plan = plan.delay(name, Duration::ZERO);
        }
        plan.install(dir).unwrap()
    };
    // First launch: the probe, the store identity and the creation points.
    let fresh = TempDir::new().unwrap();
    let guard = delay_all(fresh.path());
    drop(block_on(Store::open(options(fresh.path()))).unwrap());
    for name in [
        "sync.probe",
        "sync.store_id_file",
        "sync.store_id_dir",
        "open.after_db_created",
        "open.after_store_id",
    ] {
        assert!(
            guard.fired(name) >= 1,
            "{name} not reached by a first launch"
        );
    }
    // Upgrade: the backup and the migration.
    let old = fixture_v1();
    let guard = delay_all(old.path());
    drop(open_v2(old.path()).unwrap());
    for name in [
        "sync.backup_copy_file",
        "sync.backup_copy_dir",
        "sync.backup_rename_dir",
        "backup.before_rename",
        "backup.verify",
        "migrate.after_backup",
    ] {
        assert!(guard.fired(name) >= 1, "{name} not reached by an upgrade");
    }
}

// ---------------------------------------------------------------- CT-ST-085 (4)

fn assert_refused_with(errno: i32, dir: &TempDir) {
    let _guard = FaultPlan::new()
        .errno("sync.probe", errno)
        .install(dir.path())
        .unwrap();
    let err = block_on(Store::open(options(dir.path()))).unwrap_err();
    match err {
        StoreError::DurabilityUnsupported { path, errno: got } => {
            assert_eq!(got, errno);
            assert_eq!(path, dir.path());
        }
        other => panic!("errno {errno}: {other:?}"),
    }
}

/// CT-ST-085 through the real `Store::open`: ENOTSUP, ENOTTY and an arbitrary errno injected
/// into the probe `sync_all` each give `DurabilityUnsupported{path, errno}` (D-71, D-109); no
/// database, no store identity, no journal, no heartbeat row (there is no file that could hold
/// one) and the scratch file is removed. Kills M35 (the probe sync call replaced by a no-op).
#[test]
fn ct_st_085_a_failed_probe_sync_refuses_a_new_directory_and_writes_nothing() {
    for errno in [ENOTSUP, ENOTTY, EIO, 19, 1234] {
        let dir = TempDir::new().unwrap();
        assert_refused_with(errno, &dir);
        assert_eq!(
            names(dir.path()),
            Vec::<String>::new(),
            "errno {errno}: the directory holds nothing (no chess.db, WAL, journal, store.id, scratch file)"
        );
    }
}

#[test]
fn ct_st_085_a_failed_probe_sync_leaves_an_existing_store_byte_identical() {
    let dir = TempDir::new().unwrap();
    drop(block_on(Store::open(options(dir.path()))).unwrap());
    let db = dir.path().join("chess.db");
    // Closing the last connection checkpoints the WAL; capture the state after that.
    let db_before = fs::read(&db).unwrap();
    let id_before = fs::read(dir.path().join("store.id")).unwrap();
    let listing_before = names(dir.path());
    for errno in [ENOTSUP, ENOTTY, EIO] {
        assert_refused_with(errno, &dir);
        assert_eq!(fs::read(&db).unwrap(), db_before, "errno {errno}");
        assert_eq!(fs::read(dir.path().join("store.id")).unwrap(), id_before);
        assert_eq!(
            names(dir.path()),
            listing_before,
            "errno {errno}: no new file"
        );
    }
    // Without the plan the same directory opens.
    drop(block_on(Store::open(options(dir.path()))).unwrap());
}

#[test]
fn ct_st_085_the_probe_is_the_only_sync_that_refuses_with_a_path_and_errno() {
    // Another full-sync failure at open is not a durability refusal of the volume probe:
    // it is `Unavailable`.
    let dir = TempDir::new().unwrap();
    let _g = FaultPlan::new()
        .errno("sync.store_id_dir", EIO)
        .install(dir.path())
        .unwrap();
    let err = block_on(Store::open(options(dir.path()))).unwrap_err();
    assert!(matches!(err, StoreError::Unavailable { .. }), "{err:?}");
}

// ------------------------------------------------- the survivors M16 M17 M22 M30 M31

/// M16: the directory sync after the backup rename is gone.
#[test]
fn m16_a_failed_directory_sync_after_the_backup_rename_stops_the_upgrade() {
    let dir = fixture_v1();
    let guard = FaultPlan::new()
        .errno("sync.backup_rename_dir", EIO)
        .install(dir.path())
        .unwrap();
    let err = open_v2(dir.path()).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::Io(_)
            }
        ),
        "{err:?}"
    );
    assert_eq!(guard.fired("sync.backup_rename_dir"), 1);
    assert_eq!(
        user_version(&dir.path().join("chess.db")),
        1,
        "upgrade not started"
    );
    assert_eq!(user_version(&dir.path().join("chess.db.bak-v1")), 1);
    drop(guard);
    assert_eq!(open_v2(dir.path()).unwrap().report.upgraded_from, Some(1));
}

/// M17: the sync of the backup copy's file is gone (also the directory sync of the copy).
#[test]
fn m17_a_failed_sync_of_the_backup_copy_keeps_the_old_copy_and_removes_the_temp() {
    for point in ["sync.backup_copy_file", "sync.backup_copy_dir"] {
        let dir = fixture_v1();
        let backup = dir.path().join("chess.db.bak-v1");
        fs::write(&backup, b"last good copy").unwrap();
        let guard = FaultPlan::new()
            .errno(point, EIO)
            .install(dir.path())
            .unwrap();
        let err = open_v2(dir.path()).unwrap_err();
        assert!(
            matches!(
                err,
                StoreError::BackupFailed {
                    cause: BackupCause::Io(_)
                }
            ),
            "{point}: {err:?}"
        );
        assert_eq!(guard.fired(point), 1, "{point}");
        assert_eq!(fs::read(&backup).unwrap(), b"last good copy", "{point}");
        assert!(
            !names(dir.path())
                .iter()
                .any(|n| n.starts_with("chess.db.tmp-bak")),
            "{point}"
        );
        assert_eq!(user_version(&dir.path().join("chess.db")), 1, "{point}");
    }
}

/// M22: the result of `verify_copy` is ignored in the backup path.
#[test]
fn m22_a_copy_that_fails_verification_keeps_the_old_copy_and_the_upgrade_does_not_start() {
    let dir = fixture_v1();
    let backup = dir.path().join("chess.db.bak-v1");
    fs::write(&backup, b"last good copy").unwrap();
    let guard = FaultPlan::new()
        .errno("backup.verify", EIO)
        .install(dir.path())
        .unwrap();
    let err = open_v2(dir.path()).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::VerifyFailed
            }
        ),
        "{err:?}"
    );
    assert_eq!(guard.fired("backup.verify"), 1);
    assert_eq!(fs::read(&backup).unwrap(), b"last good copy");
    assert!(
        !names(dir.path())
            .iter()
            .any(|n| n.starts_with("chess.db.tmp-bak"))
    );
    assert_eq!(user_version(&dir.path().join("chess.db")), 1);
    drop(guard);
    assert_eq!(
        open_v2(dir.path()).unwrap().report.schema_version,
        2,
        "the retry succeeds"
    );
}

/// CT-ST-090 error half: a failure at `backup.before_rename` is handled like a failed rename.
#[test]
fn ct_st_090_an_error_before_the_rename_keeps_the_old_copy_and_removes_the_temp() {
    let dir = fixture_v1();
    let backup = dir.path().join("chess.db.bak-v1");
    fs::write(&backup, b"last good copy").unwrap();
    let _g = FaultPlan::new()
        .fail("backup.before_rename", io::ErrorKind::PermissionDenied)
        .install(dir.path())
        .unwrap();
    let err = open_v2(dir.path()).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::Io(io::ErrorKind::PermissionDenied)
            }
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(&backup).unwrap(), b"last good copy");
    assert!(
        !names(dir.path())
            .iter()
            .any(|n| n.starts_with("chess.db.tmp-bak"))
    );
}

/// M30: the file sync of `store.id` is gone. A failed sync leaves no `store.id` and no temp.
#[test]
fn m30_a_failed_sync_of_store_id_fails_the_open_and_leaves_no_identity_file() {
    let dir = TempDir::new().unwrap();
    let guard = FaultPlan::new()
        .errno("sync.store_id_file", EIO)
        .install(dir.path())
        .unwrap();
    let err = block_on(Store::open(options(dir.path()))).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::Unavailable {
                cause: UnavailableCause::Io(_)
            }
        ),
        "{err:?}"
    );
    assert_eq!(guard.fired("sync.store_id_file"), 1);
    let listing = names(dir.path());
    assert!(!listing.contains(&"store.id".to_owned()), "{listing:?}");
    assert!(
        !listing.iter().any(|n| n.starts_with(".store.id.tmp")),
        "{listing:?}"
    );
    drop(guard);
    let retry = block_on(Store::open(options(dir.path()))).unwrap();
    assert!(retry.report.first_launch);
}

/// M31: the directory sync after `store.id` was linked is gone.
#[test]
fn m31_a_failed_directory_sync_after_store_id_fails_the_open() {
    let dir = TempDir::new().unwrap();
    let guard = FaultPlan::new()
        .errno("sync.store_id_dir", EIO)
        .install(dir.path())
        .unwrap();
    let err = block_on(Store::open(options(dir.path()))).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::Unavailable {
                cause: UnavailableCause::Io(_)
            }
        ),
        "{err:?}"
    );
    assert_eq!(guard.fired("sync.store_id_dir"), 1);
    drop(guard);
    // The identity written before the failed sync stands; the retry is still a first launch.
    let id = DataDir::new(dir.path()).read_store_id().unwrap().unwrap();
    let retry = block_on(Store::open(options(dir.path()))).unwrap();
    assert!(retry.report.first_launch);
    assert_eq!(
        DataDir::new(dir.path())
            .read_store_id()
            .unwrap()
            .unwrap()
            .store_uuid,
        id.store_uuid
    );
}

/// FIND-D5-06 (fix C-107-2): plan roots and failpoint contexts are compared as canonical paths,
/// so a plan installed through a symlink (macOS `/var` versus `/private/var`) fires for the real
/// path and the other way round. Before the fix this was a silent no-op.
#[test]
fn a_plan_and_a_context_match_across_a_symlinked_directory() {
    let base = TempDir::new().unwrap();
    let real = base.path().join("real");
    fs::create_dir(&real).unwrap();
    let link = base.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let name = "backup.verify";

    // Plan through the link, context under the real directory (a context that does not exist
    // yet, as a journal file about to be created).
    let guard = FaultPlan::new().errno(name, EIO).install(&link).unwrap();
    let ctx_real = real.join("sub").join("not-created-yet.db");
    assert_eq!(
        fault::trigger(name, &ctx_real).unwrap_err().raw_os_error(),
        Some(EIO)
    );
    assert_eq!((guard.hits(name), guard.fired(name)), (1, 1));
    drop(guard);

    // Plan on the real directory, context through the link.
    let guard = FaultPlan::new().errno(name, EIO).install(&real).unwrap();
    fs::write(real.join("chess.db"), b"x").unwrap();
    assert!(fault::trigger(name, &link.join("chess.db")).is_err());
    assert_eq!(guard.fired(name), 1);
    // A context outside the plan's directory is still untouched (no over-match through `..`).
    let other = TempDir::new().unwrap();
    fault::trigger(name, &other.path().join("chess.db")).unwrap();
    assert_eq!(guard.hits(name), 1, "the outside context did not count");
    // `..` inside a context does not escape the root by name tricks.
    fault::trigger(name, &real.join("..").join("elsewhere.db")).unwrap();
    assert_eq!(guard.hits(name), 1);
}
