//! CT-ST-007 for the part D2 commands: a fault just before the commit of `finalize_game`,
//! `abort_game`, `void_unreadable_game` and `reset_player` leaves the content of every table
//! as it was, and the same command succeeds once the fault is gone. Without the feature this
//! file is empty.
#![cfg(feature = "test-support")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use chess_domain::{GameResult, GameStatus, Matchup, Mode, WallMs};
use chess_store::fault::FaultPlan;
use chess_store::{AbortRequest, FinalizeRequest, Store, StoreError, VoidSpec};
use common::{block_on, computer, gid, human, new_game, open_in, pid, raw};
use rusqlite::Connection;
use tempfile::TempDir;

const EIO: i32 = 5;

fn world() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    for (n, name) in [(1, "Ann"), (2, "Bob")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![
        computer(10, "Bot A", 1, 0, 3),
        computer(11, "Bot B", 1, 1, 4),
    ]))
    .unwrap();
    (dir, s)
}

fn dump(conn: &Connection) -> Vec<String> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut out = Vec::new();
    for table in tables {
        let mut st = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = st.column_count();
        let rows: Vec<String> = st
            .query_map([], |r| {
                let mut cells = Vec::new();
                for i in 0..columns {
                    let v: rusqlite::types::Value = r.get(i)?;
                    cells.push(format!("{v:?}"));
                }
                Ok(cells.join("|"))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        out.extend(rows.into_iter().map(|r| format!("{table}: {r}")));
    }
    out.sort();
    out
}

fn finalize_req() -> FinalizeRequest {
    FinalizeRequest {
        game: gid(1),
        result: GameResult::WhiteWins,
        termination: "checkmate".into(),
        ended_at: WallMs(9_000_000),
        white_time_used_ms: 1,
        black_time_used_ms: 2,
        total_active_ms: 3,
        eco: None,
        opening_name: None,
        analysis_used: false,
        content_hash: "sha256:v1:ff".into(),
        final_journal_seq: 7,
    }
}

/// Runs `command` with `point` failing, checks that the content is unchanged, then runs it
/// again without the fault and returns its result.
fn injected<T: std::fmt::Debug>(
    d: &TempDir,
    point: &str,
    command: impl Fn() -> Result<T, StoreError>,
) -> T {
    let before = dump(&raw(d.path()));
    let guard = FaultPlan::new()
        .errno(point, EIO)
        .install(d.path())
        .unwrap();
    let err = command().unwrap_err();
    assert!(matches!(err, StoreError::Unavailable { .. }), "{err:?}");
    assert_eq!((guard.hits(point), guard.fired(point)), (1, 1));
    assert_eq!(before, dump(&raw(d.path())), "{point}: content changed");
    drop(guard);
    let done = command().unwrap();
    assert_ne!(
        before,
        dump(&raw(d.path())),
        "{point}: the retry changed nothing"
    );
    done
}

#[test]
fn ct_st_007_finalize_with_a_fault_before_commit_changes_nothing_and_the_retry_succeeds() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    injected(&d, "finalize.before_commit", || {
        block_on(s.finalize_game(finalize_req()))
    });
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::Completed
    );
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().strict_elo, 1204);
}

#[test]
fn ct_st_007_abort_with_a_fault_before_commit_changes_nothing_and_the_retry_succeeds() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(10),
        pid(11),
        Matchup::ComputerComputer,
        Mode::Strict,
    )))
    .unwrap();
    injected(&d, "abort.before_commit", || {
        block_on(s.abort_game(AbortRequest {
            game: gid(1),
            ended_at: WallMs(5),
            content_hash: "sha256:v1:ab".into(),
            final_journal_seq: 2,
        }))
    });
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::Aborted
    );
}

#[test]
fn ct_st_007_void_with_a_fault_before_commit_changes_nothing_and_the_retry_succeeds() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    injected(&d, "void.before_commit", || {
        block_on(s.void_unreadable_game(gid(1), "sha256:v1:00".into()))
    });
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::Voided
    );
}

#[test]
fn ct_st_007_reset_with_a_fault_before_commit_changes_nothing_and_the_retry_succeeds() {
    let (d, s) = world();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
    block_on(s.lease_upsert(gid(1), "server:99999".into())).unwrap();
    injected(&d, "reset.before_commit", || {
        block_on(s.reset_player(
            pid(1),
            vec![VoidSpec {
                game: gid(1),
                content_hash: "sha256:v1:r0".into(),
            }],
            "{}".into(),
        ))
    });
    let p = block_on(s.get_player(pid(1))).unwrap();
    assert!(p.reset_at.is_some());
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::Voided
    );
}
