//! Part D5, CT-ST-007 for the statistics-side commands (DAT-7): an injected failure before the
//! commit of each write command leaves the store's content identical, and the command works
//! once the fault is gone. Without the feature `test-support` this file is empty.
#![cfg(feature = "test-support")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d5_support;

use std::path::Path;

use chess_store::fault::{FaultPlan, SDD_FAILPOINTS};
use chess_store::{EvalFlag, MoveEvalRow, Store, StoreError, UnavailableCause};
use common::{block_on, gid, raw};
use d5_support::{G, seed, world};
use rusqlite::types::Value;
use tempfile::TempDir;

const EIO: i32 = 5;
const TABLES: [&str; 3] = ["move_eval", "variation", "analysis_job"];

/// Every row of the three tables, in primary-key order.
fn dump(dir: &Path) -> Vec<(String, Vec<Vec<Value>>)> {
    let conn = raw(dir);
    TABLES
        .iter()
        .map(|table| {
            let mut st = conn
                .prepare(&format!("SELECT * FROM {table} ORDER BY 1, 2"))
                .unwrap();
            let n = st.column_count();
            let rows = st
                .query_map([], |r| (0..n).map(|i| r.get::<_, Value>(i)).collect())
                .unwrap()
                .map(Result::unwrap)
                .collect();
            ((*table).to_owned(), rows)
        })
        .collect()
}

fn eval(ply: u32) -> MoveEvalRow {
    MoveEvalRow {
        game: gid(1),
        ply,
        cp: Some(10),
        mate: None,
        best_uci: None,
        cp_loss: None,
        flag: Some(EvalFlag::Mistake),
        depth: Some(12),
        engine_version: "t".to_owned(),
    }
}

/// A store with games 1 (2 plies, finished) and 2 (3 plies, finished); one eval of game 1, one
/// variation, and jobs in every state: 1 running (owner A), 2 failed.
fn fixture(dir: &Path) -> Store {
    let s = world(dir);
    let mut g1 = G::done(1, 1, 2, "1-0");
    g1.plies = 2;
    let mut g2 = G::done(2, 1, 2, "1-0");
    g2.plies = 3;
    seed(dir, &g1);
    seed(dir, &g2);
    block_on(s.upsert_move_eval(eval(1))).unwrap();
    block_on(s.save_variation(gid(1), 0, "1. d4".to_owned())).unwrap();
    block_on(s.enqueue_job(gid(1))).unwrap();
    block_on(s.enqueue_job(gid(2))).unwrap();
    assert_eq!(block_on(s.claim_job("A".to_owned())).unwrap(), Some(gid(1)));
    assert_eq!(block_on(s.claim_job("B".to_owned())).unwrap(), Some(gid(2)));
    block_on(s.fail_job(gid(2), "B".to_owned())).unwrap();
    s
}

type Call = Box<dyn Fn(&Store) -> Result<(), StoreError>>;

fn calls() -> Vec<(&'static str, &'static str, Call)> {
    let call = |f: Call| f;
    vec![
        (
            "upsert_move_eval",
            "eval.before_commit",
            call(Box::new(|s| block_on(s.upsert_move_eval(eval(2))))),
        ),
        (
            "upsert_move_eval (replace)",
            "eval.before_commit",
            call(Box::new(|s| {
                let mut e = eval(1);
                e.cp = Some(-99);
                block_on(s.upsert_move_eval(e))
            })),
        ),
        (
            "save_variation",
            "variation.before_commit",
            call(Box::new(|s| {
                block_on(s.save_variation(gid(1), 1, "2. c4".to_owned())).map(drop)
            })),
        ),
        (
            "delete_variation",
            "variation.before_commit",
            call(Box::new(|s| {
                let v = block_on(s.list_variations(gid(1))).unwrap();
                block_on(s.delete_variation(gid(1), v[0].from_ply, v[0].pgn.clone())).map(drop)
            })),
        ),
        (
            "enqueue_job",
            "job.before_commit",
            call(Box::new(|s| {
                // A fourth game with a job to create.
                block_on(s.enqueue_job(gid(4))).map(drop)
            })),
        ),
        (
            "enqueue_unevaluated_games",
            "job.before_commit",
            call(Box::new(|s| {
                block_on(s.enqueue_unevaluated_games()).map(drop)
            })),
        ),
        (
            "claim_job",
            "job.before_commit",
            call(Box::new(|s| {
                block_on(s.claim_job("C".to_owned())).map(drop)
            })),
        ),
        (
            "job_heartbeat",
            "job.before_commit",
            call(Box::new(|s| {
                block_on(s.job_heartbeat(gid(1), "A".to_owned()))
            })),
        ),
        (
            "complete_job",
            "job.before_commit",
            call(Box::new(|s| {
                block_on(s.complete_job(gid(1), "A".to_owned()))
            })),
        ),
        (
            "fail_job",
            "job.before_commit",
            call(Box::new(|s| block_on(s.fail_job(gid(1), "A".to_owned())))),
        ),
        (
            "enqueue_failed_once_per_start",
            "job.before_commit",
            call(Box::new(|s| {
                block_on(s.enqueue_failed_once_per_start()).map(drop)
            })),
        ),
    ]
}

#[test]
fn ct_st_007_a_fault_before_the_commit_changes_nothing_and_the_command_works_afterwards() {
    for (name, failpoint, call) in calls() {
        let dir = TempDir::new().unwrap();
        let s = fixture(dir.path());
        // Game 3: queued job (claimable). Game 4: finished, with moves, no job.
        for n in [3, 4] {
            let mut g = G::done(n, 1, 2, "1-0");
            g.plies = 2;
            seed(dir.path(), &g);
        }
        block_on(s.enqueue_job(gid(3))).unwrap();
        let before = dump(dir.path());
        let guard = FaultPlan::new()
            .errno(failpoint, EIO)
            .install(dir.path())
            .unwrap();
        let failed = call(&s);
        assert!(
            matches!(
                failed,
                Err(StoreError::Unavailable {
                    cause: UnavailableCause::Io(_)
                })
            ),
            "{name}: {failed:?}"
        );
        assert!(guard.fired(failpoint) >= 1, "{name}: the failpoint fired");
        assert_eq!(dump(dir.path()), before, "{name}: store content unchanged");
        drop(guard);
        call(&s).unwrap_or_else(|e| panic!("{name}: works without the fault: {e:?}"));
        assert_ne!(
            dump(dir.path()),
            before,
            "{name}: and then it does change the store"
        );
    }
}

#[test]
fn a_failed_requeue_does_not_use_up_the_once_per_start_retry() {
    let dir = TempDir::new().unwrap();
    let s = fixture(dir.path());
    let guard = FaultPlan::new()
        .errno("job.before_commit", EIO)
        .install(dir.path())
        .unwrap();
    assert!(block_on(s.enqueue_failed_once_per_start()).is_err());
    drop(guard);
    assert_eq!(
        block_on(s.enqueue_failed_once_per_start()).unwrap(),
        1,
        "game 2 was failed"
    );
    assert_eq!(block_on(s.enqueue_failed_once_per_start()).unwrap(), 0);
}

#[test]
fn the_d5_failpoints_are_registered_and_distinct_from_the_sdd_list() {
    let registered: Vec<&str> = chess_store::fault::all().collect();
    for name in chess_store::fault::D5_FAILPOINTS {
        assert!(registered.contains(name), "{name}");
        assert!(!SDD_FAILPOINTS.contains(name), "{name} is a D5 addition");
    }
}
