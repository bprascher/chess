//! DD-ST-004 `DataDir`, DD-ST-005 `Opener` (part A) and DD-ST-006 `Migrator`, on real
//! temporary directories and database files: first launch layout and modes, missing store,
//! corrupt file, schema too new, fixtures per schema version, the pre-upgrade backup and its
//! failure (CT-ST-010, 012, 013, 016, 017, 018, 019, 021; CT-ST-004 and 005 in-process).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use chess_domain::{Matchup, Mode};
use chess_store::{
    BackupCause, DataDir, MIGRATIONS, Migration, Migrator, Store, StoreError, StoreId, StoreOptions,
};
use common::{block_on, computer, human, new_game, open_in, options, pid, raw, try_open};
use rusqlite::{Connection, OpenFlags};
use tempfile::TempDir;

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn listing(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            out.push(path.strip_prefix(base).unwrap().display().to_string());
            if path.is_dir() {
                walk(base, &path, out);
            }
        }
    }
    walk(dir, dir, &mut out);
    out
}

fn user_version(path: &Path) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

// ---------------------------------------------------------------- DataDir

#[test]
fn data_dir_resolution_follows_flag_then_env_then_default() {
    let flag = PathBuf::from("/flag");
    let env = std::ffi::OsString::from("/env");
    let home = PathBuf::from("/home/u");
    let resolve = |f: Option<&Path>, e: Option<&std::ffi::OsStr>, h: Option<&Path>| {
        DataDir::resolve(f, e, h).map(|d| d.root().to_path_buf())
    };
    assert_eq!(
        resolve(Some(&flag), Some(env.as_os_str()), Some(&home)),
        Some(flag.clone())
    );
    assert_eq!(
        resolve(None, Some(env.as_os_str()), Some(&home)),
        Some(PathBuf::from("/env"))
    );
    assert_eq!(
        resolve(None, None, Some(&home)),
        Some(PathBuf::from("/home/u/Library/Application Support/Chess"))
    );
    assert_eq!(resolve(None, None, None), None);
    // An empty flag or variable counts as unset.
    assert_eq!(
        resolve(
            Some(Path::new("")),
            Some(std::ffi::OsStr::new("")),
            Some(&home)
        ),
        Some(PathBuf::from("/home/u/Library/Application Support/Chess"))
    );
    assert_eq!(resolve(None, None, Some(Path::new(""))), None);
}

// ---------------------------------------------------------------- first launch

#[test]
fn ct_st_010_first_launch_creates_the_layout_with_modes_0700_and_0600() {
    let parent = TempDir::new().unwrap();
    let root = parent.path().join("nested/Chess"); // does not exist yet
    let opened = try_open(&root).unwrap();
    let report = &opened.report;
    assert!(report.first_launch);
    assert_eq!(report.upgraded_from, None);
    assert_eq!(report.schema_version, 1);
    assert!(!report.loose_permissions);
    assert_eq!(mode_of(&root), 0o700);
    for dir in ["journal", "journal/done", "backups"] {
        assert!(root.join(dir).is_dir(), "{dir}");
        assert_eq!(mode_of(&root.join(dir)), 0o700, "{dir}");
    }
    // A write makes SQLite create the WAL and shared-memory files; all are 0600.
    block_on(opened.store.register_player(human(1, "Ann"))).unwrap();
    for file in ["chess.db", "chess.db-wal", "chess.db-shm", "store.id"] {
        assert!(root.join(file).is_file(), "{file}");
        assert_eq!(mode_of(&root.join(file)), 0o600, "{file}");
    }
    assert_eq!(user_version(&root.join("chess.db")), 1);
    // The probe's scratch file is gone (D-71, SDD 3.9.1).
    assert!(
        !listing(&root)
            .iter()
            .any(|n| n.contains("durability-probe")),
        "{:?}",
        listing(&root)
    );
}

#[test]
fn ct_st_021_store_id_holds_a_uuid_and_the_creation_time_and_survives_reopen() {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    let data = DataDir::new(dir.path());
    let id: StoreId = data.read_store_id().unwrap().unwrap();
    assert_eq!(id.created_at, 1_000, "the injected clock");
    assert!(!id.store_uuid.is_nil());
    let again = open_in(dir.path());
    assert!(!again.report.first_launch);
    assert_eq!(data.read_store_id().unwrap().unwrap(), id);
    // Two stores get two identities.
    let other = TempDir::new().unwrap();
    drop(open_in(other.path()));
    let other_id = DataDir::new(other.path()).read_store_id().unwrap().unwrap();
    assert_ne!(other_id.store_uuid, id.store_uuid);
}

#[test]
fn a_garbled_store_id_file_is_corrupt_and_absent_is_none() {
    let dir = TempDir::new().unwrap();
    let data = DataDir::new(dir.path());
    assert_eq!(data.read_store_id().unwrap(), None);
    fs::write(data.store_id_path(), b"not json").unwrap();
    assert!(matches!(data.read_store_id(), Err(StoreError::Corrupt)));
    assert!(
        data.write_store_id(&StoreId {
            store_uuid: uuid::Uuid::nil(),
            created_at: 0
        })
        .is_err(),
        "never overwrites"
    );
}

#[test]
fn an_existing_directory_with_loose_permissions_is_reported_and_never_changed() {
    let dir = TempDir::new().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let opened = try_open(dir.path()).unwrap();
    assert!(opened.report.loose_permissions);
    assert_eq!(
        mode_of(dir.path()),
        0o755,
        "the user's directory is not chmoded"
    );
}

#[test]
fn a_path_that_is_a_file_is_unavailable_not_a_panic() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("afile");
    fs::write(&file, b"x").unwrap();
    assert!(matches!(
        try_open(&file),
        Err(StoreError::Unavailable { .. })
    ));
}

#[test]
fn an_interrupted_first_creation_is_a_first_launch_again() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("chess.db"), b"").unwrap(); // created, never migrated
    let opened = try_open(dir.path()).unwrap();
    assert!(opened.report.first_launch);
    assert_eq!(opened.report.schema_version, 1);
    assert!(DataDir::new(dir.path()).read_store_id().unwrap().is_some());
}

// ---------------------------------------------------------------- missing store

#[test]
fn ct_st_016_any_trace_without_chess_db_is_store_missing_and_nothing_is_written() {
    type Setup = fn(&Path);
    let setups: [(&str, Setup); 4] = [
        ("store.id", |p| {
            fs::write(p.join("store.id"), b"{}").unwrap()
        }),
        ("backup", |p| {
            fs::create_dir_all(p.join("backups")).unwrap();
            fs::write(p.join("backups/chess-2026-11-01.db"), b"x").unwrap();
        }),
        ("journal", |p| {
            fs::create_dir_all(p.join("journal")).unwrap();
            fs::write(
                p.join("journal/00000000-0000-0000-0000-000000000001.jnl"),
                b"x",
            )
            .unwrap();
        }),
        ("pre-upgrade copy", |p| {
            fs::write(p.join("chess.db.bak-v1"), b"x").unwrap()
        }),
    ];
    for (name, setup) in setups {
        let dir = TempDir::new().unwrap();
        setup(dir.path());
        let before = listing(dir.path());
        let err = try_open(dir.path()).unwrap_err();
        assert!(matches!(err, StoreError::Missing), "{name}: {err:?}");
        assert_eq!(listing(dir.path()), before, "{name}: files as found");
    }
}

#[test]
fn logs_config_and_empty_layout_are_not_traces_of_a_store() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("logs")).unwrap();
    fs::write(dir.path().join("logs/chess.log"), b"x").unwrap();
    fs::write(dir.path().join("config.toml"), b"x").unwrap();
    fs::create_dir_all(dir.path().join("journal/done")).unwrap();
    fs::create_dir_all(dir.path().join("backups")).unwrap();
    let opened = try_open(dir.path()).unwrap();
    assert!(opened.report.first_launch);
}

#[test]
fn a_done_journal_in_the_subdirectory_is_a_trace() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("journal/done")).unwrap();
    fs::write(dir.path().join("journal/done/g.jnl"), b"x").unwrap();
    assert!(matches!(try_open(dir.path()), Err(StoreError::Missing)));
}

// ---------------------------------------------------------------- corrupt and too new

#[test]
fn ct_st_012_a_file_that_is_not_a_database_is_corrupt_and_its_bytes_are_unchanged() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("chess.db");
    let garbage: Vec<u8> = (0..8192u32)
        .map(|i| u8::try_from(i % 251).unwrap())
        .collect();
    fs::write(&db, &garbage).unwrap();
    let err = try_open(dir.path()).unwrap_err();
    assert!(matches!(err, StoreError::Corrupt), "{err:?}");
    assert!(fs::read(&db).unwrap() == garbage, "bytes changed");
}

#[test]
fn ct_st_012_a_damaged_page_fails_quick_check_and_its_bytes_are_unchanged() {
    let dir = TempDir::new().unwrap();
    {
        let opened = open_in(dir.path());
        for n in 1..=40u128 {
            block_on(
                opened
                    .store
                    .register_player(human(n, &format!("Player number {n}"))),
            )
            .unwrap();
        }
    }
    let db = dir.path().join("chess.db");
    let mut bytes = fs::read(&db).unwrap();
    assert!(
        bytes.len() >= 4096 * 4,
        "database has several pages: {}",
        bytes.len()
    );
    for b in &mut bytes[4096 * 2..4096 * 3] {
        *b = 0xFF; // page 3
    }
    fs::write(&db, &bytes).unwrap();
    let err = try_open(dir.path()).unwrap_err();
    assert!(matches!(err, StoreError::Corrupt), "{err:?}");
    assert!(fs::read(&db).unwrap() == bytes, "bytes changed");
}

#[test]
fn a_database_of_another_program_with_user_version_zero_is_corrupt_and_untouched() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("chess.db");
    Connection::open(&db)
        .unwrap()
        .execute_batch("CREATE TABLE other (x);")
        .unwrap();
    let before = fs::read(&db).unwrap();
    assert!(matches!(try_open(dir.path()), Err(StoreError::Corrupt)));
    assert!(fs::read(&db).unwrap() == before, "bytes changed");
}

#[test]
fn ct_st_013_a_newer_user_version_is_schema_too_new_and_nothing_is_written() {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    raw(dir.path())
        .pragma_update(None, "user_version", 99)
        .unwrap();
    let db = dir.path().join("chess.db");
    let before = fs::read(&db).unwrap();
    let files_before = listing(dir.path());
    let err = try_open(dir.path()).unwrap_err();
    assert!(
        matches!(err, StoreError::SchemaTooNew { db: 99, app: 1 }),
        "{err:?}"
    );
    assert!(fs::read(&db).unwrap() == before, "bytes changed");
    let after = listing(dir.path());
    let new_files: Vec<_> = after.iter().filter(|n| !files_before.contains(n)).collect();
    // Finding F-A09: a read-only connection on a WAL database creates empty -wal and -shm
    // sidecar files. chess.db itself is byte-identical and no backup or other file appears.
    assert!(
        new_files
            .iter()
            .all(|n| *n == "chess.db-wal" || *n == "chess.db-shm"),
        "new files: {new_files:?}"
    );
}

#[test]
fn ct_st_013_schema_too_new_is_decided_before_the_wal_pragma_can_touch_the_file() {
    // A database not in WAL mode would be switched to WAL by the write connection's pragma, so
    // this fails if the version check runs after that connection opens.
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    {
        let conn = raw(dir.path());
        let mode: String = conn
            .pragma_update_and_check(None, "journal_mode", "DELETE", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "delete");
        conn.pragma_update(None, "user_version", 99).unwrap();
    }
    let db = dir.path().join("chess.db");
    let before = fs::read(&db).unwrap();
    assert!(matches!(
        try_open(dir.path()),
        Err(StoreError::SchemaTooNew { db: 99, app: 1 })
    ));
    assert!(fs::read(&db).unwrap() == before, "bytes changed");
}

// ---------------------------------------------------------------- migrations

fn v2() -> Migrator {
    Migrator::with(&[
        MIGRATIONS[0],
        Migration {
            version: 2,
            sql: "ALTER TABLE player ADD COLUMN nick TEXT; \
                  CREATE TABLE note (id INTEGER PRIMARY KEY, body TEXT NOT NULL);",
        },
    ])
    .unwrap()
}

/// A database built from the frozen fixture of schema version 1.
fn fixture_v1() -> TempDir {
    let dir = TempDir::new().unwrap();
    let conn = Connection::open(dir.path().join("chess.db")).unwrap();
    conn.execute_batch(include_str!("fixtures/schema-v1.sql"))
        .unwrap();
    // The fixture is a store that exists: it has its identity file.
    DataDir::new(dir.path())
        .write_store_id(&StoreId {
            store_uuid: uuid::Uuid::from_u128(5),
            created_at: 1,
        })
        .unwrap();
    dir
}

fn open_with(dir: &Path, migrator: Migrator) -> Result<chess_store::OpenedStore, StoreError> {
    block_on(Store::open(options(dir).with_migrator(migrator)))
}

#[test]
fn the_v1_fixture_schema_part_is_the_migration_it_claims_to_copy() {
    let fixture = include_str!("fixtures/schema-v1.sql");
    let migration = include_str!("../migrations/0001_init.sql");
    assert!(fixture.contains(migration));
}

#[test]
fn ct_st_017_the_v1_fixture_opens_at_v1_with_no_backup_and_data_intact() {
    let dir = fixture_v1();
    let opened = try_open(dir.path()).unwrap();
    assert!(!opened.report.first_launch);
    assert_eq!(opened.report.upgraded_from, None);
    assert_eq!(opened.report.schema_version, 1);
    assert!(!dir.path().join("chess.db.bak-v1").exists());
    let ann = block_on(opened.store.find_player_by_name("Fixture Ann".into()))
        .unwrap()
        .unwrap();
    assert_eq!(ann.strict_elo, 1200);
    assert_eq!(
        block_on(
            opened
                .store
                .game_moves(chess_domain::GameId::from_uuid(uuid::Uuid::from_u128(0xe1)))
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn ct_st_017_a_synthetic_v2_upgrades_the_v1_fixture_after_a_verified_backup() {
    let dir = fixture_v1();
    let opened = open_with(dir.path(), v2()).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(opened.report.schema_version, 2);
    assert!(!opened.report.first_launch);
    let db = dir.path().join("chess.db");
    assert_eq!(user_version(&db), 2);
    let backup = dir.path().join("chess.db.bak-v1");
    assert_eq!(mode_of(&backup), 0o600);
    assert_eq!(
        user_version(&backup),
        1,
        "the backup opens at the old version"
    );
    let old = Connection::open_with_flags(&backup, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let players: i64 = old
        .query_row("SELECT count(*) FROM player", [], |r| r.get(0))
        .unwrap();
    let notes: i64 = old
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'note'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!((players, notes), (2, 0), "old data, old schema");
    let quick: String = old
        .pragma_query_value(None, "quick_check", |r| r.get(0))
        .unwrap();
    assert_eq!(quick, "ok");
    let new = raw(dir.path());
    let nick: i64 = new
        .query_row(
            "SELECT count(*) FROM pragma_table_info('player') WHERE name = 'nick'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(nick, 1);
    let kept: i64 = new
        .query_row("SELECT count(*) FROM player", [], |r| r.get(0))
        .unwrap();
    assert_eq!(kept, 2);
    // The store works at v2, and a v1 build now refuses it (PLAT-5).
    block_on(opened.store.register_player(human(9, "New"))).unwrap();
    drop(opened);
    let err = try_open(dir.path()).unwrap_err();
    assert!(
        matches!(err, StoreError::SchemaTooNew { db: 2, app: 1 }),
        "{err:?}"
    );
}

fn backup_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.contains("bak-v"))
        .collect();
    names.sort();
    names
}

fn note_tables(dir: &Path) -> i64 {
    raw(dir)
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'note'",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn ct_st_019_an_existing_backup_is_replaced_by_a_verified_copy_and_the_upgrade_proceeds() {
    // D-108 (replaces the part A "never overwritten" behaviour, F-A02).
    let dir = fixture_v1();
    let backup = dir.path().join("chess.db.bak-v1");
    fs::write(&backup, b"an earlier backup").unwrap();
    let opened = open_with(dir.path(), v2()).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(user_version(&dir.path().join("chess.db")), 2);
    assert_eq!(
        user_version(&backup),
        1,
        "the old bytes are replaced by a v1 copy"
    );
    assert_eq!(mode_of(&backup), 0o600);
    let old = Connection::open_with_flags(&backup, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let quick: String = old
        .pragma_query_value(None, "quick_check", |r| r.get(0))
        .unwrap();
    assert_eq!(quick, "ok");
    assert_eq!(
        backup_names(dir.path()),
        vec!["chess.db.bak-v1"],
        "no temp left"
    );
}

#[test]
fn ct_st_090_a_stale_temp_file_is_removed_and_never_counted_as_a_backup() {
    let dir = fixture_v1();
    // The part A name and a temp of a process that is gone (NF-1): both are stale.
    let temp = dir.path().join("chess.db.tmp-bak-v1");
    fs::write(&temp, b"half written").unwrap();
    let gone = dir
        .path()
        .join(format!("chess.db.tmp-bak-v1-{}", dead_pid()));
    fs::write(&gone, b"half written").unwrap();
    // Before the upgrade the only thing that looks like a backup is not one.
    assert!(!"chess.db.tmp-bak-v1".starts_with("chess.db.bak-v"));
    let opened = open_with(dir.path(), v2()).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert!(
        !temp.exists() && !gone.exists(),
        "stale temps removed first"
    );
    assert_eq!(backup_names(dir.path()), vec!["chess.db.bak-v1"]);
}

#[test]
fn ct_st_090_a_failed_attempt_leaves_the_previous_copy_byte_identical() {
    let dir = fixture_v1();
    let backup = dir.path().join("chess.db.bak-v1");
    fs::write(&backup, b"last good copy").unwrap();
    // The temp name is occupied by a non-empty directory: the stale file cannot be removed.
    let temp = dir
        .path()
        .join(format!("chess.db.tmp-bak-v1-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    fs::write(temp.join("x"), b"x").unwrap();
    let err = open_with(dir.path(), v2()).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::Io(_)
            }
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(&backup).unwrap(), b"last good copy");
    assert_eq!(
        user_version(&dir.path().join("chess.db")),
        1,
        "upgrade did not start"
    );
    assert_eq!(note_tables(dir.path()), 0);
    // The retry succeeds once the obstacle is gone.
    fs::remove_dir_all(&temp).unwrap();
    let opened = open_with(dir.path(), v2()).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(user_version(&backup), 1);
}

#[test]
fn ct_st_090_an_unwritable_directory_keeps_the_previous_copy_byte_identical() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("a.db");
    let mut conn = Connection::open(&db).unwrap();
    Migrator::embedded()
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    let vault = dir.path().join("vault");
    fs::create_dir(&vault).unwrap();
    let backup = vault.join("chess.db.bak-v1");
    fs::write(&backup, b"last good copy").unwrap();
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o500)).unwrap();
    let err = v2()
        .upgrade(&mut conn, |_| backup.clone(), &|_| Ok(()))
        .unwrap_err();
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::Io(std::io::ErrorKind::PermissionDenied)
            }
        ),
        "{err:?}"
    );
    assert_eq!(fs::read(&backup).unwrap(), b"last good copy");
    assert_eq!(Migrator::read_version(&conn).unwrap(), 1);
}

#[test]
fn ct_st_090_the_retry_after_a_rolled_back_migration_succeeds_without_a_manual_step() {
    // The F-A02 failure: migration fails after the backup was written; the next open used to
    // get BackupFailed{TargetExists}.
    let dir = fixture_v1();
    let bad = Migrator::with(&[
        MIGRATIONS[0],
        Migration {
            version: 2,
            sql: "CREATE TABLE half (x); CREATE TABLE half (x);",
        },
    ])
    .unwrap();
    let err = open_with(dir.path(), bad).unwrap_err();
    assert!(matches!(err, StoreError::Internal), "{err:?}");
    let backup = dir.path().join("chess.db.bak-v1");
    assert_eq!(user_version(&backup), 1, "the copy of the failed attempt");
    assert_eq!(user_version(&dir.path().join("chess.db")), 1);
    let opened = open_with(dir.path(), v2()).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(user_version(&dir.path().join("chess.db")), 2);
    assert_eq!(backup_names(dir.path()), vec!["chess.db.bak-v1"]);
}

#[test]
fn ct_st_090_a_restored_pre_upgrade_copy_can_be_upgraded_again() {
    let dir = fixture_v1();
    drop(open_with(dir.path(), v2()).unwrap());
    let backup = dir.path().join("chess.db.bak-v1");
    // Restore: the v1 copy becomes the store (no WAL files of the old store remain).
    let db = dir.path().join("chess.db");
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(dir.path().join(format!("chess.db{suffix}")));
    }
    fs::copy(&backup, &db).unwrap();
    assert_eq!(user_version(&db), 1);
    let opened = open_with(dir.path(), v2()).unwrap();
    assert_eq!(opened.report.upgraded_from, Some(1));
    assert_eq!(user_version(&db), 2);
    assert_eq!(user_version(&backup), 1);
    assert_eq!(backup_names(dir.path()), vec!["chess.db.bak-v1"]);
}

#[test]
fn ct_st_019_an_unwritable_backup_location_is_backup_failed_and_a_retry_succeeds() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("a.db");
    let mut conn = Connection::open(&db).unwrap();
    Migrator::embedded()
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    let migrator = v2();
    let missing_dir = dir.path().join("no/such/dir/backup");
    let err = migrator
        .upgrade(&mut conn, |_| missing_dir.clone(), &|_| Ok(()))
        .unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::Io(std::io::ErrorKind::NotFound)
            }
        ),
        "{err:?}"
    );
    assert_eq!(
        chess_store::Migrator::read_version(&conn).unwrap(),
        1,
        "store unchanged"
    );
    // A read-only directory fails the same way.
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).unwrap();
    let err = migrator
        .upgrade(&mut conn, |_| locked.join("b"), &|_| Ok(()))
        .unwrap_err();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(
            err,
            StoreError::BackupFailed {
                cause: BackupCause::Io(std::io::ErrorKind::PermissionDenied)
            }
        ),
        "{err:?}"
    );
    assert_eq!(Migrator::read_version(&conn).unwrap(), 1);
    // Retry with a good place.
    let good = dir.path().join("good.bak");
    let outcome = migrator
        .upgrade(&mut conn, |_| good.clone(), &|_| Ok(()))
        .unwrap();
    assert_eq!((outcome.from, outcome.to, outcome.backed_up), (1, 2, true));
    assert_eq!(user_version(&good), 1);
}

#[test]
fn a_failed_migration_rolls_back_and_leaves_the_backup() {
    let dir = TempDir::new().unwrap();
    let mut conn = Connection::open(dir.path().join("a.db")).unwrap();
    Migrator::embedded()
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    let bad = Migrator::with(&[
        MIGRATIONS[0],
        Migration {
            version: 2,
            sql: "CREATE TABLE half (x); CREATE TABLE half (x);",
        },
    ])
    .unwrap();
    let backup = dir.path().join("bak");
    let err = bad
        .upgrade(&mut conn, |_| backup.clone(), &|_| Ok(()))
        .unwrap_err();
    assert!(matches!(err, StoreError::Internal), "{err:?}");
    assert_eq!(Migrator::read_version(&conn).unwrap(), 1);
    let half: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'half'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(half, 0, "DDL of the failed migration is rolled back");
    assert_eq!(user_version(&backup), 1);
}

#[test]
fn the_exclusive_guard_can_stop_an_upgrade_and_the_store_stays_at_its_version() {
    let dir = TempDir::new().unwrap();
    let mut conn = Connection::open(dir.path().join("a.db")).unwrap();
    Migrator::embedded()
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    let backup = dir.path().join("bak");
    let err = v2()
        .upgrade(&mut conn, |_| backup.clone(), &|_| Err(StoreError::Busy))
        .unwrap_err();
    assert!(matches!(err, StoreError::Busy));
    assert_eq!(Migrator::read_version(&conn).unwrap(), 1);
}

#[test]
fn a_version_change_by_another_process_before_the_exclusive_lock_is_busy() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.db");
    let mut conn = Connection::open(&path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    Migrator::embedded()
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    let other = Connection::open(&path).unwrap();
    let backup = dir.path().join("bak");
    let err = v2()
        .upgrade(
            &mut conn,
            |_| {
                other.pragma_update(None, "user_version", 2).unwrap(); // "another process migrated"
                backup.clone()
            },
            &|_| Ok(()),
        )
        .unwrap_err();
    assert!(matches!(err, StoreError::Busy), "{err:?}");
}

#[test]
fn a_newer_database_is_refused_by_the_migrator_and_equal_is_a_no_op() {
    let dir = TempDir::new().unwrap();
    let mut conn = Connection::open(dir.path().join("a.db")).unwrap();
    let m = Migrator::embedded();
    let first = m
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    assert_eq!(
        (first.from, first.to, first.backed_up),
        (0, 1, false),
        "a new database needs no backup"
    );
    let again = m
        .upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(()))
        .unwrap();
    assert_eq!((again.from, again.to, again.backed_up), (1, 1, false));
    conn.pragma_update(None, "user_version", 9).unwrap();
    assert!(matches!(
        m.upgrade(&mut conn, |_| PathBuf::new(), &|_| Ok(())),
        Err(StoreError::SchemaTooNew { db: 9, app: 1 })
    ));
}

#[test]
fn migrator_with_rejects_gaps_and_an_empty_list() {
    assert!(Migrator::with(&[]).is_err());
    assert!(
        Migrator::with(&[Migration {
            version: 2,
            sql: ""
        }])
        .is_err()
    );
    assert!(
        Migrator::with(&[
            MIGRATIONS[0],
            Migration {
                version: 3,
                sql: ""
            }
        ])
        .is_err()
    );
    assert_eq!(Migrator::embedded().latest(), 1);
}

// ---------------------------------------------------------------- concurrency (in-process)

#[test]
fn ct_st_004_in_process_two_stores_on_one_directory_write_without_loss() {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    let path = dir.path().to_path_buf();
    let workers: Vec<_> = (0..2u128)
        .map(|w| {
            let path = path.clone();
            std::thread::spawn(move || {
                let opened = try_open(&path).unwrap();
                for i in 0..25u128 {
                    let n = 100 + w * 100 + i;
                    block_on(opened.store.register_player(human(n, &format!("w{w}-{i}")))).unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let reader = open_in(dir.path());
    assert_eq!(
        block_on(reader.store.list_players(None, true))
            .unwrap()
            .len(),
        50
    );
}

#[test]
fn ct_st_005_readers_proceed_during_a_long_write_and_see_only_committed_rows() {
    let dir = TempDir::new().unwrap();
    let store = open_in(dir.path()).store;
    block_on(store.register_player(human(1, "Before"))).unwrap();
    let mut other = raw(dir.path());
    let tx = other
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    tx.execute(
        "INSERT INTO player (uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
         VALUES ('00000000-0000-0000-0000-000000000002', 'human', 'During', 'Novice', 1, 1, 1)",
        [],
    )
    .unwrap();
    // The write lock is held; a read does not wait for it and does not see the open transaction.
    let names: Vec<String> = block_on(store.list_players(None, true))
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["Before"]);
    tx.commit().unwrap();
    let names: Vec<String> = block_on(store.list_players(None, true))
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["Before", "During"]);
}

#[test]
fn data_survives_closing_and_reopening_the_store() {
    let dir = TempDir::new().unwrap();
    {
        let s = open_in(dir.path()).store;
        block_on(s.register_player(human(1, "Ann"))).unwrap();
        block_on(s.insert_computers(vec![computer(10, "Bot", 1, 3, 2)])).unwrap();
        block_on(s.start_game(new_game(
            1,
            pid(1),
            pid(10),
            Matchup::HumanComputer,
            Mode::Strict,
        )))
        .unwrap();
        block_on(s.set_app_setting("last_active_player".into(), "x".into())).unwrap();
    }
    let s = open_in(dir.path()).store;
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().name, "Ann");
    assert_eq!(block_on(s.roster_indexes()).unwrap(), [3]);
    assert_eq!(block_on(s.list_unfinished(pid(1))).unwrap().len(), 1);
    assert_eq!(
        block_on(s.get_app_setting("last_active_player".into()))
            .unwrap()
            .as_deref(),
        Some("x")
    );
    let _ = StoreOptions::new(DataDir::new(dir.path()));
}

#[test]
fn ensure_creates_the_layout_and_is_repeatable() {
    let parent = TempDir::new().unwrap();
    let data = DataDir::new(parent.path().join("d"));
    let first = data.ensure().unwrap();
    assert!(first.created && !first.loose_permissions);
    let second = data.ensure().unwrap();
    assert!(!second.created);
    for dir in [
        data.journal_dir(),
        data.journal_done_dir(),
        data.backups_dir(),
    ] {
        assert_eq!(mode_of(&dir), 0o700);
    }
    assert_eq!(
        data.pre_upgrade_backup_path(3),
        data.root().join("chess.db.bak-v3")
    );
    assert_eq!(data.db_path(), data.root().join("chess.db"));
}

#[test]
fn a_flag_wins_over_the_process_environment() {
    let flag = PathBuf::from("/some/flag/dir");
    assert_eq!(
        DataDir::resolve_from_env(Some(&flag)).unwrap().root(),
        flag.as_path()
    );
}

#[test]
fn the_default_clock_is_the_system_clock_in_milliseconds() {
    let dir = TempDir::new().unwrap();
    let opened = block_on(Store::open(StoreOptions::new(DataDir::new(dir.path())))).unwrap();
    let ann = block_on(opened.store.register_player(human(1, "Ann"))).unwrap();
    // After 2020-01-01 and before 2100-01-01, in milliseconds.
    assert!(
        (1_577_836_800_000..4_102_444_800_000).contains(&ann.created_at.0),
        "{:?}",
        ann.created_at
    );
    assert!(opened.store.data_dir().root() == dir.path());
}

#[test]
fn the_pool_size_option_is_honoured_and_at_least_one() {
    let dir = TempDir::new().unwrap();
    let mut opts = options(dir.path());
    opts.read_connections = 0; // clamped to one connection: reads must still work
    let opened = block_on(Store::open(opts)).unwrap();
    block_on(opened.store.register_player(human(1, "Ann"))).unwrap();
    assert_eq!(
        block_on(opened.store.list_players(None, true))
            .unwrap()
            .len(),
        1
    );
}

/// A pid that existed and is gone.
fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}
