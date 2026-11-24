//! Part D4, `StartupRecovery` (DD-ST-019, SDD 3.9.1, architecture 7.6): what `Store::open` does
//! with the journals that have no live lock. CT-ST-009 (a `GameEnd` with no finalize: finalized
//! once), CT-ST-084 (several pending `GameEnd` journals in `ended_at` order, ties by uuid, the
//! rating equals the in-order result), an unapplied `Voided`, a voided row with no `Voided`
//! record, D-87, B-R03 (`close_final` crash repair), one bad journal never stops the others, a
//! held journal is skipped, a parked game is reported and not retried in a loop, an empty header
//! with no row is removed, a journal of no game is left alone.
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
use std::path::Path;

use chess_domain::{GameStatus, Matchup, Mode, WallMs};
use chess_store::journal::format::{GameEndPayload, Payload, SeatKind};
use chess_store::journal::{RecoveryOutcome, RunSource};
use chess_store::{DataDir, ParkCause, Store};
use common::{block_on, computer, gid, human, jb, new_game, open_in, pid, raw};
use d4_support::{
    ENDED, apply, corrupt_record, damaged_files, done_path, dump, game_end, jpath, live, playing,
    rebase_path, rec, reopen, start, start_for, voided, world, write_journal,
};
use uuid::Uuid;

fn status(s: &Store, n: u128) -> GameStatus {
    block_on(s.get_game(gid(n))).unwrap().status
}

fn strict_elo(dir: &Path, id: u128) -> i64 {
    raw(dir)
        .query_row(
            "SELECT strict_elo FROM player WHERE uuid = ?1",
            [Uuid::from_u128(id).hyphenated().to_string()],
            |r| r.get(0),
        )
        .unwrap()
}

// ------------------------------------------------------------------------------ CT-ST-009

/// CT-ST-009: the journal holds the `GameEnd`, the finalize never committed (a kill between the
/// two). Startup finalizes it once; a second start changes nothing.
#[test]
fn ct_st_009_a_game_end_with_no_finalize_is_finalized_once_at_startup() {
    let (d, s) = live();
    let mut payloads = playing(1, 3);
    payloads.push(game_end(ENDED));
    write_journal(&s, 1, payloads);
    assert_eq!(status(&s, 1), GameStatus::InProgress);
    let elo_before = (strict_elo(d.path(), 1), strict_elo(d.path(), 2));
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Finalized)
    );
    let s = opened.store;
    assert_eq!(status(&s, 1), GameStatus::Completed);
    let g = block_on(s.get_game(gid(1))).unwrap();
    assert_eq!(g.ended_at, Some(WallMs(ENDED)));
    assert_eq!(g.journal_seq, 5);
    assert_eq!(block_on(s.game_moves(gid(1))).unwrap().len(), 3);
    let elo_after = (strict_elo(d.path(), 1), strict_elo(d.path(), 2));
    assert_ne!(elo_before, elo_after, "the rating moved once");
    assert!(done_path(d.path(), 1).exists() && !jpath(d.path(), 1).exists());
    // A second start: nothing to do, nothing changes.
    let snapshot = dump(d.path());
    let opened = reopen(d.path(), s);
    assert!(opened.report.recovery.games.is_empty());
    assert_eq!(dump(d.path()), snapshot);
    assert_eq!(
        elo_after,
        (strict_elo(d.path(), 1), strict_elo(d.path(), 2))
    );
}

/// An aborted game (`Abort` record) finalizes the same way.
#[test]
fn an_abort_record_is_applied_and_closed() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(Payload::Abort(chess_store::journal::format::AbortPayload {
        ended_at: ENDED,
        content_hash: "sha256:v1:ab".into(),
    }));
    write_journal(&s, 1, payloads);
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Finalized)
    );
    assert_eq!(status(&opened.store, 1), GameStatus::Aborted);
    assert!(done_path(d.path(), 1).exists());
}

// ------------------------------------------------------------------------------ CT-ST-084

fn hc_world(dir: &Path) -> Store {
    let s = open_in(dir).store;
    for (n, name) in [(1, "Ann"), (2, "Bob"), (3, "Cy")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![computer(10, "Bot A", 1, 0, 3)])).unwrap();
    s
}

fn hc_game(s: &Store, n: u128, human: u128) {
    block_on(s.start_game(new_game(
        n,
        pid(human),
        pid(10),
        Matchup::HumanComputer,
        Mode::Strict,
    )))
    .unwrap();
}

fn ended(result: &str, ended_at: i64) -> Payload {
    let Payload::GameEnd(end) = game_end(ended_at) else {
        panic!()
    };
    Payload::GameEnd(GameEndPayload {
        result: result.into(),
        ..end
    })
}

/// Three games against the same computer Player, their journals complete (`GameEnd`) and none
/// finalized. The uuids run opposite to the end times. Startup finalizes them ascending by
/// `ended_at`, ties by `game_uuid`, and the computer's rating equals the result of finalizing
/// them in that order on a control store (D-72, BP-012).
#[test]
fn ct_st_084_pending_game_ends_are_finalized_in_ended_at_order() {
    // (game, human, ended_at, result): game 30 ends first, 20 and 10 tie, so 10 goes before 20.
    let plan: [(u128, u128, i64, &str); 3] = [
        (10, 1, 9_500_000, "0-1"),
        (20, 2, 9_500_000, "1-0"),
        (30, 3, 9_100_000, "0-1"),
    ];
    let build = |dir: &Path| {
        let s = hc_world(dir);
        for (game, human, ended_at, result) in plan {
            hc_game(&s, game, human);
            let payloads = vec![
                start_for(game, human, 10, SeatKind::Human, SeatKind::Computer),
                jb::mv(0),
                ended(result, ended_at),
            ];
            write_journal(&s, game, payloads);
        }
        s
    };
    // The result under test.
    let dir = tempfile::TempDir::new().unwrap();
    let s = build(dir.path());
    let opened = reopen(dir.path(), s);
    let order: Vec<_> = opened
        .report
        .recovery
        .games
        .iter()
        .map(|g| g.game)
        .collect();
    assert_eq!(
        order,
        vec![gid(30), gid(10), gid(20)],
        "ended_at, then uuid"
    );
    for g in &opened.report.recovery.games {
        assert_eq!(g.outcome, RecoveryOutcome::Finalized);
    }
    let got = strict_elo(dir.path(), 10);
    drop(opened);
    // The control: the same three games finalized by `apply_records` in that order.
    let control = tempfile::TempDir::new().unwrap();
    let s = hc_world(control.path());
    for (game, human, ended_at, result) in [plan[2], plan[0], plan[1]] {
        hc_game(&s, game, human);
        let records = vec![
            rec(
                1,
                start_for(game, human, 10, SeatKind::Human, SeatKind::Computer),
            ),
            rec(2, jb::mv(0)),
            rec(3, ended(result, ended_at)),
        ];
        apply(&s, game, records);
    }
    assert_eq!(got, strict_elo(control.path(), 10), "the in-order result");
    // The other order gives another rating, so the test can tell.
    let wrong = tempfile::TempDir::new().unwrap();
    let s = hc_world(wrong.path());
    for (game, human, ended_at, result) in plan {
        hc_game(&s, game, human);
        let records = vec![
            rec(
                1,
                start_for(game, human, 10, SeatKind::Human, SeatKind::Computer),
            ),
            rec(2, jb::mv(0)),
            rec(3, ended(result, ended_at)),
        ];
        apply(&s, game, records);
    }
    assert_ne!(
        got,
        strict_elo(wrong.path(), 10),
        "the order matters for this fixture"
    );
}

// ------------------------------------------------------------------------------ Voided

/// A `Voided` the scan finds unapplied (the row is still `in_progress`, as after a restore):
/// the void is applied and the journal is closed. No rating changes.
#[test]
fn a_voided_record_that_the_store_has_not_applied_is_applied_at_startup() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(voided(Some(1)));
    write_journal(&s, 1, payloads);
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Finalized)
    );
    let g = block_on(opened.store.get_game(gid(1))).unwrap();
    assert_eq!(g.status, GameStatus::Voided);
    assert_eq!(g.ended_at, Some(WallMs(8_000_000)));
    assert!(done_path(d.path(), 1).exists());
}

/// D-87 at startup: `GameEnd` then `Voided` is a voided journal; the `GameEnd` is never
/// finalized, so no rating is settled.
#[test]
fn d_87_a_game_end_followed_by_voided_settles_no_rating_at_startup() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    payloads.push(voided(Some(1)));
    write_journal(&s, 1, payloads);
    let elo = (strict_elo(d.path(), 1), strict_elo(d.path(), 2));
    let opened = reopen(d.path(), s);
    assert_eq!(status(&opened.store, 1), GameStatus::Voided);
    assert_eq!(elo, (strict_elo(d.path(), 1), strict_elo(d.path(), 2)));
}

/// The row is `voided` but the journal has no `Voided` record (a crash between the reset commit
/// and the journal write): the scan never appends, so it reports the need and leaves the journal.
#[test]
fn a_voided_row_with_no_voided_record_is_reported_and_left_for_the_repair() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 2));
    apply(&s, 1, records);
    block_on(s.reset_player(
        pid(1),
        vec![chess_store::VoidSpec {
            game: gid(1),
            content_hash: "sha256:v1:vv".into(),
        }],
        "{}".into(),
    ))
    .unwrap();
    assert_eq!(status(&s, 1), GameStatus::Voided);
    let bytes = fs::read(jpath(d.path(), 1)).unwrap();
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::NeedsRecord)
    );
    assert_eq!(
        fs::read(jpath(d.path(), 1)).unwrap(),
        bytes,
        "the scan appends nothing"
    );
}

// ----------------------------------------------------------------------- interrupted games

/// A game with no terminal record keeps its journal; the rows are brought to the journal.
#[test]
fn an_interrupted_game_is_reconciled_and_keeps_its_journal() {
    let (d, s) = live();
    write_journal(&s, 1, playing(1, 3));
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Reconciled { recovered_moves: 3 })
    );
    assert_eq!(block_on(opened.store.game_moves(gid(1))).unwrap().len(), 3);
    assert!(jpath(d.path(), 1).exists());
    assert_eq!(status(&opened.store, 1), GameStatus::InProgress);
    // CT-ST-003 (D-65, store half): the journal ends with the Move; no GameEnd is applied.
    let g = block_on(opened.store.get_game(gid(1))).unwrap();
    assert_eq!(g.ended_at, None);
}

/// A damaged journal of an in-progress game is rebased at startup, no matter who resumes it.
#[test]
fn a_damaged_in_progress_journal_is_rebased_at_startup() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 4));
    apply(&s, 1, records);
    let original = corrupt_record(&jpath(d.path(), 1), 3, common_store(d.path()), 1);
    let opened = reopen(d.path(), s);
    assert!(matches!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(RecoveryOutcome::Rebased {
            recovered_moves: 4,
            ..
        })
    ));
    let aside = damaged_files(d.path(), 1, ".jnl");
    assert_eq!(aside.len(), 1);
    assert_eq!(fs::read(&aside[0]).unwrap(), original);
    assert!(!rebase_path(d.path(), 1).exists());
    // A second start changes nothing.
    let tree = d4_support::journal_tree(d.path());
    let snapshot = dump(d.path());
    let opened = reopen(d.path(), opened.store);
    assert!(matches!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(RecoveryOutcome::Reconciled { .. })
    ));
    assert_eq!(tree, d4_support::journal_tree(d.path()));
    assert_eq!(snapshot, dump(d.path()));
}

fn common_store(dir: &Path) -> u128 {
    DataDir::new(dir)
        .read_store_id()
        .unwrap()
        .unwrap()
        .store_uuid
        .as_u128()
}

/// RR-A-1 (b) at startup: a final game with a damaged journal: moved aside, rows unchanged.
#[test]
fn a_final_game_with_a_damaged_journal_is_moved_aside_at_startup() {
    let (d, s) = live();
    let mut payloads = playing(1, 3);
    payloads.push(game_end(ENDED));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    let original = corrupt_record(&jpath(d.path(), 1), 2, common_store(d.path()), 1);
    let before = dump(d.path());
    let opened = reopen(d.path(), s);
    assert!(matches!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(RecoveryOutcome::MovedAside { damaged: Some(_) })
    ));
    let aside = damaged_files(d.path(), 1, ".jnl");
    assert_eq!(aside.len(), 1);
    assert_eq!(fs::read(&aside[0]).unwrap(), original);
    assert!(!jpath(d.path(), 1).exists() && !rebase_path(d.path(), 1).exists());
    assert_eq!(dump(d.path()), before);
}

// ----------------------------------------------------------------------------------- B-R03

/// B-R03: the journal exists under two names (`close_final` died between the link and the
/// removal): the scan finishes the move by removing the old name; it reports success and loops
/// on nothing.
#[test]
fn b_r03_a_journal_under_two_names_is_finished_by_the_scan() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    fs::create_dir_all(d.path().join("journal/done")).unwrap();
    fs::hard_link(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    assert_eq!(fs::metadata(jpath(d.path(), 1)).unwrap().nlink(), 2);
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::FinishedClose)
    );
    assert!(!jpath(d.path(), 1).exists());
    assert_eq!(fs::metadata(done_path(d.path(), 1)).unwrap().nlink(), 1);
}

/// B-R03: a `done/` file that is **not** the same file is not a retry: neither file is touched,
/// and the scan reports it as a failure.
#[test]
fn b_r03_a_different_file_in_done_is_damage_and_nothing_is_touched() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    write_journal(&s, 1, payloads);
    fs::create_dir_all(d.path().join("journal/done")).unwrap();
    fs::write(done_path(d.path(), 1), b"another file").unwrap();
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    let opened = reopen(d.path(), s);
    assert!(matches!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(RecoveryOutcome::Failed { .. })
    ));
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
    assert_eq!(fs::read(done_path(d.path(), 1)).unwrap(), b"another file");
    assert_eq!(status(&opened.store, 1), GameStatus::InProgress);
}

/// B-R03 for `close_final` itself: a second call after a crash between the link and the removal
/// repairs instead of looping on `AlreadyExists`.
#[test]
fn b_r03_close_final_repairs_a_journal_that_already_has_its_done_name() {
    let (d, s) = live();
    let journal = s.journal().unwrap();
    let w = journal.create(&gid(1)).unwrap();
    block_on(w.append(5_001, jb::start(1))).unwrap();
    fs::create_dir_all(d.path().join("journal/done")).unwrap();
    fs::hard_link(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    w.close_final().unwrap();
    assert!(!jpath(d.path(), 1).exists());
    assert!(done_path(d.path(), 1).exists());
    // A different file in `done/` is an error and changes nothing.
    let w = journal.create(&gid(2)).unwrap();
    block_on(w.append(5_002, jb::start(2))).unwrap();
    fs::write(done_path(d.path(), 2), b"other").unwrap();
    let err = w.close_final().unwrap_err();
    assert!(matches!(err, chess_store::JournalError::WriteFailed { .. }));
    assert!(jpath(d.path(), 2).exists());
    assert_eq!(fs::read(done_path(d.path(), 2)).unwrap(), b"other");
}

// -------------------------------------------------------------------------- robustness

/// One bad journal never stops the others: a journal of another store, one the store refuses
/// (a parked game), and a good one in between.
#[test]
fn one_bad_journal_never_stops_the_others() {
    let (d, s) = world();
    start(&s, 1);
    block_on(s.register_player(human(3, "Cy"))).unwrap();
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
    // Game 1: a journal of another store (the header names a foreign store id).
    write_journal(&s, 1, playing(1, 1));
    let foreign = {
        let mut bytes = fs::read(jpath(d.path(), 1)).unwrap();
        let header = chess_store::journal::format::encode_header(
            Uuid::from_u128(1),
            Uuid::from_u128(0xDEAD),
        );
        bytes[..header.len()].copy_from_slice(&header);
        bytes
    };
    fs::write(jpath(d.path(), 1), &foreign).unwrap();
    // Game 2: the store refuses it (empty hash): it is parked.
    let refused = Payload::GameEnd(GameEndPayload {
        content_hash: String::new(),
        ..match game_end(ENDED) {
            Payload::GameEnd(e) => e,
            _ => panic!(),
        }
    });
    write_journal(
        &s,
        2,
        vec![
            start_for(2, 3, 10, SeatKind::Human, SeatKind::Computer),
            jb::mv(0),
            refused,
        ],
    );
    // Game 3: fine, and it ends.
    write_journal(
        &s,
        3,
        vec![
            start_for(3, 2, 11, SeatKind::Human, SeatKind::Computer),
            jb::mv(0),
            game_end(ENDED + 5),
        ],
    );
    let opened = reopen(d.path(), s);
    let report = &opened.report.recovery;
    assert!(matches!(
        report.outcome_of(gid(1)),
        Some(RecoveryOutcome::Failed { .. })
    ));
    assert!(matches!(
        report.outcome_of(gid(2)),
        Some(RecoveryOutcome::Parked {
            cause: Some(ParkCause::InvariantViolated)
        })
    ));
    assert_eq!(report.outcome_of(gid(3)), Some(&RecoveryOutcome::Finalized));
    assert_eq!(status(&opened.store, 3), GameStatus::Completed);
    // The refused journal and the foreign one are untouched.
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), foreign);
    assert!(jpath(d.path(), 2).exists());
    // The parked game is listed once; the scan did not loop.
    let parked = block_on(opened.store.parked_games()).unwrap();
    assert_eq!(parked.len(), 1);
    assert_eq!(parked[0].game, gid(2));
}

/// A journal whose lock is held by a live writer is skipped and untouched.
#[test]
fn a_journal_with_a_live_lock_is_skipped() {
    let (d, s) = live();
    let w = s.journal().unwrap().create(&gid(1)).unwrap();
    block_on(w.append(5_001, jb::start(1))).unwrap();
    block_on(w.append(5_002, jb::mv(0))).unwrap();
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    // A second store handle opens the same directory while the lock is held.
    let second = common::try_open(d.path());
    let opened = second.unwrap();
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Held)
    );
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
    assert!(
        block_on(opened.store.game_moves(gid(1)))
            .unwrap()
            .is_empty()
    );
    drop(w);
}

/// A journal of a game that has no row is left alone; a file that is only a header is removed
/// (STATE-ST-01).
#[test]
fn a_journal_of_no_game_is_left_alone_and_an_empty_one_is_removed() {
    let (d, s) = world();
    // A journal with content for game 40, a header-only one for game 41; neither has a row.
    let journal = s.journal().unwrap();
    let w = journal.create(&gid(40)).unwrap();
    block_on(w.append(1_000, jb::start(40))).unwrap();
    drop(w);
    drop(journal.create(&gid(41)).unwrap());
    let content = fs::read(jpath(d.path(), 40)).unwrap();
    let opened = reopen(d.path(), s);
    let report = &opened.report.recovery;
    assert_eq!(
        report.outcome_of(gid(40)),
        Some(&RecoveryOutcome::NoGameRow)
    );
    assert_eq!(
        report.outcome_of(gid(41)),
        Some(&RecoveryOutcome::RemovedEmpty)
    );
    assert_eq!(fs::read(jpath(d.path(), 40)).unwrap(), content);
    assert!(!jpath(d.path(), 41).exists());
}

/// A row with no journal at startup: nothing to scan, nothing created (the resume creates it).
#[test]
fn a_row_with_no_journal_is_not_touched_by_the_scan() {
    let (d, s) = live();
    let opened = reopen(d.path(), s);
    assert!(opened.report.recovery.games.is_empty());
    assert!(!jpath(d.path(), 1).exists());
}

/// Files that are not journals (Finder droppings, a directory) are ignored.
#[test]
fn files_that_are_not_journals_are_ignored() {
    let (d, s) = live();
    fs::write(d.path().join("journal/.DS_Store"), b"x").unwrap();
    fs::write(d.path().join("journal/not-a-uuid.jnl"), b"x").unwrap();
    let opened = reopen(d.path(), s);
    assert!(opened.report.recovery.games.is_empty());
    assert!(!opened.report.recovery.scan_failed);
}

/// A first launch reports no recovery at all.
#[test]
fn a_first_launch_has_an_empty_recovery_report() {
    let dir = tempfile::TempDir::new().unwrap();
    let opened = open_in(dir.path());
    assert!(opened.report.first_launch);
    assert_eq!(
        opened.report.recovery,
        chess_store::journal::StartupReport::default()
    );
}

/// The unpark repair end to end after a start: `unpark_game`, then the next resume passes the
/// whole journal.
#[test]
fn an_unparked_game_is_repaired_by_the_next_resume() {
    let (d, s) = live();
    let payloads = playing(1, 2);
    let refused_end = Payload::GameEnd(GameEndPayload {
        content_hash: String::new(),
        ..match game_end(ENDED) {
            Payload::GameEnd(e) => e,
            _ => panic!(),
        }
    });
    let mut all = payloads;
    all.push(refused_end);
    write_journal(&s, 1, all);
    let opened = reopen(d.path(), s);
    assert!(matches!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(RecoveryOutcome::Parked { .. })
    ));
    let s = opened.store;
    assert_eq!(block_on(s.unpark_game(gid(1))).unwrap(), Some(1));
    // The journal is repaired by hand here: the refused record is replaced by a valid end.
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let mut good = playing(1, 2);
    good.push(game_end(ENDED));
    write_journal(&s, 1, good);
    let journal = s.journal().unwrap();
    let (writer, recovered) = block_on(journal.open_for_resume(&s, &gid(1))).unwrap();
    assert_eq!(recovered.records.len(), 4);
    assert_eq!(status(&s, 1), GameStatus::Completed);
    assert_eq!(recovered.report.source, RunSource::Journal);
    writer.unwrap().close_final().unwrap();
}
