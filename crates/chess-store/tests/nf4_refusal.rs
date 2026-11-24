//! NF-4 (Brian Prascher, 2026-11-22; store-rework-specs section 2): an empty or schemaless
//! `chess.db` beside traces of an earlier store's data is **not** a first launch. `Store::open`
//! refuses with `Missing` and writes nothing. `create_fresh` (the way out) is part C and is not
//! built here: only the refusal is tested. FIND-002 (empty database plus `store.id` only is a
//! first launch) and FIND-007 (a store with a schema and no `store.id` opens, `store.id` is not
//! recreated) stay as they were.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chess_store::{DataDir, StoreError};
use common::{block_on, open_in, try_open};
use rusqlite::Connection;
use tempfile::TempDir;

/// Bytes of a valid `store.id`, made by a first launch in another directory.
fn valid_store_id() -> Vec<u8> {
    let src = TempDir::new().unwrap();
    drop(open_in(src.path()));
    fs::read(src.path().join("store.id")).unwrap()
}

/// An empty (zero-length) `chess.db`, a valid `store.id` and the empty layout.
fn empty_store_dir(dir: &Path) {
    fs::write(dir.join("chess.db"), b"").unwrap();
    fs::write(dir.join("store.id"), valid_store_id()).unwrap();
    DataDir::new(dir).ensure().unwrap();
}

fn write(dir: &Path, rel: &str, content: &[u8]) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Length and FNV-1a of the bytes: a readable failure message instead of a byte dump.
fn digest(bytes: &[u8]) -> (usize, u64) {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    (bytes.len(), hash)
}

/// Every regular file with its bytes and every directory, ignoring the zero-length `-wal` and
/// `-shm` that the read-only probe of a WAL database may leave (F-A09).
fn snapshot(dir: &Path) -> BTreeMap<String, Option<(usize, u64)>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            if p.is_dir() {
                out.insert(rel, None);
                stack.push(p);
                continue;
            }
            let bytes = fs::read(&p).unwrap();
            // `-shm` is the WAL index SQLite builds on any open of a WAL database, empty or not.
            if rel.ends_with("-shm") || (bytes.is_empty() && rel.ends_with("-wal")) {
                continue;
            }
            out.insert(rel, Some(digest(&bytes)));
        }
    }
    out
}

fn assert_missing_and_untouched(dir: &Path, label: &str) {
    let before = snapshot(dir);
    let err = try_open(dir)
        .err()
        .unwrap_or_else(|| panic!("{label}: opened"));
    assert!(matches!(err, StoreError::Missing), "{label}: {err:?}");
    assert_eq!(
        before,
        snapshot(dir),
        "{label}: nothing written, moved or removed"
    );
    // NF4-14: stable and repeatable.
    let again = try_open(dir)
        .err()
        .unwrap_or_else(|| panic!("{label}: opened twice"));
    assert!(matches!(again, StoreError::Missing), "{label}: {again:?}");
    assert_eq!(
        before,
        snapshot(dir),
        "{label}: second refusal writes nothing"
    );
}

const JNL: &str = "journal/11111111-1111-1111-1111-111111111111.jnl";

#[test]
fn nf4_1_an_empty_directory_is_a_first_launch() {
    let dir = TempDir::new().unwrap();
    assert!(try_open(dir.path()).unwrap().report.first_launch);
}

#[test]
fn nf4_2_a_zero_length_chess_db_alone_is_a_first_launch() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("chess.db"), b"").unwrap();
    assert!(try_open(dir.path()).unwrap().report.first_launch);
    assert!(dir.path().join("store.id").exists());
}

/// FIND-002 regression: empty `chess.db`, valid `store.id`, empty layout: the same identity is
/// kept and this is the first launch.
#[test]
fn nf4_3_empty_database_with_store_id_and_an_empty_layout_is_a_first_launch() {
    let dir = TempDir::new().unwrap();
    empty_store_dir(dir.path());
    let id_before = fs::read(dir.path().join("store.id")).unwrap();
    let opened = try_open(dir.path()).unwrap();
    assert!(opened.report.first_launch);
    drop(opened);
    assert_eq!(fs::read(dir.path().join("store.id")).unwrap(), id_before);
}

#[test]
fn nf4_4_to_8_empty_database_beside_each_data_trace_is_refused_and_nothing_is_written() {
    let traces: [(&str, &str, &[u8]); 8] = [
        ("NF4-4 journal file", JNL, b"x"),
        (
            "NF4-5 journal/done",
            "journal/done/22222222-2222-2222-2222-222222222222.jnl",
            b"x",
        ),
        ("NF4-5 journal/damaged", "journal/damaged/a.1.jnl", b"x"),
        ("NF4-6 backups", "backups/chess-2026-11-01.db", b"x"),
        ("NF4-7 bak-v1", "chess.db.bak-v1", b"x"),
        ("NF4-7 tmp-bak", "chess.db.tmp-bak-v1-7", b"x"),
        ("NF4-7 pre-import", "chess.db.pre-import-x", b"x"),
        ("NF4-7 restoring", "chess.db.restoring", b"x"),
    ];
    for (label, rel, content) in traces {
        let dir = TempDir::new().unwrap();
        empty_store_dir(dir.path());
        write(dir.path(), rel, content);
        let started = std::time::Instant::now();
        assert_missing_and_untouched(dir.path(), label);
        // Ruling 2026-11-23 (FIND-RW1-01): no settle wait; a refusal is immediate (two opens,
        // far below the 1 s that the removed `WAL_SETTLE` cost per call).
        assert!(
            started.elapsed() < std::time::Duration::from_millis(900),
            "{label}: the refusal is immediate, {:?}",
            started.elapsed()
        );
        // The store is not created: chess.db is still empty and holds no schema.
        assert_eq!(
            fs::metadata(dir.path().join("chess.db")).unwrap().len(),
            0,
            "{label}"
        );
    }
}

/// NF4-8: the data traces are checked before `store.id` is read, so no `store.id` is needed.
#[test]
fn nf4_8_the_traces_are_checked_before_store_id_is_read() {
    for store_id in [None, Some(b"not json".as_slice())] {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("chess.db"), b"").unwrap();
        DataDir::new(dir.path()).ensure().unwrap();
        write(dir.path(), JNL, b"x");
        if let Some(bytes) = store_id {
            fs::write(dir.path().join("store.id"), bytes).unwrap();
        }
        assert_missing_and_untouched(dir.path(), &format!("store.id {store_id:?}"));
        assert_eq!(
            dir.path().join("store.id").exists(),
            store_id.is_some(),
            "no store.id was created"
        );
    }
}

/// NF4-9: a non-zero file that SQLite reads as schemaless (`user_version` 0, no tables) is the
/// same state as a zero-length one.
#[test]
fn nf4_9_a_schemaless_non_zero_database_beside_a_backup_is_refused() {
    let dir = TempDir::new().unwrap();
    {
        let c = Connection::open(dir.path().join("chess.db")).unwrap();
        c.execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE x (a); DROP TABLE x;")
            .unwrap();
    }
    assert!(fs::metadata(dir.path().join("chess.db")).unwrap().len() > 0);
    fs::write(dir.path().join("store.id"), valid_store_id()).unwrap();
    DataDir::new(dir.path()).ensure().unwrap();
    write(dir.path(), "backups/chess-2026-11-01.db", b"x");
    assert_missing_and_untouched(dir.path(), "schemaless non-zero");
}

/// NF4-7b (ruling 2026-11-23, package row 7b): a zero-length `chess.db` beside a non-empty
/// `-wal` is refused before SQLite touches it (SQLite would delete the WAL, FIND-ST-RW-01), for
/// every WAL content and with or without `store.id`, and nothing is written or removed.
#[test]
fn nf4_7b_a_zero_length_database_beside_a_non_empty_wal_is_refused_untouched() {
    for (label, wal) in wal_forms() {
        for with_id in [false, true] {
            let dir = TempDir::new().unwrap();
            fs::write(dir.path().join("chess.db"), b"").unwrap();
            if with_id {
                fs::write(dir.path().join("store.id"), valid_store_id()).unwrap();
            }
            DataDir::new(dir.path()).ensure().unwrap();
            write(dir.path(), "chess.db-wal", &wal);
            assert_missing_and_untouched(dir.path(), &format!("{label} store.id={with_id}"));
            assert_eq!(fs::read(dir.path().join("chess.db-wal")).unwrap(), wal);
        }
    }
}

/// The `-wal` contents a creator killed during the switch to WAL leaves (package section (a),
/// the Reviewer's states): a bare 32-byte header, real committed frames of a schema that holds
/// no table, and garbage. Returns `(label, wal bytes)`.
fn wal_forms() -> Vec<(&'static str, Vec<u8>)> {
    let (_, wal) = real_wal_pair();
    vec![
        ("garbage", b"not a wal, but not empty".to_vec()),
        ("32-byte header", wal[..32].to_vec()),
        ("real frames", wal),
    ]
}

/// A WAL-mode `chess.db` that holds no table and a `-wal` that holds committed frames, copied
/// while the connection is still open (a clean close would checkpoint and delete the WAL).
fn real_wal_pair() -> (Vec<u8>, Vec<u8>) {
    let src = TempDir::new().unwrap();
    let c = Connection::open(src.path().join("chess.db")).unwrap();
    c.pragma_update(None, "journal_mode", "WAL").unwrap();
    c.execute_batch("CREATE TABLE x (a); DROP TABLE x;")
        .unwrap();
    let db = fs::read(src.path().join("chess.db")).unwrap();
    let wal = fs::read(src.path().join("chess.db-wal")).unwrap();
    assert!(
        !db.is_empty() && wal.len() > 32,
        "fixture: db and WAL have content"
    );
    (db, wal)
}

/// FIND-RW1-01, state 1 (the Reviewer's killed-creation state): a non-zero `chess.db` that
/// SQLite reads as schemaless beside a non-empty `-wal` is a creation that was cut off. It
/// completes as a first launch, with or without `store.id`.
#[test]
fn rw1_01_a_schemaless_non_zero_database_beside_a_wal_completes_as_a_first_launch() {
    let (real_db, _) = real_wal_pair();
    let rollback_db = {
        let d = TempDir::new().unwrap();
        Connection::open(d.path().join("chess.db"))
            .unwrap()
            .execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE x (a); DROP TABLE x;")
            .unwrap();
        fs::read(d.path().join("chess.db")).unwrap()
    };
    for (label, wal) in wal_forms() {
        for (db_label, db) in [
            ("wal-mode db", &real_db),
            ("rollback-mode db", &rollback_db),
        ] {
            for with_id in [false, true] {
                let dir = TempDir::new().unwrap();
                fs::write(dir.path().join("chess.db"), db).unwrap();
                fs::write(dir.path().join("chess.db-wal"), &wal).unwrap();
                if with_id {
                    fs::write(dir.path().join("store.id"), valid_store_id()).unwrap();
                }
                let case = format!("{db_label} + {label} wal, store.id={with_id}");
                let opened = try_open(dir.path()).unwrap_or_else(|e| panic!("{case}: {e:?}"));
                assert!(opened.report.first_launch, "{case}");
                block_on(opened.store.register_player(common::human(1, "Ann"))).unwrap();
                drop(opened);
                assert!(!try_open(dir.path()).unwrap().report.first_launch, "{case}");
            }
        }
    }
}

/// FIND-RW1-01, state 2: a zero-length `chess.db` beside a non-empty rollback journal and
/// `store.id` only (the creator was killed inside its first transaction) completes as a first
/// launch and keeps the identity. SQLite treats a journal beside a zero-page database as junk.
#[test]
fn rw1_01_a_zero_length_database_beside_a_journal_completes_as_a_first_launch() {
    let mut magic = vec![0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];
    magic.resize(512, 0);
    for (label, journal) in [
        ("garbage", vec![7u8; 512]),
        ("journal magic", magic),
        ("one byte", vec![1u8]),
    ] {
        for with_id in [false, true] {
            let dir = TempDir::new().unwrap();
            fs::write(dir.path().join("chess.db"), b"").unwrap();
            fs::write(dir.path().join("chess.db-journal"), &journal).unwrap();
            let id = valid_store_id();
            if with_id {
                fs::write(dir.path().join("store.id"), &id).unwrap();
            }
            DataDir::new(dir.path()).ensure().unwrap();
            let case = format!("{label} journal, store.id={with_id}");
            let opened = try_open(dir.path()).unwrap_or_else(|e| panic!("{case}: {e:?}"));
            assert!(opened.report.first_launch, "{case}");
            drop(opened);
            if with_id {
                assert_eq!(fs::read(dir.path().join("store.id")).unwrap(), id, "{case}");
            }
        }
    }
}

/// FIND-RW1-01 (class P wins): the same killed-creation states beside any one data trace are
/// still refused `Missing`, nothing is written, moved or removed.
#[test]
fn rw1_01_each_data_trace_beside_a_killed_creation_state_is_still_refused() {
    let (real_db, real_wal) = real_wal_pair();
    let traces: [(&str, &str); 7] = [
        ("journal file", JNL),
        (
            "journal/done",
            "journal/done/22222222-2222-2222-2222-222222222222.jnl",
        ),
        ("backups", "backups/chess-2026-11-01.db"),
        ("bak-v1", "chess.db.bak-v1"),
        ("tmp-bak", "chess.db.tmp-bak-v1-7"),
        ("pre-import", "chess.db.pre-import-x"),
        ("restoring", "chess.db.restoring"),
    ];
    for (label, rel) in traces {
        // State 1: schemaless non-zero database and a non-empty WAL.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("chess.db"), &real_db).unwrap();
        fs::write(dir.path().join("chess.db-wal"), &real_wal).unwrap();
        fs::write(dir.path().join("store.id"), valid_store_id()).unwrap();
        DataDir::new(dir.path()).ensure().unwrap();
        write(dir.path(), rel, b"x");
        assert_missing_and_untouched(dir.path(), &format!("state 1 + {label}"));
        // State 2: zero-length database and a non-empty `-journal`.
        let dir = TempDir::new().unwrap();
        empty_store_dir(dir.path());
        write(dir.path(), "chess.db-journal", &[7u8; 512]);
        write(dir.path(), rel, b"x");
        assert_missing_and_untouched(dir.path(), &format!("state 2 + {label}"));
    }
}

/// NF4-10: tables and `user_version` 0 stay `Corrupt` (as built), traces or not.
#[test]
fn nf4_10_tables_with_user_version_zero_stay_corrupt() {
    let dir = TempDir::new().unwrap();
    Connection::open(dir.path().join("chess.db"))
        .unwrap()
        .execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE foreign_table (x);")
        .unwrap();
    write(dir.path(), JNL, b"x");
    assert!(matches!(try_open(dir.path()), Err(StoreError::Corrupt)));
}

/// NF4-11 (FIND-007): a database with the v1 schema, journals present and **no** `store.id`
/// opens; `store.id` is not created; the journal is not available.
#[test]
fn nf4_11_a_schema_without_store_id_opens_and_store_id_is_not_recreated() {
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    fs::remove_file(dir.path().join("store.id")).unwrap();
    write(dir.path(), JNL, b"x");
    let opened = try_open(dir.path()).unwrap();
    assert!(!opened.report.first_launch);
    assert!(!dir.path().join("store.id").exists());
    assert!(matches!(opened.store.journal(), Err(StoreError::Corrupt)));
}

#[test]
fn nf4_12_and_13_benign_names_beside_an_empty_database_are_a_first_launch() {
    let dir = TempDir::new().unwrap();
    empty_store_dir(dir.path());
    write(dir.path(), "journal/.DS_Store", b"x");
    write(dir.path(), "journal/._x", b"x");
    write(dir.path(), "backups/.DS_Store", b"x");
    write(dir.path(), "backups/._y", b"x");
    write(dir.path(), "chess.db-wal", b"");
    write(dir.path(), "chess.db-shm", b"");
    write(dir.path(), "chess.db-journal", b"");
    write(dir.path(), "logs/app.log", b"x");
    write(dir.path(), "config.toml", b"x");
    write(dir.path(), "missing-2026-11-01/store.id", b"x");
    assert!(try_open(dir.path()).unwrap().report.first_launch);
}

/// A store with players and a schema is never refused for lack of traces: NF-4 looks only at an
/// empty database.
#[test]
fn nf4_a_database_with_a_schema_is_never_refused_by_nf4() {
    let dir = TempDir::new().unwrap();
    drop(common::open_in(dir.path()));
    write(dir.path(), JNL, b"x");
    write(dir.path(), "chess.db.bak-v1", b"x");
    write(dir.path(), "backups/chess-2026-11-01.db", b"x");
    assert!(!try_open(dir.path()).unwrap().report.first_launch);
}
