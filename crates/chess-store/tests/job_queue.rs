//! Part D5, the analysis job queue (DD-ST-013, SDD 3.11.7, STATE-ST-02): CT-ST-030 claim,
//! CT-ST-031 stale reclaim, CT-ST-032 lost-owner rejection, all with an injected clock.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d5_support;

use std::sync::atomic::Ordering;

use chess_store::{ANALYSIS_LEASE, Entity, EvalFlag, JobState, MoveEvalRow, Store, StoreError};
use common::{block_on, gid, raw};
use d5_support::{G, reopen_with_clock, seed, world_with_clock};
use tempfile::TempDir;

const LEASE: i64 = 10_000;

fn finished(dir: &std::path::Path, n: u128, plies: u32) {
    let mut g = G::done(n, 1, 2, "1-0");
    g.plies = plies;
    seed(dir, &g);
}

fn job_row(dir: &std::path::Path, n: u128) -> (String, Option<String>, Option<i64>) {
    raw(dir)
        .query_row(
            "SELECT state, owner, heartbeat_at FROM analysis_job \
             WHERE game_id = (SELECT id FROM game WHERE uuid = ?1)",
            [gid(n).as_uuid().hyphenated().to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
}

fn state(s: &Store, n: u128) -> Option<JobState> {
    block_on(s.get_game_analysis(gid(n))).unwrap().job
}

#[test]
fn the_claim_lease_is_ten_seconds() {
    assert_eq!(ANALYSIS_LEASE.as_millis(), 10_000);
}

#[test]
fn ct_st_030_claim_takes_the_lowest_queued_job_and_marks_it_running() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    for n in [3, 1, 2] {
        finished(dir.path(), n, 2);
    }
    // Enqueue out of order; the claim order is game order (the row key), lowest first.
    for n in [2, 3, 1] {
        assert!(block_on(s.enqueue_job(gid(n))).unwrap());
    }
    // game.id order follows insertion: 3, 1, 2.
    now.store(2_000, Ordering::SeqCst);
    let first = block_on(s.claim_job("cli:1".to_owned())).unwrap();
    assert_eq!(first, Some(gid(3)));
    assert_eq!(
        job_row(dir.path(), 3),
        ("running".into(), Some("cli:1".into()), Some(2_000))
    );
    let second = block_on(s.claim_job("cli:2".to_owned())).unwrap();
    assert_eq!(
        second,
        Some(gid(1)),
        "the next queued job, not the running one"
    );
    assert_eq!(
        block_on(s.claim_job("cli:3".to_owned())).unwrap(),
        Some(gid(2))
    );
    assert_eq!(
        block_on(s.claim_job("cli:4".to_owned())).unwrap(),
        None,
        "every job is running with a fresh heartbeat"
    );
    assert_eq!(state(&s, 1), Some(JobState::Running));
}

#[test]
fn claim_with_an_empty_queue_is_none() {
    let dir = TempDir::new().unwrap();
    let (s, _now) = world_with_clock(dir.path(), 1_000);
    assert_eq!(block_on(s.claim_job("cli:1".to_owned())).unwrap(), None);
}

#[test]
fn ct_st_031_a_running_job_is_reclaimable_only_when_its_heartbeat_is_older_than_the_lease() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 2);
    block_on(s.enqueue_job(gid(1))).unwrap();
    now.store(50_000, Ordering::SeqCst);
    assert_eq!(block_on(s.claim_job("A".to_owned())).unwrap(), Some(gid(1)));
    // Exactly the lease old: not stale (older than the lease means strictly older).
    now.store(50_000 + LEASE, Ordering::SeqCst);
    assert_eq!(
        block_on(s.claim_job("B".to_owned())).unwrap(),
        None,
        "10 s old is fresh"
    );
    // A heartbeat resets the age.
    now.store(50_000 + LEASE - 1, Ordering::SeqCst);
    block_on(s.job_heartbeat(gid(1), "A".to_owned())).unwrap();
    now.store(50_000 + LEASE - 1 + LEASE, Ordering::SeqCst);
    assert_eq!(block_on(s.claim_job("B".to_owned())).unwrap(), None);
    // One millisecond past the lease: B takes it.
    now.store(50_000 + LEASE - 1 + LEASE + 1, Ordering::SeqCst);
    assert_eq!(block_on(s.claim_job("B".to_owned())).unwrap(), Some(gid(1)));
    assert_eq!(
        job_row(dir.path(), 1),
        ("running".into(), Some("B".into()), Some(50_000 + 2 * LEASE))
    );
}

#[test]
fn a_heartbeat_dated_in_the_future_counts_as_fresh() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 2);
    block_on(s.enqueue_job(gid(1))).unwrap();
    now.store(900_000, Ordering::SeqCst);
    block_on(s.claim_job("A".to_owned())).unwrap();
    now.store(100_000, Ordering::SeqCst); // the wall clock stepped back
    assert_eq!(block_on(s.claim_job("B".to_owned())).unwrap(), None);
}

#[test]
fn a_running_job_without_a_heartbeat_is_stale() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 2);
    block_on(s.enqueue_job(gid(1))).unwrap();
    raw(dir.path())
        .execute(
            "UPDATE analysis_job SET state = 'running', owner = 'x:1', heartbeat_at = NULL",
            [],
        )
        .unwrap();
    now.store(2_000, Ordering::SeqCst);
    assert_eq!(block_on(s.claim_job("B".to_owned())).unwrap(), Some(gid(1)));
}

#[test]
fn ct_st_032_a_worker_that_lost_the_job_cannot_touch_it() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 2);
    block_on(s.enqueue_job(gid(1))).unwrap();
    now.store(5_000, Ordering::SeqCst);
    block_on(s.claim_job("A".to_owned())).unwrap();
    now.store(5_000 + LEASE + 1, Ordering::SeqCst);
    block_on(s.claim_job("B".to_owned())).unwrap().unwrap();
    let before = job_row(dir.path(), 1);
    assert_eq!(
        before,
        ("running".into(), Some("B".into()), Some(5_000 + LEASE + 1))
    );
    now.store(99_999, Ordering::SeqCst);
    let a = || "A".to_owned();
    assert!(matches!(
        block_on(s.job_heartbeat(gid(1), a())),
        Err(StoreError::NotOwner)
    ));
    assert!(matches!(
        block_on(s.complete_job(gid(1), a())),
        Err(StoreError::NotOwner)
    ));
    assert!(matches!(
        block_on(s.fail_job(gid(1), a())),
        Err(StoreError::NotOwner)
    ));
    assert_eq!(
        job_row(dir.path(), 1),
        before,
        "the new owner's row is untouched"
    );
    // B still can.
    block_on(s.job_heartbeat(gid(1), "B".to_owned())).unwrap();
    assert_eq!(job_row(dir.path(), 1).2, Some(99_999));
    block_on(s.complete_job(gid(1), "B".to_owned())).unwrap();
    assert_eq!(state(&s, 1), Some(JobState::Done));
}

#[test]
fn owner_commands_on_a_job_that_is_not_running_or_not_there() {
    let dir = TempDir::new().unwrap();
    let (s, _now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 2);
    let o = || "A".to_owned();
    // No job row.
    assert!(matches!(
        block_on(s.complete_job(gid(1), o())),
        Err(StoreError::NotOwner)
    ));
    // Queued, not claimed.
    block_on(s.enqueue_job(gid(1))).unwrap();
    assert!(matches!(
        block_on(s.job_heartbeat(gid(1), o())),
        Err(StoreError::NotOwner)
    ));
    assert!(matches!(
        block_on(s.fail_job(gid(1), o())),
        Err(StoreError::NotOwner)
    ));
    assert_eq!(state(&s, 1), Some(JobState::Queued));
    // Unknown game.
    assert!(matches!(
        block_on(s.complete_job(gid(9), o())),
        Err(StoreError::NotFound {
            entity: Entity::Game,
            ..
        })
    ));
    // Done is final: the same owner's retry of `complete` is answered `Ok` again (FIND-D5R-05),
    // every other command on the done job is refused, and the state stays.
    block_on(s.claim_job(o())).unwrap();
    block_on(s.complete_job(gid(1), o())).unwrap();
    block_on(s.complete_job(gid(1), o())).unwrap();
    assert!(matches!(
        block_on(s.fail_job(gid(1), o())),
        Err(StoreError::NotOwner)
    ));
    assert!(matches!(
        block_on(s.job_heartbeat(gid(1), o())),
        Err(StoreError::NotOwner)
    ));
    assert_eq!(state(&s, 1), Some(JobState::Done));
}

/// FIND-D5R-05: a worker whose reply was lost retries `complete_job` or `fail_job` and gets
/// success again, with no write (the row is identical); a different owner gets `NotOwner`, also
/// for the repeat of the other command and after a reclaim.
#[test]
fn complete_and_fail_are_idempotent_for_the_owner_that_already_did_it() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    for n in [1, 2, 3] {
        finished(dir.path(), n, 2);
        block_on(s.enqueue_job(gid(n))).unwrap();
    }
    let who = |name: &str| name.to_owned();
    // Game 1: complete, then complete again by A, by B, then fail by A.
    assert_eq!(block_on(s.claim_job(who("A"))).unwrap(), Some(gid(1)));
    block_on(s.complete_job(gid(1), who("A"))).unwrap();
    let done = job_row(dir.path(), 1);
    block_on(s.complete_job(gid(1), who("A"))).unwrap();
    assert_eq!(job_row(dir.path(), 1), done, "the retry wrote nothing");
    assert!(matches!(
        block_on(s.complete_job(gid(1), who("B"))),
        Err(StoreError::NotOwner)
    ));
    assert!(matches!(
        block_on(s.fail_job(gid(1), who("A"))),
        Err(StoreError::NotOwner)
    ));
    assert_eq!(job_row(dir.path(), 1), done);
    // Game 2: fail, then fail again by A, not by B, and not complete by A.
    assert_eq!(block_on(s.claim_job(who("A"))).unwrap(), Some(gid(2)));
    block_on(s.fail_job(gid(2), who("A"))).unwrap();
    let failed = job_row(dir.path(), 2);
    assert_eq!(failed.0, "failed");
    block_on(s.fail_job(gid(2), who("A"))).unwrap();
    assert_eq!(job_row(dir.path(), 2), failed);
    assert!(matches!(
        block_on(s.fail_job(gid(2), who("B"))),
        Err(StoreError::NotOwner)
    ));
    assert!(matches!(
        block_on(s.complete_job(gid(2), who("A"))),
        Err(StoreError::NotOwner)
    ));
    // Game 3: A claims, loses the job to B by lease, B completes: A's retry is refused, B's is not.
    now.store(5_000, Ordering::SeqCst);
    assert_eq!(block_on(s.claim_job(who("A"))).unwrap(), Some(gid(3)));
    now.store(5_000 + LEASE + 1, Ordering::SeqCst);
    assert_eq!(block_on(s.claim_job(who("B"))).unwrap(), Some(gid(3)));
    block_on(s.complete_job(gid(3), who("B"))).unwrap();
    assert!(matches!(
        block_on(s.complete_job(gid(3), who("A"))),
        Err(StoreError::NotOwner)
    ));
    block_on(s.complete_job(gid(3), who("B"))).unwrap();
    // An unknown game is still `NotFound`, a game without a job still `NotOwner`.
    assert!(matches!(
        block_on(s.complete_job(gid(9), who("A"))),
        Err(StoreError::NotFound { .. })
    ));
}

#[test]
fn a_done_job_never_goes_back() {
    let dir = TempDir::new().unwrap();
    let (s, _now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 2);
    block_on(s.enqueue_job(gid(1))).unwrap();
    block_on(s.claim_job("A".to_owned())).unwrap();
    block_on(s.complete_job(gid(1), "A".to_owned())).unwrap();
    assert!(
        !block_on(s.enqueue_job(gid(1))).unwrap(),
        "existing job: no new row"
    );
    assert_eq!(block_on(s.enqueue_failed_once_per_start()).unwrap(), 0);
    assert_eq!(block_on(s.enqueue_unevaluated_games()).unwrap(), 0);
    assert_eq!(block_on(s.claim_job("B".to_owned())).unwrap(), None);
    assert_eq!(state(&s, 1), Some(JobState::Done));
}

#[test]
fn enqueue_job_is_idempotent_and_leaves_running_and_failed_jobs_alone() {
    let dir = TempDir::new().unwrap();
    let (s, _now) = world_with_clock(dir.path(), 1_000);
    for n in [1, 2] {
        finished(dir.path(), n, 2);
    }
    assert!(block_on(s.enqueue_job(gid(1))).unwrap());
    assert!(!block_on(s.enqueue_job(gid(1))).unwrap());
    block_on(s.claim_job("A".to_owned())).unwrap();
    assert!(!block_on(s.enqueue_job(gid(1))).unwrap());
    assert_eq!(
        job_row(dir.path(), 1).1,
        Some("A".into()),
        "a running job keeps its owner"
    );
    block_on(s.fail_job(gid(1), "A".to_owned())).unwrap();
    assert!(!block_on(s.enqueue_job(gid(1))).unwrap());
    assert_eq!(state(&s, 1), Some(JobState::Failed));
    assert!(matches!(
        block_on(s.enqueue_job(gid(9))),
        Err(StoreError::NotFound {
            entity: Entity::Game,
            ..
        })
    ));
}

#[test]
fn enqueue_unevaluated_games_selects_finished_games_with_missing_evals() {
    let dir = TempDir::new().unwrap();
    let (s, _now) = world_with_clock(dir.path(), 1_000);
    finished(dir.path(), 1, 3); // no evals: queued
    finished(dir.path(), 2, 2); // fully evaluated: skipped
    finished(dir.path(), 3, 3); // partly evaluated: queued
    finished(dir.path(), 4, 0); // no moves: skipped
    finished(dir.path(), 5, 2); // has a job already: skipped
    let mut open = G::done(6, 1, 2, "1-0");
    open.status = "in_progress";
    open.result = None;
    open.plies = 3;
    seed(dir.path(), &open); // in progress: skipped
    let mut voided = G::done(7, 1, 2, "1-0");
    voided.status = "voided";
    voided.result = None;
    voided.plies = 2;
    seed(dir.path(), &voided); // voided is finished: queued
    let ev = |game, ply| MoveEvalRow {
        game: gid(game),
        ply,
        cp: Some(0),
        mate: None,
        best_uci: None,
        cp_loss: None,
        flag: None::<EvalFlag>,
        depth: None,
        engine_version: "t".to_owned(),
    };
    for ply in [1, 2] {
        block_on(s.upsert_move_eval(ev(2, ply))).unwrap();
    }
    block_on(s.upsert_move_eval(ev(3, 1))).unwrap();
    block_on(s.enqueue_job(gid(5))).unwrap();
    assert_eq!(
        block_on(s.enqueue_unevaluated_games()).unwrap(),
        3,
        "games 1, 3 and 7"
    );
    for (n, want) in [
        (1, Some(JobState::Queued)),
        (2, None),
        (3, Some(JobState::Queued)),
        (4, None),
        (5, Some(JobState::Queued)),
        (6, None),
        (7, Some(JobState::Queued)),
    ] {
        assert_eq!(state(&s, n), want, "game {n}");
    }
    assert_eq!(
        block_on(s.enqueue_unevaluated_games()).unwrap(),
        0,
        "idempotent"
    );
}

#[test]
fn enqueue_failed_once_per_start_requeues_failed_jobs_once_per_open() {
    let dir = TempDir::new().unwrap();
    let (s, now) = world_with_clock(dir.path(), 1_000);
    for n in [1, 2, 3] {
        finished(dir.path(), n, 2);
        block_on(s.enqueue_job(gid(n))).unwrap();
    }
    for n in [1, 2] {
        block_on(s.claim_job("A".to_owned())).unwrap();
        block_on(s.fail_job(gid(n), "A".to_owned())).unwrap();
    }
    // Game 3 stays queued. Two failed jobs go back to queued, owner and heartbeat cleared.
    assert_eq!(block_on(s.enqueue_failed_once_per_start()).unwrap(), 2);
    assert_eq!(job_row(dir.path(), 1), ("queued".into(), None, None));
    assert_eq!(job_row(dir.path(), 2), ("queued".into(), None, None));
    // Fails again: this start does not retry it a second time.
    block_on(s.claim_job("A".to_owned())).unwrap();
    block_on(s.fail_job(gid(1), "A".to_owned())).unwrap();
    assert_eq!(block_on(s.enqueue_failed_once_per_start()).unwrap(), 0);
    assert_eq!(state(&s, 1), Some(JobState::Failed));
    // A new start (a reopened store) may retry it once more.
    drop(s);
    let s = reopen_with_clock(dir.path(), &now);
    assert_eq!(block_on(s.enqueue_failed_once_per_start()).unwrap(), 1);
    assert_eq!(state(&s, 1), Some(JobState::Queued));
    assert_eq!(block_on(s.enqueue_failed_once_per_start()).unwrap(), 0);
}
