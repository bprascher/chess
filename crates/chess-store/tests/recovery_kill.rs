//! Part D4 kill tests with real child processes (feature `test-support`): a child is killed at a
//! failpoint, the parent restarts the store (`Store::open`, which runs the startup recovery)
//! and checks what is on disk.
//!
//! - CT-ST-003 (every `journal.*` failpoint, and the kill between the last `Move` and the
//!   `GameEnd` for each terminal reason), with the real store: every acknowledged move is present
//!   after the restart and the resume, and the clocks equal the last recorded ones.
//! - CT-ST-009 (kill between the journal `GameEnd` and the finalize commit; and after the commit
//!   and before the journal moves to `done/`): startup finalizes once.
//! - CT-ST-088 (six kill points of the rebase through the real recovery): after the restart
//!   exactly one `damaged/<uuid>.<k>.jnl`, no `.rebase`, no file under two names, the final
//!   journal equals the uninterrupted result, `journal_seq` equals its last `seq`, and a second
//!   restart changes nothing.
//! - B-R03 (kill between the link and the removal of `close_final`).
//!
//! `CHESS_STORE_KILL_ROUNDS` sets the rounds per failpoint (default 3; the evidence run uses 50,
//! which is 300 rounds for each of the two families).
#![cfg(feature = "test-support")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d4_support;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chess_domain::GameStatus;
use chess_store::DataDir;
use chess_store::Store;
use chess_store::fault::FaultPlan;
use chess_store::journal::format::{EndReason, GameEndPayload, Payload};
use chess_store::journal::{RecoveryOutcome, fold, scan_file};
use common::child::{self, Spawned};
use common::{block_on, gid, jb, options};
use d4_support::{
    corrupt_record, damaged_files, done_path, dump, game_end, journal_tree, jpath, rebase_path,
    start, world, write_journal,
};
use tempfile::TempDir;
use uuid::Uuid;

const ENV_SCENARIO: &str = "CHESS_STORE_KC_SCENARIO";
const ENV_DIR: &str = "CHESS_STORE_KC_DIR";
const ENV_PLAN: &str = "CHESS_STORE_KC_PLAN";
const ENV_REASON: &str = "CHESS_STORE_KC_REASON";

fn rounds() -> usize {
    std::env::var("CHESS_STORE_KILL_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3)
}

fn store_uuid(dir: &Path) -> Uuid {
    DataDir::new(dir)
        .read_store_id()
        .unwrap()
        .unwrap()
        .store_uuid
}

fn open_store(dir: &Path) -> chess_store::OpenedStore {
    block_on(Store::open(options(dir))).unwrap()
}

const REASONS: [(&str, EndReason); 9] = [
    ("checkmate", EndReason::Checkmate),
    ("stalemate", EndReason::Stalemate),
    ("insufficient_material", EndReason::InsufficientMaterial),
    ("threefold_repetition", EndReason::ThreefoldRepetition),
    ("fifty_move_rule", EndReason::FiftyMoveRule),
    ("time_forfeit", EndReason::TimeForfeit),
    (
        "timeout_opponent_cannot_mate",
        EndReason::TimeoutOpponentCannotMate,
    ),
    ("resignation", EndReason::Resignation),
    ("draw_agreed", EndReason::DrawAgreed),
];

fn end_with(reason: EndReason) -> Payload {
    let Payload::GameEnd(end) = game_end(d4_support::ENDED) else {
        panic!()
    };
    Payload::GameEnd(GameEndPayload { reason, ..end })
}

// ------------------------------------------------------------------------------- the child

/// The child acts as a session would: journal first (`append`, acknowledged when it returns),
/// then `apply_records` to the store. A `Kill` failpoint stops it at the named point.
#[test]
fn kill_child_entry() {
    let Ok(scenario) = std::env::var(ENV_SCENARIO) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(ENV_DIR).unwrap());
    let plan = FaultPlan::parse(&std::env::var(ENV_PLAN).unwrap()).unwrap();
    let _guard = plan.install(&dir).unwrap();
    match scenario.as_str() {
        "session" => {
            let store = open_store(&dir).store;
            let writer = store.journal().unwrap().create(&gid(1)).unwrap();
            let ack = |seq: u64, what: &str| println!("CHILD ACK {seq} {what}");
            let step = |wall: i64, payload: Payload, what: &str| {
                let seq = block_on(writer.append(wall, payload.clone())).unwrap();
                ack(seq, what);
                block_on(store.apply_records(
                    gid(1),
                    vec![chess_store::journal::JournalRecord {
                        seq,
                        wall_ms: wall,
                        payload,
                    }],
                ))
                .unwrap();
            };
            step(5_001, jb::start(1), "start");
            for ply in 0..8 {
                step(10 + i64::from(ply), jb::mv(ply), "move");
            }
        }
        "terminal" => {
            let reason = std::env::var(ENV_REASON).unwrap();
            let (_, end) = REASONS.iter().find(|(n, _)| *n == reason).unwrap();
            let store = open_store(&dir).store;
            let writer = store.journal().unwrap().create(&gid(1)).unwrap();
            block_on(writer.append(5_001, jb::start(1))).unwrap();
            for ply in 0..3 {
                block_on(writer.append(10 + i64::from(ply), jb::mv(ply))).unwrap();
            }
            println!("CHILD ACK final move");
            // The kill comes before this record is written.
            let _ = block_on(writer.append(50, end_with(*end)));
        }
        "finalize" => {
            // The journal holds the GameEnd (acknowledged); the finalize is killed inside.
            let store = open_store(&dir).store;
            let writer = store.journal().unwrap().create(&gid(1)).unwrap();
            let mut records = Vec::new();
            let mut payloads = d4_support::playing(1, 3);
            payloads.push(game_end(d4_support::ENDED));
            for (i, payload) in payloads.into_iter().enumerate() {
                let seq = block_on(writer.append(100 + i64::try_from(i).unwrap(), payload.clone()))
                    .unwrap();
                records.push(chess_store::journal::JournalRecord {
                    seq,
                    wall_ms: 100 + i64::try_from(i).unwrap(),
                    payload,
                });
            }
            println!("CHILD ACK game end");
            let _ = block_on(store.apply_records(gid(1), records));
            println!("CHILD SURVIVED");
        }
        "close" => {
            let store = open_store(&dir).store;
            let writer = store.journal().unwrap().create(&gid(1)).unwrap();
            let mut records = Vec::new();
            let mut payloads = d4_support::playing(1, 2);
            payloads.push(game_end(d4_support::ENDED));
            for (i, payload) in payloads.into_iter().enumerate() {
                let wall = 100 + i64::try_from(i).unwrap();
                let seq = block_on(writer.append(wall, payload.clone())).unwrap();
                records.push(chess_store::journal::JournalRecord {
                    seq,
                    wall_ms: wall,
                    payload,
                });
            }
            block_on(store.apply_records(gid(1), records)).unwrap();
            println!("CHILD ACK finalized");
            let _ = writer.close_final();
            println!("CHILD SURVIVED");
        }
        "open" => {
            // The recovery itself runs inside `Store::open`; the plan kills it half way.
            let _store = open_store(&dir);
            println!("CHILD SURVIVED");
        }
        other => panic!("unknown scenario {other}"),
    }
}

fn run(scenario: &str, dir: &Path, plan: &str, reason: &str) -> Spawned {
    child::spawn(
        "kill_child_entry",
        &[
            (ENV_SCENARIO, scenario),
            (ENV_DIR, dir.to_str().unwrap()),
            (ENV_PLAN, plan),
            (ENV_REASON, reason),
        ],
    )
}

/// Runs the child to its kill, returns the lines it printed before.
fn kill_at(scenario: &str, dir: &Path, plan: &str, reason: &str, point: &str) -> Vec<String> {
    let mut child = run(scenario, dir, plan, reason);
    let at = child
        .killed_at(Duration::from_secs(120))
        .unwrap_or_else(|| panic!("the child never reached {point}"));
    assert_eq!(at, point);
    let mut lines = Vec::new();
    while let Some(line) = child.line(Duration::from_millis(100)) {
        lines.push(line);
    }
    child.kill();
    lines
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn status(s: &Store, n: u128) -> GameStatus {
    block_on(s.get_game(gid(n))).unwrap().status
}

// -------------------------------------------------------------------------------- CT-ST-003

/// CT-ST-003: kill at each `journal.*` failpoint of one append (the first to the ninth); restart;
/// every acknowledged move is present, the clocks equal the last recorded, the game resumes and
/// the writer appends again. `journal_seq` never exceeds the journal.
#[test]
fn ct_st_003_a_kill_at_each_journal_failpoint_loses_no_acknowledged_move() {
    let points = [
        "journal.before_write",
        "journal.partial_write",
        "journal.after_write",
        "journal.before_fsync",
        "journal.fsync_error",
        "journal.after_fsync",
    ];
    for point in points {
        for round in 0..rounds() {
            // The append that is killed: 0 is the `Start`, 1 the first move, ... (hits are
            // counted per append, a skip of n kills the (n+1)-th).
            let skip = round % 9;
            let (dir, s) = world();
            start(&s, 1);
            drop(s);
            let lines = kill_at(
                "session",
                dir.path(),
                &format!("{point}=kill@{skip}"),
                "",
                point,
            );
            let acked_moves = lines
                .iter()
                .filter(|l| l.starts_with("ACK ") && l.ends_with(" move"))
                .count();
            let context = format!("{point} round {round} (skip {skip})");
            // The restart: startup recovery, then the resume.
            let opened = open_store(dir.path());
            let s = opened.store;
            let journal = s.journal().unwrap();
            let (writer, recovered) = block_on(journal.open_for_resume(&s, &gid(1))).unwrap();
            let writer = writer.unwrap();
            let rows = block_on(s.game_moves(gid(1))).unwrap();
            assert!(
                rows.len() >= acked_moves,
                "{context}: {} < {acked_moves}",
                rows.len()
            );
            assert_eq!(rows.len(), recovered.moves.len(), "{context}");
            assert_eq!(recovered.report.damaged, None, "{context}");
            // Each stored move is the one the journal holds, in order.
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(row.uci, format!("u{i}"), "{context}");
            }
            // The clocks equal the last recorded ones (the last Move record that is present).
            let scan = scan_file(
                &jpath(dir.path(), 1),
                Uuid::from_u128(1),
                store_uuid(dir.path()),
            )
            .unwrap();
            assert!(
                scan.damaged.is_none() && !scan.torn_tail_dropped,
                "{context}"
            );
            let folded = fold(&scan.records);
            assert_eq!(recovered.clocks, folded.clocks.unwrap_or(None), "{context}");
            if let Some(last) = rows.last() {
                assert_eq!(
                    recovered.clocks, last.clocks_ms,
                    "{context}: the clocks of the last recorded move"
                );
            }
            let g = block_on(s.get_game(gid(1))).unwrap();
            assert!(g.journal_seq <= scan.last_seq(), "{context}");
            // The writer continues where the journal ends.
            assert_eq!(writer.next_seq(), scan.last_seq() + 1, "{context}");
            // A kill before the Start leaves a file with a header only (or a torn Start):
            // there the row has no moves at all.
            if skip == 0 {
                assert!(rows.is_empty(), "{context}");
            }
        }
    }
}

/// CT-ST-003, Amd 8 (D-65): a kill between the `Move` record and the `GameEnd` record for each
/// terminal reason. The journal ends with the `Move`; the restart and the scan return that state
/// unchanged and apply no `GameEnd`.
#[test]
fn ct_st_003_a_kill_between_the_move_and_the_game_end_applies_no_game_end() {
    for (reason, _) in REASONS {
        let (dir, s) = world();
        start(&s, 1);
        drop(s);
        // Start and three moves pass; the fifth append (the GameEnd) is killed before it writes.
        kill_at(
            "terminal",
            dir.path(),
            "journal.before_write=kill@4",
            reason,
            "journal.before_write",
        );
        let opened = open_store(dir.path());
        assert_eq!(
            opened.report.recovery.outcome_of(gid(1)),
            Some(&RecoveryOutcome::Reconciled { recovered_moves: 3 }),
            "{reason}"
        );
        let s = opened.store;
        assert_eq!(status(&s, 1), GameStatus::InProgress, "{reason}");
        let scan = scan_file(
            &jpath(dir.path(), 1),
            Uuid::from_u128(1),
            store_uuid(dir.path()),
        )
        .unwrap();
        assert!(
            matches!(
                scan.records.last().map(|r| &r.payload),
                Some(Payload::Move(_))
            ),
            "{reason}: the journal ends with the Move"
        );
        assert_eq!(block_on(s.game_moves(gid(1))).unwrap().len(), 3, "{reason}");
    }
}

// -------------------------------------------------------------------------------- CT-ST-009

/// Ann's strict rating and rated-game counter (the sum over both players is zero-sum, so one
/// player is read).
fn elo_sum(dir: &Path) -> (i64, i64) {
    common::raw(dir)
        .query_row(
            "SELECT strict_elo, strict_rated_games FROM player WHERE uuid LIKE '%0001'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

/// CT-ST-009: the journal holds the `GameEnd` (acknowledged), the finalize transaction is killed
/// before its commit: startup finalizes once. Killed after the commit and before the journal
/// moves to `done/`: startup closes the journal and changes no rating.
#[test]
fn ct_st_009_a_kill_between_the_game_end_and_the_finalize_commit_finalizes_once() {
    for round in 0..rounds().max(2) {
        // Before the commit: the row is still in progress.
        let (dir, s) = world();
        start(&s, 1);
        drop(s);
        let before = elo_sum(dir.path());
        kill_at(
            "finalize",
            dir.path(),
            "apply.before_commit=kill",
            "",
            "apply.before_commit",
        );
        assert_eq!(
            elo_sum(dir.path()),
            before,
            "round {round}: nothing committed"
        );
        let opened = open_store(dir.path());
        assert_eq!(
            opened.report.recovery.outcome_of(gid(1)),
            Some(&RecoveryOutcome::Finalized),
            "round {round}"
        );
        assert_eq!(status(&opened.store, 1), GameStatus::Completed);
        let once = dump(dir.path());
        let settled = elo_sum(dir.path());
        drop(opened);
        let opened = open_store(dir.path());
        assert!(opened.report.recovery.games.is_empty());
        assert_eq!(
            dump(dir.path()),
            once,
            "round {round}: a second start changes nothing"
        );
        assert_eq!(elo_sum(dir.path()), settled);
        assert!(done_path(dir.path(), 1).exists());
        drop(opened);

        // After the commit: the rating is settled once and the restart does not settle it again.
        let (dir, s) = world();
        start(&s, 1);
        drop(s);
        kill_at(
            "finalize",
            dir.path(),
            "apply.after_commit=kill",
            "",
            "apply.after_commit",
        );
        let committed = elo_sum(dir.path());
        assert_ne!(committed, before, "round {round}: the finalize committed");
        let opened = open_store(dir.path());
        assert_eq!(
            opened.report.recovery.outcome_of(gid(1)),
            Some(&RecoveryOutcome::ClosedLeftover),
            "round {round}"
        );
        assert_eq!(
            elo_sum(dir.path()),
            committed,
            "round {round}: settled once"
        );
        assert!(done_path(dir.path(), 1).exists() && !jpath(dir.path(), 1).exists());
    }
}

/// B-R03: a kill between the link and the removal of `close_final` leaves the journal under two
/// names; the restart removes the old name and reports it, once; a second start changes nothing.
#[test]
fn b_r03_a_kill_between_the_link_and_the_removal_is_repaired_at_startup() {
    for round in 0..rounds().max(2) {
        let (dir, s) = world();
        start(&s, 1);
        drop(s);
        kill_at(
            "close",
            dir.path(),
            "journal.close_final.linked=kill",
            "",
            "journal.close_final.linked",
        );
        assert_eq!(
            fs::metadata(jpath(dir.path(), 1)).unwrap().nlink(),
            2,
            "round {round}"
        );
        let opened = open_store(dir.path());
        assert_eq!(
            opened.report.recovery.outcome_of(gid(1)),
            Some(&RecoveryOutcome::FinishedClose),
            "round {round}"
        );
        assert!(!jpath(dir.path(), 1).exists());
        assert_eq!(fs::metadata(done_path(dir.path(), 1)).unwrap().nlink(), 1);
        let tree = journal_tree(dir.path());
        drop(opened);
        let opened = open_store(dir.path());
        assert!(opened.report.recovery.games.is_empty());
        assert_eq!(tree, journal_tree(dir.path()));
        assert_eq!(status(&opened.store, 1), GameStatus::Completed);
    }
}

// -------------------------------------------------------------------------------- CT-ST-088

/// A store whose game 1 has four moves in the database and a journal damaged in the middle.
/// Returns the directory and the bytes of the damaged file.
fn damaged_fixture() -> (TempDir, Vec<u8>) {
    let (dir, s) = world();
    start(&s, 1);
    let records = write_journal(&s, 1, d4_support::playing(1, 4));
    d4_support::apply(&s, 1, records);
    drop(s);
    let original = corrupt_record(
        &jpath(dir.path(), 1),
        3,
        store_uuid(dir.path()).as_u128(),
        1,
    );
    (dir, original)
}

fn assert_no_file_under_two_names(dir: &Path, context: &str) {
    for (name, _, links) in journal_tree(dir) {
        assert_eq!(links, 1, "{context}: {name} has {links} names");
    }
}

fn assert_rebase_result(dir: &Path, original: &[u8], control: &[u8], context: &str) {
    let aside = damaged_files(dir, 1, ".jnl");
    assert_eq!(
        aside.len(),
        1,
        "{context}: exactly one damaged file: {aside:?}"
    );
    assert_eq!(
        fs::read(&aside[0]).unwrap(),
        original,
        "{context}: the old bytes are kept"
    );
    assert!(!rebase_path(dir, 1).exists(), "{context}: no .rebase");
    assert!(damaged_files(dir, 1, ".rebase").is_empty(), "{context}");
    assert_no_file_under_two_names(dir, context);
    let bytes = fs::read(jpath(dir, 1)).unwrap();
    assert_eq!(
        bytes, control,
        "{context}: the final journal equals the uninterrupted result"
    );
    let scan = scan_file(&jpath(dir, 1), Uuid::from_u128(1), store_uuid(dir)).unwrap();
    assert!(
        scan.damaged.is_none() && !scan.torn_tail_dropped,
        "{context}"
    );
    let seq: i64 = common::raw(dir)
        .query_row(
            "SELECT journal_seq FROM game WHERE uuid LIKE '%0001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        u64::try_from(seq).unwrap(),
        scan.last_seq(),
        "{context}: journal_seq"
    );
}

/// CT-ST-088: kill at the six rebase failpoints inside the real recovery, restart, and check the
/// invariants; a second restart changes nothing.
#[test]
fn ct_st_088_a_kill_at_each_of_the_six_rebase_failpoints_is_repaired() {
    let points = [
        "journal.rebase.built",
        "journal.rebase.db_committed",
        "journal.rebase.old_linked",
        "journal.rebase.old_aside",
        "journal.rebase.new_linked",
        "journal.rebase.new_in_place",
    ];
    // The uninterrupted result, once: the same inputs always give the same bytes.
    let (pristine, original) = damaged_fixture();
    for point in points {
        for round in 0..rounds() {
            let context = format!("{point} round {round}");
            let dir = TempDir::new().unwrap();
            copy_dir(pristine.path(), dir.path());
            let control_dir = TempDir::new().unwrap();
            copy_dir(pristine.path(), control_dir.path());
            drop(open_store(control_dir.path()));
            let control = fs::read(jpath(control_dir.path(), 1)).unwrap();
            let lines = kill_at("open", dir.path(), &format!("{point}=kill"), "", point);
            assert!(!lines.iter().any(|l| l.contains("SURVIVED")), "{context}");
            // The restart repairs the state the kill left.
            let opened = open_store(dir.path());
            assert_rebase_result(dir.path(), &original, &control, &context);
            let tree = journal_tree(dir.path());
            let snapshot = dump(dir.path());
            drop(opened);
            // A second restart changes nothing.
            drop(open_store(dir.path()));
            assert_eq!(
                tree,
                journal_tree(dir.path()),
                "{context}: second restart, files"
            );
            assert_eq!(
                snapshot,
                dump(dir.path()),
                "{context}: second restart, database"
            );
        }
    }
}

/// The same kill points, with the game resumed by `open_for_resume` instead of the startup scan:
/// the interrupted rebase is finished by the first process that takes the lock. The scan skips
/// the game while a lock is held on its journal (the lock stands in for a live session), and the
/// resume comes after the lock is released.
#[test]
fn ct_st_088_the_resume_finishes_a_rebase_that_the_scan_did_not() {
    let (pristine, original) = damaged_fixture();
    let control_dir = TempDir::new().unwrap();
    copy_dir(pristine.path(), control_dir.path());
    drop(open_store(control_dir.path()));
    let control = fs::read(jpath(control_dir.path(), 1)).unwrap();
    for point in [
        "journal.rebase.db_committed",
        "journal.rebase.old_linked",
        "journal.rebase.new_linked",
    ] {
        let dir = TempDir::new().unwrap();
        copy_dir(pristine.path(), dir.path());
        kill_at("open", dir.path(), &format!("{point}=kill"), "", point);
        let held = fs::File::open(jpath(dir.path(), 1)).unwrap();
        held.try_lock().unwrap();
        let opened = open_store(dir.path());
        assert_eq!(
            opened.report.recovery.outcome_of(gid(1)),
            Some(&RecoveryOutcome::Held),
            "{point}: the scan skips a held journal"
        );
        held.unlock().unwrap();
        drop(held);
        let s = opened.store;
        let journal = s.journal().unwrap();
        let (writer, recovered) = block_on(journal.open_for_resume(&s, &gid(1))).unwrap();
        assert!(writer.is_some(), "{point}");
        assert_eq!(recovered.moves.len(), 4, "{point}");
        drop(writer);
        assert_rebase_result(dir.path(), &original, &control, point);
    }
}
