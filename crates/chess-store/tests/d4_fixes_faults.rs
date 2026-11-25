//! Part D4 review fixes that need the `test-support` seams (feature `test-support`):
//!
//! - FIND-D4R-01 and -02: two resumers of a game that has **no journal file**. The seam
//!   `journal.resume.no_file_seen` widens the window between "there is no file" and the rebase;
//!   the same race is run between two real processes.
//! - FIND-D4R-07: the startup scan's total wait for a database that another process holds.
//! - FIND-D4R-09: the grace of B-R04 and "a `create` that loses to a holder leaves the file",
//!   made certain by the seam `journal.create.created` (the earlier tests rely on chance).
//! - sync faults on the B-R03 repair at startup and on the aside moves (mutants D04, F13).
//!
//! `CHESS_STORE_D4R_ROUNDS` sets the rounds of the two-process race (default 30).
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
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chess_domain::{Matchup, Mode};
use chess_store::fault::FaultPlan;
use chess_store::journal::format::SeatKind;
use chess_store::journal::{
    JournalWriter, LeaseState, MoveJournal, Recovered, RecoveryOutcome, ResumeError, Settled,
    move_aside_damaged, scan_file, settle_rebase_files,
};
use chess_store::{DataDir, JournalError, Store};
use common::child;
use common::{block_on, gid, human, jb, new_game, options, pid, raw};
use d4_support::{
    ENDED, apply, corrupt_record, damaged_files, done_path, game_end, jdir, jpath, live, playing,
    rebase_path, reopen, start_for, world, write_journal,
};
use uuid::Uuid;

const EIO: i32 = 5;
const ENV_DIR: &str = "CHESS_STORE_D4R_DIR";
const NO_FILE_SEEN: &str = "journal.resume.no_file_seen";

type Resumed = Result<(Option<JournalWriter>, Recovered), ResumeError>;

fn store_uuid(dir: &Path) -> Uuid {
    DataDir::new(dir)
        .read_store_id()
        .unwrap()
        .unwrap()
        .store_uuid
}

fn resume(s: &Store, n: u128) -> Resumed {
    let journal = s.journal().unwrap();
    block_on(journal.open_for_resume(s, &gid(n)))
}

/// A live game 1 with `moves` moves in the database and no journal file.
fn row_with_no_journal(moves: u32) -> (tempfile::TempDir, Store) {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, moves));
    apply(&s, 1, records);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    (d, s)
}

fn is_locked<T>(result: &Result<T, ResumeError>) -> bool {
    matches!(
        result,
        Err(ResumeError::Journal(JournalError::Locked { .. }))
    )
}

// ================================================================== FIND-D4R-01 (the seam)

/// The reviewer's repro without a sleep in the library: resumer B decides "no journal file",
/// then waits (the seam; in real life: behind the store thread's backlog or a plan applier
/// wait). Resumer A resumes the same game meanwhile, rebases, and holds a live writer. B then
/// builds its `.rebase` and must **not** move A's locked journal into `damaged/`: it answers
/// `Locked` (never `WriteFailed`), leaves nothing of its own, and A's next append lands in the
/// live journal. Fails before the fix: B returned a second writer and `damaged/<uuid>.1.jnl`
/// held A's acknowledged record.
#[test]
fn d4r_01_a_second_resumer_with_no_journal_never_moves_a_live_journal() {
    let (d, s) = row_with_no_journal(3);
    // The second rule only counts: the store step of the rebase (`db_committed`) must run once,
    // for A. B stops before it (the re-check under its `.rebase`), it does not rely on the swap
    // refusing afterwards.
    let guard = FaultPlan::new()
        .delay(NO_FILE_SEEN, Duration::from_millis(1_500))
        .once()
        .delay("journal.rebase.db_committed", Duration::ZERO)
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let (b, writer_a) = std::thread::scope(|scope| {
        let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
        // B has passed `prepare` and sleeps in the seam; A takes the whole path.
        std::thread::sleep(Duration::from_millis(400));
        let (writer, recovered) = resume(&s, 1).unwrap();
        let writer = writer.expect("A resumes the game");
        assert!(recovered.report.rebased);
        let seq = block_on(writer.append(9_000, jb::clock(1, 2))).unwrap();
        assert_eq!(seq, 6, "after Start, three moves and the closing Clock");
        (b.join().unwrap(), writer)
    });
    assert!(
        is_locked(&b),
        "B must be told Locked: {:?}",
        b.as_ref().err()
    );
    assert_eq!(
        guard.hits("journal.rebase.db_committed"),
        1,
        "B must not run step 2"
    );
    // A's journal is where it was: no `damaged/` file, no `.rebase`, one writer.
    assert!(damaged_files(d.path(), 1, "").is_empty());
    assert!(!rebase_path(d.path(), 1).exists());
    assert_eq!(fs::metadata(jpath(d.path(), 1)).unwrap().nlink(), 1);
    let seq = block_on(writer_a.append(9_001, jb::clock(3, 4))).unwrap();
    assert_eq!(seq, 7);
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        store_uuid(d.path()),
    )
    .unwrap();
    assert_eq!(
        scan.records.len(),
        7,
        "both of A's appends are in the live file"
    );
    assert!(scan.damaged.is_none());
}

/// The same window for a row that holds nothing: the resumer that decided "no file" meets a
/// journal that another resumer created. `create` answers `AlreadyExists`; the second attempt
/// finds the file, takes its lock and is told `Locked`.
#[test]
fn d4r_01_a_second_resumer_of_an_empty_row_is_told_locked() {
    let (d, s) = live();
    let _guard = FaultPlan::new()
        .delay(NO_FILE_SEEN, Duration::from_millis(1_000))
        .once()
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let b = std::thread::scope(|scope| {
        let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
        std::thread::sleep(Duration::from_millis(300));
        let (writer, recovered) = resume(&s, 1).unwrap();
        assert!(recovered.report.created_empty);
        let b = b.join().unwrap();
        drop(writer);
        b
    });
    assert!(is_locked(&b), "{:?}", b.as_ref().err());
}

/// The last line of defence, tested alone: B passes its re-check (no journal yet), is held at
/// the point after its database step, and meanwhile a live writer creates the journal. B's swap
/// must not move that journal aside (mutants M02, M07): `Locked`, the live file in place.
#[test]
fn d4r_01_the_swap_leaves_a_journal_that_appeared_after_the_recheck() {
    let (d, s) = row_with_no_journal(3);
    let _guard = FaultPlan::new()
        .delay("journal.rebase.db_committed", Duration::from_millis(1_500))
        .once()
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let (b, writer) = std::thread::scope(|scope| {
        let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
        // B has built `.rebase`, committed step 2 and waits; the journal does not exist yet.
        std::thread::sleep(Duration::from_millis(600));
        assert!(!jpath(d.path(), 1).exists() && rebase_path(d.path(), 1).exists());
        let writer = journal.create(&gid(1)).unwrap();
        block_on(writer.append(5_001, jb::start(1))).unwrap();
        (b.join().unwrap(), writer)
    });
    assert!(is_locked(&b), "{:?}", b.as_ref().err());
    assert!(
        damaged_files(d.path(), 1, "").is_empty(),
        "the live journal stays"
    );
    assert!(
        !rebase_path(d.path(), 1).exists(),
        "B took its `.rebase` away"
    );
    assert_eq!(block_on(writer.append(5_002, jb::mv(0))).unwrap(), 2);
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        store_uuid(d.path()),
    )
    .unwrap();
    assert_eq!(scan.records.len(), 2);
}

/// The same, one step later: the journal appears after the swap looked for it and before it
/// links the new file in. The no-replace link refuses (`AlreadyExists`) and B answers `Locked`
/// (mutant M07), taking its `.rebase` away.
#[test]
fn d4r_01_the_link_of_the_new_journal_never_replaces_a_journal_that_just_appeared() {
    let (d, s) = row_with_no_journal(3);
    let _guard = FaultPlan::new()
        .delay("journal.rebase.old_aside", Duration::from_millis(1_500))
        .once()
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let (b, writer) = std::thread::scope(|scope| {
        let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
        std::thread::sleep(Duration::from_millis(600));
        let writer = journal.create(&gid(1)).unwrap();
        block_on(writer.append(5_001, jb::start(1))).unwrap();
        (b.join().unwrap(), writer)
    });
    assert!(is_locked(&b), "{:?}", b.as_ref().err());
    assert!(!rebase_path(d.path(), 1).exists());
    assert!(damaged_files(d.path(), 1, "").is_empty());
    assert_eq!(block_on(writer.append(5_002, jb::mv(0))).unwrap(), 2);
}

/// A `.rebase` that another process is building (it holds the lock) appears after B decided
/// "no file": B leaves it exactly as it is and says `Locked` (mutant M03: the file steps ran
/// without a lock and moved it aside as "incomplete").
#[test]
fn d4r_01_a_rebase_in_flight_is_not_touched_by_a_resumer_without_a_lock() {
    let (d, s) = row_with_no_journal(3);
    let _guard = FaultPlan::new()
        .delay(NO_FILE_SEEN, Duration::from_millis(1_000))
        .once()
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let b = std::thread::scope(|scope| {
        let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
        std::thread::sleep(Duration::from_millis(300));
        // Another resumer: its `.rebase`, created and locked, half written.
        fs::write(rebase_path(d.path(), 1), b"half a rebase").unwrap();
        let mine = fs::File::open(rebase_path(d.path(), 1)).unwrap();
        mine.try_lock().unwrap();
        let b = b.join().unwrap();
        drop(mine);
        b
    });
    assert!(is_locked(&b), "{:?}", b.as_ref().err());
    assert_eq!(
        fs::read(rebase_path(d.path(), 1)).unwrap(),
        b"half a rebase"
    );
    assert!(damaged_files(d.path(), 1, "").is_empty());
    assert!(!jpath(d.path(), 1).exists());
}

/// An empty row: the other resumer created the header-only journal and let go again before B
/// tried to create it. B's `create` meets `AlreadyExists`, starts again, finds the file, locks
/// it and resumes on it (mutant M04: it answered a write failure).
#[test]
fn d4r_01_a_resumer_that_meets_a_created_but_free_journal_resumes_on_it() {
    let (d, s) = live();
    let _guard = FaultPlan::new()
        .delay(NO_FILE_SEEN, Duration::from_millis(1_000))
        .once()
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let b = std::thread::scope(|scope| {
        let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
        std::thread::sleep(Duration::from_millis(300));
        drop(journal.create(&gid(1)).unwrap());
        b.join().unwrap()
    });
    let (writer, recovered) = b.expect("B resumes on the file that exists");
    assert!(writer.is_some());
    assert!(
        !recovered.report.created_empty,
        "the file was not B's creation"
    );
    assert!(damaged_files(d.path(), 1, "").is_empty());
}

/// The case of the review's `rv_two_resume_nojournal`: no seam, two resumers together; the
/// loser says `Locked` and not `WriteFailed` (FIND-D4R-02). Fails before the fix.
#[test]
fn d4r_02_the_loser_of_two_resumers_is_locked_not_write_failed() {
    for round in 0..10 {
        let (d, s) = row_with_no_journal(3);
        let journal = s.journal().unwrap();
        let (a, b) = std::thread::scope(|scope| {
            let a = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
            let b = scope.spawn(|| block_on(journal.open_for_resume(&s, &gid(1))));
            (a.join().unwrap(), b.join().unwrap())
        });
        let (winner, loser) = match (a.is_ok(), b.is_ok()) {
            (true, false) => (a, b),
            (false, true) => (b, a),
            other => panic!("round {round}: exactly one must win: {other:?}"),
        };
        assert!(is_locked(&loser), "round {round}: {:?}", loser.err());
        assert!(
            damaged_files(d.path(), 1, "").is_empty(),
            "round {round}: nothing was moved aside"
        );
        drop(winner);
    }
}

// ----------------------------------------------------------------- two real processes

#[test]
fn d4r_race_child() {
    let Ok(dir) = std::env::var(ENV_DIR) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let store = block_on(Store::open(options(&dir))).unwrap().store;
    let journal = store.journal().unwrap();
    let mut held: Option<JournalWriter> = None;
    println!("CHILD READY");
    loop {
        let line = child::read_stdin_line();
        let mut words = line.split_whitespace();
        match words.next() {
            Some("go") => {
                let ms: u64 = words.next().unwrap().parse().unwrap();
                let guard = (ms > 0).then(|| {
                    FaultPlan::new()
                        .delay(NO_FILE_SEEN, Duration::from_millis(ms))
                        .once()
                        .install(&dir)
                        .unwrap()
                });
                let result = block_on(journal.open_for_resume(&store, &gid(1)));
                drop(guard);
                match result {
                    Ok((writer, _)) => {
                        held = writer;
                        println!("CHILD RESULT ok");
                    }
                    Err(ResumeError::Journal(JournalError::Locked { .. })) => {
                        println!("CHILD RESULT locked");
                    }
                    Err(other) => println!("CHILD RESULT other {other:?}"),
                }
            }
            Some("drop") => {
                held = None;
                println!("CHILD DROPPED");
            }
            _ => break,
        }
    }
    drop(held);
}

fn rounds() -> usize {
    std::env::var("CHESS_STORE_D4R_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}

/// Two processes resume the same game, which has rows and no journal, at the same moment, with
/// the window of the seam set to different lengths in each. In every round exactly one gets the
/// writer and the other is told `Locked`; no file lands in `damaged/`, no `.rebase` stays, the
/// live journal scans clean and has one name. Fails before the fix (the loser: `WriteFailed`).
#[test]
fn d4r_01_two_processes_resuming_a_game_with_no_journal_never_make_two_writers() {
    let (d, s) = row_with_no_journal(3);
    let mut other = child::spawn("d4r_race_child", &[(ENV_DIR, d.path().to_str().unwrap())]);
    other.expect("READY");
    let schedule = [(0u64, 0u64), (0, 40), (40, 0), (40, 40), (90, 15), (15, 90)];
    let (mut wins_mine, mut wins_theirs) = (0, 0);
    for round in 0..rounds() {
        let (mine_ms, their_ms) = schedule[round % schedule.len()];
        other.send(&format!("go {their_ms}\n"));
        let guard = (mine_ms > 0).then(|| {
            FaultPlan::new()
                .delay(NO_FILE_SEEN, Duration::from_millis(mine_ms))
                .once()
                .install(d.path())
                .unwrap()
        });
        let mine = resume(&s, 1);
        drop(guard);
        let theirs = other.expect("RESULT");
        let theirs_ok = theirs.ends_with("ok");
        assert!(
            theirs_ok || theirs.ends_with("locked"),
            "round {round}: {theirs}"
        );
        match (&mine, theirs_ok) {
            (Ok(_), false) => wins_mine += 1,
            (Err(_), true) => {
                assert!(is_locked(&mine), "round {round}: {:?}", mine.as_ref().err());
                wins_theirs += 1;
            }
            _ => panic!(
                "round {round}: not exactly one writer: mine ok = {}, theirs = {theirs}",
                mine.is_ok()
            ),
        }
        if mine.is_err() {
            assert!(is_locked(&mine), "round {round}");
        }
        assert!(
            damaged_files(d.path(), 1, "").is_empty(),
            "round {round}: a live journal was moved aside"
        );
        assert!(!rebase_path(d.path(), 1).exists(), "round {round}");
        assert_eq!(fs::metadata(jpath(d.path(), 1)).unwrap().nlink(), 1);
        let scan = scan_file(
            &jpath(d.path(), 1),
            Uuid::from_u128(1),
            store_uuid(d.path()),
        )
        .unwrap();
        assert!(
            scan.damaged.is_none() && !scan.torn_tail_dropped,
            "round {round}"
        );
        // Release both and start the next round from "rows, no journal".
        drop(mine);
        other.send("drop\n");
        other.expect("DROPPED");
        fs::remove_file(jpath(d.path(), 1)).unwrap();
    }
    other.finish();
    eprintln!(
        "rounds {}: mine won {wins_mine}, theirs won {wins_theirs}",
        rounds()
    );
}

// ======================================================================= FIND-D4R-07 (budget)

/// Three games whose journals have a damaged middle, so each needs the plan applier.
fn three_games_with_damaged_journals() -> (tempfile::TempDir, Store) {
    let (d, s) = world();
    block_on(s.register_player(human(3, "Cy"))).unwrap();
    block_on(s.start_game(new_game(
        1,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
    block_on(s.start_game(new_game(
        2,
        pid(3),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
    block_on(s.start_game(new_game(
        3,
        pid(2),
        pid(11),
        Matchup::HumanComputer,
        Mode::Training,
    )))
    .unwrap();
    let store = store_uuid(d.path()).as_u128();
    let payloads = |n: u128| match n {
        1 => playing(1, 2),
        2 => vec![
            start_for(2, 3, 10, SeatKind::Human, SeatKind::Computer),
            jb::mv(0),
            jb::mv(1),
        ],
        _ => vec![
            start_for(3, 2, 11, SeatKind::Human, SeatKind::Computer),
            jb::mv(0),
            jb::mv(1),
        ],
    };
    for n in 1..=3 {
        let records = write_journal(&s, n, payloads(n));
        apply(&s, n, records);
        corrupt_record(&jpath(d.path(), n), 1, store, n);
    }
    (d, s)
}

/// Another process holds the write lock of the database for much longer than the budget. The old
/// scan waited 5 s per game (15 s for three). Now the **total** wait is bounded (10 s, an
/// `ESTIMATE`): the third game, which finds the budget spent, is reported `Failed` (busy) at once
/// and `Store::open` returns within the budget plus a little. Every journal stays; the next start
/// (the lock released) rebuilds all three. Fails before the fix (about 15 s).
#[test]
fn d4r_07_the_startup_scan_waits_for_a_busy_database_at_most_its_budget() {
    let (d, s) = three_games_with_damaged_journals();
    drop(s);
    let dir = d.path().to_path_buf();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder = {
        let dir = dir.clone();
        std::thread::spawn(move || {
            // The first rebase writes its `.rebase` before its database step: the cue.
            let cue = Instant::now();
            while !(1..=3).any(|n| rebase_path(&dir, n).exists()) {
                assert!(cue.elapsed() < Duration::from_secs(60), "no rebase began");
                std::thread::sleep(Duration::from_millis(2));
            }
            let conn = raw(&dir);
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(60));
            conn.execute_batch("ROLLBACK").unwrap();
        })
    };
    // Hold the first rebase after its file is built so that the holder is certain to be first.
    let _guard = FaultPlan::new()
        .delay("journal.rebase.built", Duration::from_millis(500))
        .once()
        .install(&dir)
        .unwrap();
    let started = Instant::now();
    let opened = block_on(Store::open(options(&dir))).unwrap();
    let took = started.elapsed();
    let games = &opened.report.recovery.games;
    assert_eq!(games.len(), 3, "{games:?}");
    assert!(
        took < Duration::from_millis(12_500),
        "the open waited {took:?} for a busy database"
    );
    let busy = games
        .iter()
        .filter(|g| matches!(&g.outcome, RecoveryOutcome::Failed { error } if error == "the store is busy"))
        .count();
    assert_eq!(busy, 3, "{games:?}");
    for n in 1..=3 {
        assert!(jpath(&dir, n).exists(), "game {n}: its journal stays");
        assert!(
            !rebase_path(&dir, n).exists(),
            "game {n}: no `.rebase` of this attempt stays"
        );
        assert!(damaged_files(&dir, n, "").is_empty(), "game {n}");
    }
    release_tx.send(()).unwrap();
    holder.join().unwrap();
    drop(_guard);
    // The next start finds the lock free and recovers all three.
    let again = reopen(&dir, opened.store);
    for n in 1..=3 {
        assert!(
            matches!(
                again.report.recovery.outcome_of(gid(n)),
                Some(RecoveryOutcome::Rebased { .. })
            ),
            "game {n}: {:?}",
            again.report.recovery.outcome_of(gid(n))
        );
    }
}

/// A lock held for less than the budget costs only what it is held: all three are rebuilt.
#[test]
fn d4r_07_a_short_database_lock_does_not_fail_any_game() {
    let (d, s) = three_games_with_damaged_journals();
    drop(s);
    let dir = d.path().to_path_buf();
    let holder = {
        let dir = dir.clone();
        std::thread::spawn(move || {
            let cue = Instant::now();
            while !(1..=3).any(|n| rebase_path(&dir, n).exists()) {
                assert!(cue.elapsed() < Duration::from_secs(60), "no rebase began");
                std::thread::sleep(Duration::from_millis(2));
            }
            let conn = raw(&dir);
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            std::thread::sleep(Duration::from_millis(1_500));
            conn.execute_batch("ROLLBACK").unwrap();
        })
    };
    let _guard = FaultPlan::new()
        .delay("journal.rebase.built", Duration::from_millis(400))
        .once()
        .install(&dir)
        .unwrap();
    let opened = block_on(Store::open(options(&dir))).unwrap();
    holder.join().unwrap();
    for n in 1..=3 {
        assert!(
            matches!(
                opened.report.recovery.outcome_of(gid(n)),
                Some(RecoveryOutcome::Rebased { .. })
            ),
            "game {n}: {:?}",
            opened.report.recovery.outcome_of(gid(n))
        );
    }
}

// ==================================================================== FIND-D4R-09: B-R04 grace

/// The seam `journal.create.created` makes the race of B-R04 certain: a thread takes the lock of
/// the new name at once and holds it for 80 ms; `create` looks at the lock 40 ms after it made
/// the file, meets `WouldBlock`, and the grace (about 80 ms of retries) lets it in once the
/// holder lets go. Kills the mutants with no grace, with no retry and with the grace spent
/// (W01, W02, W06).
#[test]
fn b_r04_the_grace_lets_create_in_after_a_short_holder() {
    let (d, s) = world();
    let journal: MoveJournal = s.journal().unwrap();
    let game = gid(60);
    let path = journal.path(&game);
    let _guard = FaultPlan::new()
        .delay("journal.create.created", Duration::from_millis(40))
        .once()
        .install(d.path())
        .unwrap();
    let holder = {
        let path = path.clone();
        std::thread::spawn(move || {
            let cue = Instant::now();
            loop {
                if let Ok(file) = fs::File::open(&path)
                    && file.try_lock().is_ok()
                {
                    std::thread::sleep(Duration::from_millis(80));
                    drop(file);
                    return;
                }
                assert!(cue.elapsed() < Duration::from_secs(10), "no file appeared");
                std::thread::yield_now();
            }
        })
    };
    let writer = journal
        .create(&game)
        .expect("the grace waits for the holder");
    holder.join().unwrap();
    assert_eq!(journal.probe(&game), LeaseState::Held);
    drop(writer);
}

/// A holder that stays longer than the grace: `create` says `Locked` and the file stays (it
/// belongs to the holder; it used to be removed, mutant W03). The seam makes the holder certain.
#[test]
fn b_r04_a_holder_that_outlasts_the_grace_keeps_its_file() {
    let (d, s) = world();
    let journal: MoveJournal = s.journal().unwrap();
    let game = gid(61);
    let path = journal.path(&game);
    let _guard = FaultPlan::new()
        .delay("journal.create.created", Duration::from_millis(30))
        .once()
        .install(d.path())
        .unwrap();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let holder = {
        let path = path.clone();
        std::thread::spawn(move || {
            let cue = Instant::now();
            loop {
                if let Ok(file) = fs::File::open(&path)
                    && file.try_lock().is_ok()
                {
                    let _ = stop_rx.recv_timeout(Duration::from_secs(10));
                    drop(file);
                    return;
                }
                assert!(cue.elapsed() < Duration::from_secs(10), "no file appeared");
                std::thread::yield_now();
            }
        })
    };
    let started = Instant::now();
    let result = journal.create(&game);
    let waited = started.elapsed();
    assert!(
        matches!(result, Err(JournalError::Locked { .. })),
        "{:?}",
        result.err()
    );
    assert!(
        waited >= Duration::from_millis(60),
        "the grace was used: {waited:?}"
    );
    assert!(path.exists(), "create must not remove a holder's file");
    stop_tx.send(()).unwrap();
    holder.join().unwrap();
    assert!(path.exists());
}

// ============================================================ sync faults on the repair paths

/// The removal of the old name at startup (B-R03 finished) is followed by a sync of `journal/`
/// (mutant D04): with that sync failing, the scan does not report success.
#[test]
fn the_b_r03_repair_at_startup_syncs_the_journal_directory() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    fs::create_dir_all(jdir(d.path()).join("done")).unwrap();
    fs::hard_link(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    drop(s);
    let guard = FaultPlan::new()
        .errno("sync.journal_move_dir", EIO)
        .install(d.path())
        .unwrap();
    let opened = block_on(Store::open(options(d.path()))).unwrap();
    drop(guard);
    assert!(
        matches!(
            opened.report.recovery.outcome_of(gid(1)),
            Some(RecoveryOutcome::Failed { .. })
        ),
        "{:?}",
        opened.report.recovery.outcome_of(gid(1))
    );
    // The removal itself was made; the repair left one name.
    assert!(!jpath(d.path(), 1).exists());
    assert_eq!(fs::metadata(done_path(d.path(), 1)).unwrap().nlink(), 1);
}

/// The aside moves sync `journal/` and then `journal/damaged/`; a failure of the second sync is
/// an error (mutant F13: no sync of `damaged/` in the aside path of an incomplete `.rebase`).
#[test]
fn the_aside_move_of_an_incomplete_rebase_syncs_the_damaged_directory() {
    let (d, _s) = world();
    let store = store_uuid(d.path());
    fs::write(rebase_path(d.path(), 1), b"cut").unwrap();
    let guard = FaultPlan::new()
        .errno("sync.journal_move_dir", EIO)
        .skip(1)
        .install(d.path())
        .unwrap();
    let result = settle_rebase_files(&jdir(d.path()), Uuid::from_u128(1), store);
    drop(guard);
    assert!(
        result.is_err(),
        "the second sync (damaged/) must be made: {result:?}"
    );
    // The move itself was made before the failing sync: the bytes are kept and a second call has
    // nothing left to do.
    assert_eq!(damaged_files(d.path(), 1, ".rebase").len(), 1);
    assert_eq!(
        settle_rebase_files(&jdir(d.path()), Uuid::from_u128(1), store).unwrap(),
        Settled::Nothing
    );
}

/// The same for the damaged-journal move: the second sync is the one of `damaged/`.
#[test]
fn the_damaged_move_syncs_both_directories() {
    let (d, _s) = world();
    fs::write(jpath(d.path(), 1), b"journal").unwrap();
    let guard = FaultPlan::new()
        .errno("sync.journal_move_dir", EIO)
        .skip(1)
        .install(d.path())
        .unwrap();
    let result = move_aside_damaged(&jdir(d.path()), Uuid::from_u128(1));
    drop(guard);
    assert!(result.is_err(), "{result:?}");
}
