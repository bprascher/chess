//! Part B rework items of the store part A review: NF-2 (a first launch cut off inside the WAL
//! switch must not wedge the directory).
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

use chess_store::{StoreError, UnavailableCause};
use common::{open_in, try_open};
use rusqlite::Connection;
use tempfile::TempDir;

/// Leaves `chess.db` and a **hot rollback journal** in `target`: a transaction that created a
/// table and spilled pages is in flight while the two files are copied, as a kill -9 would
/// leave them (the state the reviewer found in 1.3 % of random kills). The committed state
/// before the transaction is an empty database (no tables, `user_version` 0).
fn hot_journal_dir(target: &Path) {
    let src = TempDir::new().unwrap();
    let db = src.path().join("chess.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE x (a); DROP TABLE x;")
        .unwrap();
    conn.execute_batch("PRAGMA cache_size=1; BEGIN; CREATE TABLE t (x);")
        .unwrap();
    for _ in 0..50 {
        conn.execute("INSERT INTO t VALUES (zeroblob(4000))", [])
            .unwrap();
    }
    assert!(
        src.path().join("chess.db-journal").exists(),
        "the journal is hot"
    );
    fs::copy(&db, target.join("chess.db")).unwrap();
    fs::copy(
        src.path().join("chess.db-journal"),
        target.join("chess.db-journal"),
    )
    .unwrap();
    // The source transaction rolls back when `conn` drops; the copies are the crash state.
}

fn read_only_probe_fails(dir: &Path) -> bool {
    let c = Connection::open_with_flags(
        dir.join("chess.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
        .is_err()
}

/// NF-2: the state is real (a read-only connection fails on it), and the open completes the
/// interrupted first creation instead of answering `Unavailable{ReadOnly}` for ever.
#[test]
fn nf_2_a_hot_rollback_journal_of_an_interrupted_first_creation_is_recovered() {
    let dir = TempDir::new().unwrap();
    hot_journal_dir(dir.path());
    assert!(
        read_only_probe_fails(dir.path()),
        "the read-only probe cannot read this state"
    );
    let opened = try_open(dir.path()).unwrap();
    assert!(
        opened.report.first_launch,
        "the interrupted creation is completed"
    );
    assert_eq!(opened.report.schema_version, 1);
    drop(opened);
    assert!(
        !open_in(dir.path()).report.first_launch,
        "and the next open is ordinary"
    );
}

/// The exception is narrow: a hot journal beside signs of a store with data (a backup, a
/// journal file, a pre-upgrade copy) is **not** touched: the error stays and no byte changes.
#[test]
fn nf_2_the_exception_does_not_apply_to_a_directory_with_a_store_in_it() {
    let marks: [(&str, bool); 3] = [
        ("backups/chess-2026-11-01.db", false),
        ("journal/11111111-1111-1111-1111-111111111111.jnl", false),
        ("chess.db.bak-v1", true),
    ];
    for (mark, _) in marks {
        let dir = TempDir::new().unwrap();
        hot_journal_dir(dir.path());
        let path = dir.path().join(mark);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"x").unwrap();
        let before = (
            fs::read(dir.path().join("chess.db")).unwrap(),
            fs::read(dir.path().join("chess.db-journal")).unwrap(),
        );
        let err = try_open(dir.path()).err().unwrap();
        assert!(
            matches!(
                err,
                StoreError::Unavailable {
                    cause: UnavailableCause::ReadOnly
                }
            ),
            "{mark}: {err:?}"
        );
        let after = (
            fs::read(dir.path().join("chess.db")).unwrap(),
            fs::read(dir.path().join("chess.db-journal")).unwrap(),
        );
        assert_eq!(before, after, "{mark}: nothing written");
    }
}
