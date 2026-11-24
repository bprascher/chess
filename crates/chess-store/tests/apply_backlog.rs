//! Part D3, feature `test-support`: the apply backlog end to end (SDD 3.12, SEQ-ST-02) and the
//! failure tests of `apply_records`: CT-ST-007 (a fault just before and just after the commit),
//! CT-ST-008 (the database locked by another process: play continues from the journal, the
//! backlog drains, `StoreBacklog`), `sync_backlog`, the 100 ms lock wait, and kill tests at
//! `apply.before_commit` and `apply.after_commit` (a SIGKILLed child, reopen, apply again).
#![cfg(feature = "test-support")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use chess_domain::{GameStatus, Matchup, Mode};
use chess_store::fault::FaultPlan;
use chess_store::journal::format::{EndReason, GameEndPayload, JournalRecord, Payload};
use chess_store::journal::scan_file;
use chess_store::{ApplyOutcome, DataDir, Store, StoreError, StoreOptions};
use common::child::{self, Spawned};
use common::{
    block_on, block_on_within, computer, gid, human, jb, new_game, open_in, pid, raw, ticking_clock,
};
use rusqlite::Connection;
use tempfile::TempDir;

const ENV_SCENARIO: &str = "CHESS_STORE_AB_SCENARIO";
const ENV_DIR: &str = "CHESS_STORE_AB_DIR";
const ENV_PLAN: &str = "CHESS_STORE_AB_PLAN";

fn seed(s: &Store) {
    for (n, name) in [(1, "Ann"), (2, "Bob")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![computer(10, "Bot A", 1, 0, 3)])).unwrap();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
}

fn live() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    seed(&s);
    (dir, s)
}

/// A store whose backlog clock is a number the test sets.
fn live_with_clock() -> (TempDir, Store, Arc<AtomicU64>) {
    let dir = TempDir::new().unwrap();
    let now = Arc::new(AtomicU64::new(0));
    let clock = Arc::clone(&now);
    let options = StoreOptions::new(DataDir::new(dir.path()))
        .with_clock(ticking_clock(1_000))
        .with_monotonic_clock(Arc::new(move || clock.load(Ordering::SeqCst)));
    let s = block_on(Store::open(options)).unwrap().store;
    seed(&s);
    (dir, s, now)
}

/// The outer real-time limit of every `sync_backlog` call here (FIND-D3R-11): the injected
/// clock of some tests is frozen, so only real time can turn a regression into a failure.
const OUTER_LIMIT: Duration = Duration::from_secs(60);

fn sync(s: &Store, timeout: Duration) -> Result<(), StoreError> {
    block_on_within(OUTER_LIMIT, s.sync_backlog(timeout))
}

fn rec(seq: u64, payload: Payload) -> JournalRecord {
    JournalRecord {
        seq,
        wall_ms: 1_000 + i64::try_from(seq).unwrap(),
        payload,
    }
}

fn game_end() -> Payload {
    Payload::GameEnd(GameEndPayload {
        result: "1-0".into(),
        reason: EndReason::Checkmate,
        ended_at: 9_000_000,
        eco: None,
        opening_name: None,
        analysis_used: false,
        content_hash: "sha256:v1:aa".into(),
        white_time_used_ms: 1,
        black_time_used_ms: 2,
        total_active_ms: 3,
    })
}

/// The tables a game's apply changes, as text. (The registry table holds a pid and a time.)
fn content(conn: &Connection) -> Vec<String> {
    let mut out = Vec::new();
    for table in [
        "game",
        "move",
        "player",
        "rating_history",
        "unfinished_slot",
        "analysis_job",
    ] {
        let mut st = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let n = st.column_count();
        let rows: Vec<String> = st
            .query_map([], |r| {
                let mut cells = Vec::new();
                for i in 0..n {
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

fn plies(s: &Store) -> Vec<(u32, String)> {
    block_on(s.game_moves(gid(1)))
        .unwrap()
        .into_iter()
        .map(|m| (m.ply, m.uci))
        .collect()
}

// ---------------------------------------------------------------------------------- CT-ST-007

/// CT-ST-007 for `apply_records`, a fault just before the commit: the store holds exactly what
/// it held, the batch waits in the backlog (an I/O error is a transient one), and once the
/// fault is gone `sync_backlog` applies it.
#[test]
fn ct_st_007_apply_with_a_fault_before_the_commit_changes_nothing_and_the_retry_succeeds() {
    let (d, s) = live();
    let before = content(&raw(d.path()));
    let guard = FaultPlan::new()
        .errno("apply.before_commit", 5)
        .install(d.path())
        .unwrap();
    let batch = vec![rec(2, jb::mv(0)), rec(3, jb::mv(1)), rec(4, game_end())];
    let out = block_on(s.apply_records(gid(1), batch)).unwrap();
    assert_eq!(out, ApplyOutcome::Queued);
    assert_eq!(
        (
            guard.hits("apply.before_commit"),
            guard.fired("apply.before_commit")
        ),
        (1, 1)
    );
    assert_eq!(
        before,
        content(&raw(d.path())),
        "no row of any table changed"
    );
    assert_eq!(block_on(s.backlog_status()).unwrap().pending_batches, 1);
    // Still failing: a forced pass fails and the batch stays.
    let still = sync(&s, Duration::from_millis(50));
    assert!(matches!(still, Err(StoreError::Busy)), "{still:?}");
    assert_eq!(before, content(&raw(d.path())));
    drop(guard);
    sync(&s, Duration::from_secs(5)).unwrap();
    assert_eq!(plies(&s).len(), 2);
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::Completed
    );
    assert_eq!(block_on(s.get_player(pid(1))).unwrap().strict_elo, 1220);
}

/// A fault that fires once is retried by the backlog's own timer (no `sync_backlog` needed).
#[test]
fn a_fault_that_fires_once_is_retried_by_the_timer() {
    let (d, s, now) = live_with_clock();
    let _guard = FaultPlan::new()
        .errno("apply.before_commit", 5)
        .once()
        .install(d.path())
        .unwrap();
    let out = block_on(s.apply_records(gid(1), vec![rec(2, jb::mv(0))])).unwrap();
    assert_eq!(out, ApplyOutcome::Queued);
    assert!(plies(&s).is_empty());
    now.store(100, Ordering::SeqCst); // the first retry is due at 100 ms
    wait_until(|| block_on(s.backlog_status()).unwrap().pending_batches == 0);
    assert_eq!(plies(&s).len(), 1);
}

/// A fault just after the commit: the rows are there, the caller is told the attempt failed,
/// the retry is a no-op, and the rating is settled once.
#[test]
fn ct_st_007_apply_with_a_fault_after_the_commit_is_retried_without_a_second_effect() {
    let (d, s) = live();
    let guard = FaultPlan::new()
        .errno("apply.after_commit", 5)
        .install(d.path())
        .unwrap();
    let batch = vec![rec(2, jb::mv(0)), rec(3, game_end())];
    assert_eq!(
        block_on(s.apply_records(gid(1), batch)).unwrap(),
        ApplyOutcome::Queued
    );
    assert_eq!(guard.fired("apply.after_commit"), 1);
    let committed = content(&raw(d.path()));
    assert_eq!(
        plies(&s).len(),
        1,
        "the commit happened before the failpoint"
    );
    drop(guard);
    sync(&s, Duration::from_secs(5)).unwrap();
    assert_eq!(committed, content(&raw(d.path())));
    assert_eq!(
        block_on(s.get_player(pid(1))).unwrap().strict_rated_games,
        1
    );
    let ratings: i64 = raw(d.path())
        .query_row(
            "SELECT count(*) FROM rating_history WHERE game_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ratings, 2, "one row per side, not two");
}

fn wait_until(mut done: impl FnMut() -> bool) {
    let begun = Instant::now();
    while !done() {
        assert!(begun.elapsed() < Duration::from_secs(10), "timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

// ---------------------------------------------------------------------- the lock, in process

/// The database is write-locked by another connection: an apply attempt waits 100 ms, not 5 s,
/// the batch is queued, a second batch of the game queues behind it without another wait, and
/// the thread answers commands at once. Released, the backlog drains in order.
#[test]
fn a_locked_database_costs_one_short_attempt_and_the_backlog_drains_in_order() {
    let (d, s) = live();
    let other = raw(d.path());
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    let begun = Instant::now();
    let first = block_on(s.apply_records(gid(1), vec![rec(2, jb::mv(0))])).unwrap();
    let waited = begun.elapsed();
    assert_eq!(first, ApplyOutcome::Queued);
    assert!(waited >= Duration::from_millis(90), "{waited:?}");
    assert!(
        waited < Duration::from_millis(1_500),
        "never the 5 s timeout: {waited:?}"
    );
    let begun = Instant::now();
    let second = block_on(s.apply_records(gid(1), vec![rec(3, jb::mv(1))])).unwrap();
    let third = block_on(s.apply_records(gid(1), vec![rec(4, jb::mv(2))])).unwrap();
    assert_eq!(
        (second, third),
        (ApplyOutcome::Queued, ApplyOutcome::Queued)
    );
    let status = block_on(s.backlog_status()).unwrap();
    assert!(
        begun.elapsed() < Duration::from_millis(80),
        "{:?}",
        begun.elapsed()
    );
    assert_eq!((status.pending_batches, status.pending_games), (3, 1));
    assert!(!status.store_backlog);
    other.execute_batch("ROLLBACK").unwrap();
    sync(&s, Duration::from_secs(10)).unwrap();
    let rows = plies(&s);
    assert_eq!(
        rows,
        vec![
            (1, "u0".to_owned()),
            (2, "u1".to_owned()),
            (3, "u2".to_owned())
        ]
    );
    assert_eq!(block_on(s.get_game(gid(1))).unwrap().journal_seq, 4);
    assert_eq!(block_on(s.backlog_status()).unwrap().pending_batches, 0);
}

/// `sync_backlog` answers `Busy` when the backlog is not empty at its timeout (real clock).
#[test]
fn sync_backlog_is_busy_at_its_timeout_while_the_lock_is_held_and_ok_after() {
    let (d, s) = live();
    let other = raw(d.path());
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    block_on(s.apply_records(gid(1), vec![rec(2, jb::mv(0))])).unwrap();
    let begun = Instant::now();
    let out = sync(&s, Duration::from_millis(400));
    let waited = begun.elapsed();
    assert!(matches!(out, Err(StoreError::Busy)), "{out:?}");
    assert!(waited >= Duration::from_millis(350), "{waited:?}");
    assert!(waited < Duration::from_millis(3_000), "{waited:?}");
    other.execute_batch("ROLLBACK").unwrap();
    sync(&s, Duration::from_secs(10)).unwrap();
    assert_eq!(plies(&s).len(), 1);
}

/// With nothing waiting `sync_backlog` returns at once, also with a zero timeout.
#[test]
fn sync_backlog_with_an_empty_backlog_returns_at_once() {
    let (_d, s) = live();
    let begun = Instant::now();
    sync(&s, Duration::ZERO).unwrap();
    assert!(begun.elapsed() < Duration::from_millis(500));
}

// ------------------------------------------------------------------------------------ D3-09

/// A live terminal command for a game first applies that game's queued batches (D3-09, ruling
/// 2026-11-24): with the injected clock frozen, the retry timer never fires, so the only thing
/// that can apply the queued moves before the command is the store's own flush. After each of
/// `finalize_game`, `abort_game`, `void_unreadable_game` and `reset_player` the backlog is empty
/// and the rows of the queued batches are in the database.
#[test]
fn d3_09_a_live_terminal_command_applies_the_queued_batches_of_its_game_first() {
    use chess_domain::{GameResult, WallMs};
    use chess_store::{AbortRequest, FinalizeRequest, VoidSpec};
    for command in ["finalize", "abort", "void_unreadable", "reset"] {
        let (d, s, _now) = live_with_clock();
        let other = raw(d.path());
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        for (seq, ply) in [(2, 0), (3, 1)] {
            let out = block_on(s.apply_records(gid(1), vec![rec(seq, jb::mv(ply))])).unwrap();
            assert_eq!(out, ApplyOutcome::Queued, "{command}");
        }
        other.execute_batch("ROLLBACK").unwrap();
        assert!(plies(&s).is_empty(), "{command}: nothing applied yet");
        let hash = "sha256:v1:aa".to_owned();
        match command {
            "finalize" => {
                let request = FinalizeRequest {
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
                    content_hash: hash,
                    final_journal_seq: 4,
                };
                block_on(s.finalize_game(request)).unwrap();
            }
            "abort" => {
                let request = AbortRequest {
                    game: gid(1),
                    ended_at: WallMs(9_000_000),
                    content_hash: hash,
                    final_journal_seq: 4,
                };
                block_on(s.abort_game(request)).unwrap();
            }
            "void_unreadable" => {
                block_on(s.void_unreadable_game(gid(1), hash)).unwrap();
            }
            _ => block_on(s.reset_player(
                pid(1),
                vec![VoidSpec {
                    game: gid(1),
                    content_hash: hash,
                }],
                "{}".into(),
            ))
            .unwrap(),
        }
        assert_eq!(
            block_on(s.backlog_status()).unwrap().pending_batches,
            0,
            "{command}: the queue was flushed first"
        );
        assert_eq!(
            plies(&s).len(),
            2,
            "{command}: the queued moves are in the database"
        );
        assert_ne!(
            block_on(s.get_game(gid(1))).unwrap().status,
            GameStatus::InProgress,
            "{command}"
        );
    }
}

/// When the queue cannot be applied (the lock is still held) the live command answers `Busy`
/// and does not run: the game is as it was. For a parked game it answers `GameParked`.
#[test]
fn d3_09_a_live_command_is_refused_while_the_queue_cannot_drain_or_the_game_is_parked() {
    let (d, s, _now) = live_with_clock();
    let other = raw(d.path());
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    block_on(s.apply_records(gid(1), vec![rec(2, jb::mv(0))])).unwrap();
    let out = block_on(s.void_unreadable_game(gid(1), "sha256:v1:aa".into()));
    assert!(matches!(out, Err(StoreError::Busy)), "{out:?}");
    other.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::InProgress
    );
    assert_eq!(block_on(s.backlog_status()).unwrap().pending_batches, 1);
    sync(&s, Duration::from_secs(5)).unwrap();

    let Payload::Move(mut half) = jb::mv(1) else {
        panic!()
    };
    half.black_ms = None;
    assert!(block_on(s.apply_records(gid(1), vec![rec(3, Payload::Move(half))])).is_err());
    let out = block_on(s.void_unreadable_game(gid(1), "sha256:v1:aa".into()));
    assert!(matches!(out, Err(StoreError::GameParked { .. })), "{out:?}");
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::InProgress
    );
}

/// FIND-D3R-04 end to end on the store: after a game is parked, a batch of another game that
/// has to wait starts a fresh 0.1 s timer (it used to inherit the due timer and the 5 s delay).
/// The injected clock decides: nothing is attempted at 99 ms after the push, one attempt at 100.
#[test]
fn a_game_parked_earlier_does_not_leave_a_stale_timer_to_the_next_backlog() {
    let (d, s, now) = live_with_clock();
    block_on(s.start_game(new_game(
        2,
        pid(1),
        pid(10),
        Matchup::HumanComputer,
        Mode::Training,
    )))
    .unwrap();
    let Payload::Move(mut half) = jb::mv(0) else {
        panic!()
    };
    half.black_ms = None;
    assert!(block_on(s.apply_records(gid(1), vec![rec(2, Payload::Move(half))])).is_err());
    now.store(72_000, Ordering::SeqCst);
    let other = raw(d.path());
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    let out = block_on(s.apply_records(gid(2), vec![rec(2, jb::mv(0))])).unwrap();
    assert_eq!(out, ApplyOutcome::Queued);
    other.execute_batch("ROLLBACK").unwrap();
    // Not due 99 ms after the push; the stale timer would have been due at once.
    now.store(72_099, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(block_on(s.backlog_status()).unwrap().pending_batches, 2);
    now.store(72_100, Ordering::SeqCst);
    wait_until(|| block_on(s.backlog_status()).unwrap().pending_batches == 1);
}

// ---------------------------------------------------------------------- CT-ST-008, two processes

fn lock_child(dir: &Path) -> Spawned {
    child::spawn(
        "backlog_child_entry",
        &[
            (ENV_SCENARIO, "lock"),
            (ENV_DIR, dir.to_str().unwrap()),
            (ENV_PLAN, ""),
        ],
    )
}

/// CT-ST-008: another **process** holds the database lock. Play continues from the journal (the
/// session appends and syncs records; each `apply_records` is queued, none blocks play), the
/// backlog is reported as `StoreBacklog` once its oldest batch is 30 s old (injected clock),
/// and after the other process lets go the backlog drains by the timer and the rows equal the
/// journal.
#[test]
fn ct_st_008_a_database_locked_by_another_process_queues_play_and_drains_afterwards() {
    let (d, s, now) = live_with_clock();
    let journal = s.journal().unwrap();
    let writer = journal.create(&gid(1)).unwrap();
    let mut other = lock_child(d.path());
    other.expect("LOCKED");
    // Play: three moves. Each is journaled (the durability point), then handed to the store.
    for ply in 0..3u32 {
        let seq = block_on(writer.append(1_000 + i64::from(ply), jb::mv(ply))).unwrap();
        let begun = Instant::now();
        let out = block_on(s.apply_records(gid(1), vec![rec(seq, jb::mv(ply))])).unwrap();
        assert_eq!(out, ApplyOutcome::Queued, "ply {ply}");
        assert!(
            begun.elapsed() < Duration::from_millis(1_500),
            "play was held up"
        );
    }
    assert!(plies(&s).is_empty(), "nothing is in the database yet");
    // 29.9 s: not yet reported. 30 s: reported.
    now.store(29_999, Ordering::SeqCst);
    assert!(!block_on(s.backlog_status()).unwrap().store_backlog);
    now.store(30_000, Ordering::SeqCst);
    let status = block_on(s.backlog_status()).unwrap();
    assert!(status.store_backlog);
    assert_eq!(status.pending_batches, 3);
    // The other process lets go. The timer (injected clock, 5 s on) drains the backlog.
    other.stdin.take();
    other.expect("RELEASED");
    now.store(36_000, Ordering::SeqCst);
    wait_until(|| block_on(s.backlog_status()).unwrap().pending_batches == 0);
    assert!(!block_on(s.backlog_status()).unwrap().store_backlog);
    // The rows are the journal's.
    let scan = scan_file(
        &journal.path(&gid(1)),
        gid(1).as_uuid(),
        s.journal().unwrap().store_id(),
    )
    .unwrap();
    assert_eq!(scan.records.len(), 3);
    let folded = chess_store::journal::fold(&scan.records);
    let rows = plies(&s);
    assert_eq!(rows.len(), folded.moves.len());
    for (ply, uci) in rows {
        assert_eq!(folded.moves[&ply].uci, uci);
    }
    other.finish();
}

// ------------------------------------------------------------------------------- the children

#[test]
fn backlog_child_entry() {
    let Ok(scenario) = std::env::var(ENV_SCENARIO) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(ENV_DIR).unwrap());
    match scenario.as_str() {
        "lock" => {
            let conn = Connection::open(dir.join("chess.db")).unwrap();
            conn.busy_timeout(Duration::from_secs(5)).unwrap();
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            println!("CHILD LOCKED");
            child::wait_for_stdin_eof();
            conn.execute_batch("ROLLBACK").unwrap();
            println!("CHILD RELEASED");
        }
        "apply" => {
            let plan = FaultPlan::parse(&std::env::var(ENV_PLAN).unwrap()).unwrap();
            let _guard = plan.install(&dir).unwrap();
            let s = open_in(&dir).store;
            let batch = vec![rec(2, jb::mv(0)), rec(3, jb::mv(1)), rec(4, game_end())];
            println!("CHILD APPLYING");
            let _ = block_on(s.apply_records(gid(1), batch));
            println!("CHILD RETURNED");
        }
        other => panic!("unknown scenario {other}"),
    }
}

fn apply_child(dir: &Path, plan: &str) -> Spawned {
    child::spawn(
        "backlog_child_entry",
        &[
            (ENV_SCENARIO, "apply"),
            (ENV_DIR, dir.to_str().unwrap()),
            (ENV_PLAN, plan),
        ],
    )
}

/// CT-ST-007 / kill: SIGKILL a child at `apply.before_commit` and at `apply.after_commit`. The
/// directory opens; before the commit the store holds what it held, after it the batch is
/// there; applying the same batch again reaches the same state and settles the rating once.
#[test]
fn a_process_killed_at_the_apply_failpoints_leaves_a_store_that_applies_the_batch_once() {
    let mut finals = Vec::new();
    for (point, committed) in [("apply.before_commit", false), ("apply.after_commit", true)] {
        let dir = TempDir::new().unwrap();
        {
            let s = open_in(dir.path()).store;
            seed(&s);
        }
        let before = content(&raw(dir.path()));
        let mut child = apply_child(dir.path(), &format!("{point}=kill"));
        let at = child
            .killed_at(Duration::from_secs(60))
            .unwrap_or_else(|| panic!("never reached {point}"));
        assert_eq!(at, point);
        child.kill();
        let reopened = open_in(dir.path()).store;
        assert_eq!(plies(&reopened).is_empty(), !committed, "{point}");
        if !committed {
            assert_eq!(
                before,
                content(&raw(dir.path())),
                "{point}: nothing changed"
            );
        }
        let batch = vec![rec(2, jb::mv(0)), rec(3, jb::mv(1)), rec(4, game_end())];
        let again = block_on(reopened.apply_records(gid(1), batch.clone())).unwrap();
        assert_eq!(again, ApplyOutcome::Applied, "{point}");
        let state = content(&raw(dir.path()));
        block_on(reopened.apply_records(gid(1), batch)).unwrap();
        assert_eq!(
            state,
            content(&raw(dir.path())),
            "{point}: a third time changes nothing"
        );
        assert_eq!(
            block_on(reopened.get_player(pid(1)))
                .unwrap()
                .strict_rated_games,
            1,
            "{point}"
        );
        finals.push(state);
    }
    assert_eq!(
        finals[0], finals[1],
        "both kill points end in the same state"
    );
}
