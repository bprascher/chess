//! FIND-B-06 narrowed (Brian Prascher, 2026-11-22; store-rework-specs section 1): the opener
//! rolls back a hot rollback journal only for an interrupted first creation **and** when a
//! rollback of a private copy leaves no players and no games. NF2-2 and NF2-3 are the real v1
//! store case of review R06: the error stays and `chess.db` is byte for byte as found.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d5_support;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chess_store::{StoreError, UnavailableCause};
use common::{open_in, try_open};
use d5_support::{G, seed};
use rusqlite::Connection;
use tempfile::TempDir;

/// These tests look at the system temp directory for `chess-nf2-<pid>-*` entries; one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Scratch directories of this process that the opener left in the temp directory.
fn leaked_scratch() -> Vec<String> {
    let prefix = format!("chess-nf2-{}-", std::process::id());
    fs::read_dir(std::env::temp_dir())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(&prefix))
        .collect()
}

/// Leaves `chess.db` (and `store.id` when the source has one) and a **hot rollback journal** in
/// `target`: a transaction that created a table and spilled pages is in flight in `src` while
/// the files are copied, as a kill -9 would leave them. The committed state before the
/// transaction is whatever `src` held.
fn hot_journal_from(src: &Path, target: &Path) {
    let db = src.join("chess.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch("PRAGMA journal_mode=DELETE;").unwrap();
    conn.execute_batch("PRAGMA cache_size=1; BEGIN; CREATE TABLE zz_inflight (x);")
        .unwrap();
    for _ in 0..50 {
        conn.execute("INSERT INTO zz_inflight VALUES (zeroblob(4000))", [])
            .unwrap();
    }
    assert!(src.join("chess.db-journal").exists(), "the journal is hot");
    fs::copy(&db, target.join("chess.db")).unwrap();
    fs::copy(
        src.join("chess.db-journal"),
        target.join("chess.db-journal"),
    )
    .unwrap();
    if src.join("store.id").exists() {
        fs::copy(src.join("store.id"), target.join("store.id")).unwrap();
    }
}

/// An interrupted first creation: an empty database file and a hot journal.
fn hot_journal_empty(target: &Path) {
    let src = TempDir::new().unwrap();
    Connection::open(src.path().join("chess.db"))
        .unwrap()
        .execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE x (a); DROP TABLE x;")
        .unwrap();
    hot_journal_from(src.path(), target);
}

/// A real v1 store (made by `Store::open`), with `fill` applied, closed; then the hot journal.
fn hot_journal_store(target: &Path, fill: impl FnOnce(&Path)) {
    let src = TempDir::new().unwrap();
    let store = d5_support::world(src.path());
    drop(store);
    fill(src.path());
    hot_journal_from(src.path(), target);
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

/// Length and FNV-1a of the bytes: a readable failure message instead of a byte dump.
fn digest(bytes: &[u8]) -> (usize, u64) {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    (bytes.len(), hash)
}

/// Every regular file under `dir` with its bytes, ignoring the zero-length `-wal` and `-shm`
/// that a read-only probe of a WAL database may leave (F-A09).
fn snapshot(dir: &Path) -> BTreeMap<String, (usize, u64)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            let bytes = fs::read(&p).unwrap();
            // `-shm` is the WAL index SQLite builds on any open of a WAL database, empty or not.
            if rel.ends_with("-shm") || (bytes.is_empty() && rel.ends_with("-wal")) {
                continue;
            }
            out.insert(rel, digest(&bytes));
        }
    }
    out
}

fn is_read_only(err: &StoreError) -> bool {
    matches!(
        err,
        StoreError::Unavailable {
            cause: UnavailableCause::ReadOnly
        }
    )
}

fn assert_refused_and_untouched(dir: &Path, label: &str) {
    assert!(
        read_only_probe_fails(dir),
        "{label}: the read-only probe cannot read this state"
    );
    let before = snapshot(dir);
    let err = try_open(dir)
        .err()
        .unwrap_or_else(|| panic!("{label}: opened"));
    assert!(is_read_only(&err), "{label}: {err:?}");
    assert_eq!(before, snapshot(dir), "{label}: not a byte changed");
}

/// NF2-1 (with the positive test in `hardening.rs`) and the size measurement behind
/// `NF2_COPY_CAP`: a freshly created schema is far below a sixteenth of the 4 MiB cap.
#[test]
fn nf2_1_an_interrupted_first_creation_is_completed_and_the_cap_has_room() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    hot_journal_empty(dir.path());
    assert!(read_only_probe_fails(dir.path()));
    let opened = try_open(dir.path()).unwrap();
    assert!(opened.report.first_launch);
    drop(opened);
    assert!(
        leaked_scratch().is_empty(),
        "NF2-10: {:?}",
        leaked_scratch()
    );
    // Size of an empty v1 schema: pages x page size of a store with no data.
    let conn = Connection::open(dir.path().join("chess.db")).unwrap();
    let pages: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .unwrap();
    let size: i64 = conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .unwrap();
    let bytes = pages * size;
    eprintln!("empty v1 schema: {pages} pages x {size} = {bytes} bytes");
    assert!(bytes * 16 <= 4 * 1024 * 1024, "{bytes}");
}

/// NF2-2, the real v1 store case (review R06): a store with one Player, a hot journal and no
/// marker files keeps its error, `chess.db` and `chess.db-journal` are byte for byte as found,
/// and no scratch directory is left.
#[test]
fn nf2_2_a_v1_store_with_a_player_keeps_its_error_and_chess_db_is_not_touched() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    hot_journal_store(dir.path(), |_| {});
    assert_refused_and_untouched(dir.path(), "player");
    assert!(leaked_scratch().is_empty(), "{:?}", leaked_scratch());
    // The data is still there, in the files, for a later repair: a copy rolled back shows Ann.
    let copy = TempDir::new().unwrap();
    for f in ["chess.db", "chess.db-journal"] {
        fs::copy(dir.path().join(f), copy.path().join(f)).unwrap();
    }
    let n: i64 = Connection::open(copy.path().join("chess.db"))
        .unwrap()
        .query_row("SELECT count(*) FROM player WHERE name = 'Ann'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 1);
}

/// NF2-3: the same with a finished game (and its Players).
#[test]
fn nf2_3_a_v1_store_with_a_game_keeps_its_error() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    hot_journal_store(dir.path(), |src| {
        let mut g = G::done(1, 1, 2, "1-0");
        g.plies = 3;
        seed(src, &g);
    });
    assert_refused_and_untouched(dir.path(), "game");
    assert!(leaked_scratch().is_empty());
}

/// NF2-3b: a game row alone (an imported game has no Player) is data too: condition G is
/// "no players **and** no games".
#[test]
fn nf2_3b_a_game_without_any_player_still_keeps_the_error() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    drop(open_in(src.path())); // schema only
    let mut g = G::done(1, 1, 2, "1-0");
    g.white = None;
    g.black = None;
    g.status = "imported";
    g.plies = 2;
    seed_without_players(src.path(), &g);
    hot_journal_from(src.path(), dir.path());
    assert_refused_and_untouched(dir.path(), "imported game, no players");
}

fn seed_without_players(dir: &Path, g: &G) {
    let conn = common::raw(dir);
    assert_eq!(
        conn.query_row("SELECT count(*) FROM player", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    d5_support::seed_on(&conn, g);
}

/// A store that has Players but whose only content is a game row: also refused. A store with a
/// schema and no players and no games (NF2-4) takes the exception.
#[test]
fn nf2_4_a_schema_with_no_players_and_no_games_takes_the_exception() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    drop(open_in(src.path())); // schema, no Player, no game
    hot_journal_from(src.path(), dir.path());
    assert!(read_only_probe_fails(dir.path()));
    let opened = try_open(dir.path()).unwrap();
    assert!(
        !opened.report.first_launch,
        "an existing empty store, not a first launch"
    );
    assert_eq!(opened.report.schema_version, 1);
    drop(opened);
    assert!(leaked_scratch().is_empty());
}

/// NF2-5: each of the conditions C to F alone, with an otherwise empty database and a hot
/// journal: the error stays and nothing changes.
#[test]
fn nf2_5_each_data_marker_alone_keeps_the_error() {
    let _g = serial();
    let markers: [(&str, &[u8]); 8] = [
        ("journal/11111111-1111-1111-1111-111111111111.jnl", b"x"),
        ("backups/chess-2026-11-01.db", b"x"),
        ("chess.db.bak-v1", b"x"),
        ("chess.db.tmp-bak-v1-1", b"x"),
        ("chess.db.pre-import-x", b"x"),
        ("chess.db.restoring", b"x"),
        ("chess.db-wal", b"not empty"),
        (
            "journal/done/22222222-2222-2222-2222-222222222222.jnl",
            b"x",
        ),
    ];
    for (mark, content) in markers {
        let dir = TempDir::new().unwrap();
        hot_journal_empty(dir.path());
        let path = dir.path().join(mark);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        assert_refused_and_untouched(dir.path(), mark);
    }
    assert!(leaked_scratch().is_empty());
}

/// Benign names beside the hot journal do not stop the exception (`.DS_Store`, zero-length
/// `-wal`, an empty `journal/done`).
#[test]
fn nf2_5b_benign_names_do_not_block_the_exception() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    hot_journal_empty(dir.path());
    fs::create_dir_all(dir.path().join("journal/done")).unwrap();
    fs::create_dir_all(dir.path().join("backups")).unwrap();
    fs::write(dir.path().join("backups/.DS_Store"), b"x").unwrap();
    fs::write(dir.path().join("journal/._x"), b"x").unwrap();
    fs::write(dir.path().join("config.toml"), b"").unwrap();
    assert!(try_open(dir.path()).unwrap().report.first_launch);
}

/// NF2-6 (H): `chess.db` above the copy cap with a hot journal: the error stays, untouched.
#[test]
fn nf2_6_a_database_above_the_copy_cap_is_not_copied_and_keeps_the_error() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    hot_journal_empty(dir.path());
    let f = fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join("chess.db"))
        .unwrap();
    f.set_len(5 * 1024 * 1024).unwrap(); // sparse
    drop(f);
    assert_refused_and_untouched(dir.path(), "5 MiB database");
    assert!(leaked_scratch().is_empty());
}

/// NF2-8 (I): a journal SQLite cannot use on the copy gives "cannot tell" and the error stays.
#[test]
fn nf2_8_a_copy_that_cannot_be_examined_keeps_the_error() {
    let _g = serial();
    let dir = TempDir::new().unwrap();
    hot_journal_store(dir.path(), |_| {});
    // Damage the main file's header after the copy: the journal is still hot, the database is
    // not a database any more.
    let mut bytes = fs::read(dir.path().join("chess.db")).unwrap();
    bytes[..16].copy_from_slice(b"NOT AN SQLITE DB");
    fs::write(dir.path().join("chess.db"), &bytes).unwrap();
    let before = snapshot(dir.path());
    let err = try_open(dir.path()).err().unwrap();
    assert!(is_read_only(&err), "{err:?}");
    assert_eq!(before, snapshot(dir.path()), "not a byte changed");
    assert!(leaked_scratch().is_empty());
}

#[cfg(feature = "test-support")]
mod injected {
    use super::*;
    use chess_store::fault::FaultPlan;

    /// NF2-7 (I): the copy fails (injected I/O error): the original `ReadOnly` error, not the I/O
    /// error, and the real files are untouched. The failpoint was reached.
    #[test]
    fn nf2_7_a_failed_copy_keeps_the_original_error() {
        let _g = serial();
        let dir = TempDir::new().unwrap();
        hot_journal_empty(dir.path());
        let guard = FaultPlan::new()
            .errno("open.nf2_copy", 5)
            .install(dir.path())
            .unwrap();
        assert_refused_and_untouched(dir.path(), "copy fails");
        assert_eq!(
            (guard.hits("open.nf2_copy"), guard.fired("open.nf2_copy")),
            (1, 1)
        );
        assert!(
            leaked_scratch().is_empty(),
            "scratch removed after the failure"
        );
        drop(guard);
        // Without the fault the same directory is recovered.
        assert!(try_open(dir.path()).unwrap().report.first_launch);
    }

    /// NF2-6, second half: above the cap nothing is copied (the copy failpoint is never reached).
    #[test]
    fn nf2_6_above_the_cap_the_copy_is_never_attempted() {
        let _g = serial();
        let dir = TempDir::new().unwrap();
        hot_journal_empty(dir.path());
        fs::OpenOptions::new()
            .write(true)
            .open(dir.path().join("chess.db"))
            .unwrap()
            .set_len(5 * 1024 * 1024)
            .unwrap();
        let guard = FaultPlan::new()
            .errno("open.nf2_copy", 5)
            .install(dir.path())
            .unwrap();
        assert!(is_read_only(&try_open(dir.path()).err().unwrap()));
        assert_eq!(guard.hits("open.nf2_copy"), 0);
    }

    const ENV_DIR: &str = "CHESS_STORE_NF2_DIR";

    /// The child of `nf2_8b`: opens the directory with a `Kill` plan at the copy failpoint. Runs
    /// only when the parent set the variable.
    #[test]
    fn nf2_scratch_child_entry() {
        let Ok(dir) = std::env::var(ENV_DIR) else {
            return;
        };
        let _guard = FaultPlan::parse("open.nf2_copy=kill")
            .unwrap()
            .install(Path::new(&dir))
            .unwrap();
        let _ = try_open(Path::new(&dir));
    }

    /// FIND-RW1-08: an opener killed inside the NF-2 copy leaves its scratch directory in the
    /// temp directory (mode `0700`); the next open, in another process, removes it, and the
    /// directory itself is still recovered as an interrupted first creation.
    #[test]
    fn nf2_8b_a_killed_opener_leaves_scratch_and_the_next_open_sweeps_it() {
        use std::os::unix::fs::PermissionsExt;
        let _g = serial();
        let dir = TempDir::new().unwrap();
        hot_journal_empty(dir.path());
        let mut child = common::child::spawn(
            "injected::nf2_scratch_child_entry",
            &[(ENV_DIR, dir.path().to_str().unwrap())],
        );
        let at = child.killed_at(std::time::Duration::from_secs(60));
        let pid = child.pid();
        child.kill();
        assert_eq!(
            at.as_deref(),
            Some("open.nf2_copy"),
            "the child reached the copy"
        );
        let stale = std::env::temp_dir().join(format!("chess-nf2-{pid}-0"));
        let mode = fs::metadata(&stale).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "the killed opener left its scratch directory");
        assert!(try_open(dir.path()).unwrap().report.first_launch);
        assert!(!stale.exists(), "the next open swept it");
    }

    /// Condition G is what decides: with data the copy is made (the failpoint is reached) and
    /// the answer is still no.
    #[test]
    fn nf2_2b_a_store_with_data_reaches_the_copy_and_is_refused() {
        let _g = serial();
        let dir = TempDir::new().unwrap();
        hot_journal_store(dir.path(), |_| {});
        let guard = FaultPlan::new()
            .delay("open.nf2_copy", std::time::Duration::ZERO)
            .install(dir.path())
            .unwrap();
        assert_refused_and_untouched(dir.path(), "copy then no");
        assert_eq!(guard.hits("open.nf2_copy"), 1);
    }
}
