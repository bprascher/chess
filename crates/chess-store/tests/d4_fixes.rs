//! Part D4 review fixes (FIND-D4R-02 to -06, -09, -10, -12; the review is
//! `docs/engineering/review-chess-store-D4.md`). No test here needs the `test-support` feature;
//! the ones that do (the lock race of FIND-D4R-01, the startup budget of FIND-D4R-07, the grace
//! of B-R04, the sync faults) are in `d4_fixes_faults.rs`.
//!
//! Each test names the finding it closes. A test that must fail on the code of commit `0b0a47d`
//! says so (`fails before the fix`); the ones that pin a property the old tests did not check
//! (the mutants D23, D32, D36 to D38, D46, R03 to R05, F12, W04 of the review) say which.
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
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::Path;

use chess_domain::{Matchup, Mode};
use chess_store::journal::format::{HEADER_LEN, Payload, encode_header, encode_record};
use chess_store::journal::{
    JournalWriter, LeaseState, MoveJournal, Rebased, Recovered, RecoveryOutcome, ResumeError,
    Settled, move_aside_damaged, scan_file, settle_rebase_files, write_rebase_file,
};
use chess_store::{DataDir, JournalError, NewGame, Store, WriteCause};
use common::{block_on, gid, jb, new_game, open_in, pid};
use d4_support::{
    ENDED, apply, corrupt_record, damaged_dir, damaged_files, done_path, game_end, jdir, jpath,
    live, playing, rebase_path, rec, reopen, world, write_journal,
};
use uuid::Uuid;

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

/// The outcome by its `Debug` name, so that a test of a new outcome compiles (and fails) on the
/// code before the fix too.
fn outcome_name(opened: &chess_store::OpenedStore, n: u128) -> String {
    format!("{:?}", opened.report.recovery.outcome_of(gid(n)))
}

/// A live game 1 with `moves` moves in the database and **no** journal file.
fn row_with_no_journal(moves: u32) -> (tempfile::TempDir, Store) {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, moves));
    apply(&s, 1, records);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    (d, s)
}

/// A finished game 1 (`GameEnd` applied), its journal still in `journal/`.
fn finished_with_journal() -> (tempfile::TempDir, Store) {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    (d, s)
}

fn make_done_dir(dir: &Path) {
    fs::create_dir_all(jdir(dir).join("done")).unwrap();
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).unwrap().file_type().is_symlink()
}

// ============================================================ FIND-D4R-03: symbolic links

/// `close_final` with a symbolic link at `done/<uuid>.jnl` that points at the journal: the old
/// code took the link for the same file, removed the journal and returned `Ok`. Fails before
/// the fix.
#[test]
fn d4r_03_close_final_refuses_a_symlink_at_the_done_name() {
    let (d, s) = finished_with_journal();
    make_done_dir(d.path());
    symlink(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    let (writer, _) = resume(&s, 1).unwrap();
    let err = writer
        .expect("an intact journal of a final game")
        .close_final();
    assert_eq!(
        err,
        Err(JournalError::WriteFailed {
            cause: WriteCause::Io(std::io::ErrorKind::AlreadyExists)
        })
    );
    assert_eq!(
        fs::read(jpath(d.path(), 1)).unwrap(),
        before,
        "journal kept"
    );
    assert!(
        is_symlink(&done_path(d.path(), 1)),
        "the link is left alone"
    );
}

/// The same with a link that points nowhere (the old code: `Err(NotFound)`; now the same
/// refusal as for any name that is not this file).
#[test]
fn d4r_03_close_final_refuses_a_dangling_symlink_at_the_done_name() {
    let (d, s) = finished_with_journal();
    make_done_dir(d.path());
    symlink(d.path().join("nowhere"), done_path(d.path(), 1)).unwrap();
    let (writer, _) = resume(&s, 1).unwrap();
    let err = writer.unwrap().close_final();
    assert_eq!(
        err,
        Err(JournalError::WriteFailed {
            cause: WriteCause::Io(std::io::ErrorKind::AlreadyExists)
        })
    );
    assert!(jpath(d.path(), 1).exists());
    assert!(is_symlink(&done_path(d.path(), 1)));
}

/// Startup with a symlink at `done/`: the journal is not removed and the game is reported with
/// its own outcome (before the fix: `FinishedClose`, and the journal was gone). Fails before the
/// fix.
#[test]
fn d4r_03_startup_does_not_delete_a_journal_because_of_a_symlink() {
    let (d, s) = finished_with_journal();
    make_done_dir(d.path());
    symlink(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    let opened = reopen(d.path(), s);
    assert_eq!(outcome_name(&opened, 1), "Some(DoneNameCollision)");
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
    assert!(is_symlink(&done_path(d.path(), 1)));
}

// ================================================= FIND-D4R-04: the row decides, B-R03

/// A game still in progress whose journal is also under `done/` (the same inode): the old name
/// is not removed (before the fix: `FinishedClose`, and the next resume rebuilt the game from the
/// store as `StoreAhead`). The scan goes on as for any journal, and a resume continues on
/// `journal/`. Fails before the fix.
#[test]
fn d4r_04_the_same_inode_in_done_does_not_finish_a_close_for_a_game_in_progress() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 2));
    apply(&s, 1, records);
    make_done_dir(d.path());
    fs::hard_link(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Reconciled { recovered_moves: 2 })
    );
    assert!(jpath(d.path(), 1).exists(), "the live name stays");
    assert_eq!(fs::metadata(jpath(d.path(), 1)).unwrap().nlink(), 2);
    let (writer, recovered) = resume(&opened.store, 1).unwrap();
    assert!(writer.is_some());
    assert_eq!(recovered.records.len(), 3);
    assert_eq!(recovered.report.damaged, None, "no rebuild, no StoreAhead");
}

/// A **different** file at `done/` for a game in progress: nothing is touched by the scan, the
/// scan reports the collision, and the game is **not** made unresumable: the resume goes on from
/// `journal/` and returns a writer. Fails before the fix (`Failed`, and the resume answered
/// `WriteFailed` for ever).
#[test]
fn d4r_04_a_different_file_in_done_leaves_the_game_resumable() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 2));
    apply(&s, 1, records);
    make_done_dir(d.path());
    fs::write(done_path(d.path(), 1), b"someone else's file").unwrap();
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    let opened = reopen(d.path(), s);
    assert_eq!(outcome_name(&opened, 1), "Some(DoneNameCollision)");
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
    assert_eq!(
        fs::read(done_path(d.path(), 1)).unwrap(),
        b"someone else's file"
    );
    let (writer, recovered) = resume(&opened.store, 1).expect("not blocked by the collision");
    assert!(writer.is_some());
    assert_eq!(recovered.records.len(), 3);
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
}

/// The resume of a final game whose journal is already under `done/` under the same inode:
/// the old name is removed (B-R03 finished), no writer. (No test covered the resume side.)
#[test]
fn d4r_04_a_resume_finishes_a_close_for_a_final_game() {
    let (d, s) = finished_with_journal();
    make_done_dir(d.path());
    fs::hard_link(jpath(d.path(), 1), done_path(d.path(), 1)).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(writer.is_none());
    assert!(recovered.records.is_empty());
    assert!(!jpath(d.path(), 1).exists());
    assert_eq!(fs::metadata(done_path(d.path(), 1)).unwrap().nlink(), 1);
}

// ============================================================ FIND-D4R-05: the cut .rebase

/// A game with `Start`, two moves, a stopped `Clock` (a pause) and a third move, all applied
/// (`journal_seq` 5), and the bytes a rebase of it writes (the six records, the closing `Clock`
/// last). Returns the directory, the store, the full `.rebase` bytes and the offset after the
/// stopped `Clock`.
fn rebase_with_a_stopped_clock_in_the_middle() -> (tempfile::TempDir, Store, Vec<u8>, usize) {
    let (d, s) = live();
    let payloads = vec![
        jb::start(1),
        jb::mv(0),
        jb::mv(1),
        jb::clock(299_000, 301_000),
        jb::mv(2),
    ];
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records.clone());
    let store = store_uuid(d.path());
    let mut bytes = encode_header(Uuid::from_u128(1), store).to_vec();
    for record in &records {
        bytes.extend(encode_record(record).unwrap());
    }
    bytes.extend(encode_record(&rec(6, jb::clock(298_000, 302_000))).unwrap());
    // The boundary after the stopped Clock (record index 3) is the start of record index 4.
    let boundary = d4_support::record_offset(&bytes, 4, store.as_u128(), 1);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    (d, s, bytes, boundary)
}

/// FIND-D4R-05: a `.rebase` cut right after a stopped `Clock` from the middle of the journal
/// looks like the rebase of a journal that ends in a stopped `Clock`. The store's `journal_seq`
/// (5) says the file must reach at least `seq` 5, so it is incomplete: kept in `damaged/`, the
/// game rebuilt from the store. Fails before the fix (the cut file was installed: 4 records).
#[test]
fn d4r_05_a_rebase_cut_at_a_stopped_clock_in_the_middle_is_not_installed() {
    let (d, s, full, boundary) = rebase_with_a_stopped_clock_in_the_middle();
    fs::write(rebase_path(d.path(), 1), &full[..boundary]).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(writer.is_some());
    assert!(recovered.report.rebased, "rebuilt from the store");
    let aside = damaged_files(d.path(), 1, ".rebase");
    assert_eq!(aside.len(), 1);
    assert_eq!(fs::read(&aside[0]).unwrap(), &full[..boundary]);
    assert!(!rebase_path(d.path(), 1).exists());
    // The rebuilt journal holds the three stored moves.
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        store_uuid(d.path()),
    )
    .unwrap();
    assert_eq!(
        scan.records
            .iter()
            .filter(|r| matches!(r.payload, Payload::Move(_)))
            .count(),
        3
    );
}

/// The control: the whole file is complete and is installed as it is.
#[test]
fn d4r_05_the_whole_rebase_is_installed() {
    let (d, s, full, _) = rebase_with_a_stopped_clock_in_the_middle();
    fs::write(rebase_path(d.path(), 1), &full).unwrap();
    let (writer, recovered) = resume(&s, 1).unwrap();
    assert!(writer.is_some());
    assert!(!recovered.report.rebased);
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), full);
    assert!(damaged_files(d.path(), 1, "").is_empty());
}

// ============================================================ FIND-D4R-06: moved_aside

fn damaged_game() -> (tempfile::TempDir, Store, Vec<u8>) {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 4));
    apply(&s, 1, records);
    let original = corrupt_record(&jpath(d.path(), 1), 2, store_uuid(d.path()).as_u128(), 1);
    (d, s, original)
}

/// `damaged/<uuid>.2.jnl` and `.3.jnl` exist, so the rebase makes `.1.jnl` (the first unused
/// name). The report names that file, not the highest `k` (mutant D36). Fails before the fix.
#[test]
fn d4r_06_the_report_names_the_file_just_made_not_the_highest_k() {
    let (d, s, original) = damaged_game();
    let dir = damaged_dir(d.path());
    fs::create_dir_all(&dir).unwrap();
    let id = Uuid::from_u128(1).hyphenated();
    fs::write(dir.join(format!("{id}.2.jnl")), b"two").unwrap();
    fs::write(dir.join(format!("{id}.3.jnl")), b"three").unwrap();
    let (_, recovered) = resume(&s, 1).unwrap();
    let named = recovered
        .report
        .moved_aside
        .expect("a file was moved aside");
    assert_eq!(named, dir.join(format!("{id}.1.jnl")));
    assert_eq!(fs::read(&named).unwrap(), original);
}

/// A rebase of a game that had no journal moved nothing aside (mutant D32).
#[test]
fn d4r_06_a_rebase_from_the_store_reports_no_file_moved_aside() {
    let (d, s) = row_with_no_journal(3);
    let (_, recovered) = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased);
    assert_eq!(recovered.report.moved_aside, None);
    assert!(damaged_files(d.path(), 1, "").is_empty());
}

// ============================================================ FIND-D4R-02: Locked

/// A `.rebase` that exists is another rebase in progress: `Locked`, and the file is not touched.
/// Before the fix the answer was `WriteFailed{AlreadyExists}`. Fails before the fix.
#[test]
fn d4r_02_write_rebase_file_answers_locked_when_a_rebase_exists() {
    let dir = tempfile::TempDir::new().unwrap();
    let data = DataDir::new(dir.path());
    data.ensure().unwrap();
    let jd = data.journal_dir();
    let game = Uuid::from_u128(5);
    let name = jd.join(format!("{}.jnl.rebase", game.hyphenated()));
    fs::write(&name, b"another process's rebase").unwrap();
    let rebased = Rebased {
        bytes: vec![1, 2, 3],
        last_seq: 1,
    };
    assert!(matches!(
        write_rebase_file(&jd, game, &rebased),
        Err(JournalError::Locked { .. })
    ));
    assert_eq!(fs::read(&name).unwrap(), b"another process's rebase");
}

// ============================================== FIND-D4R-12: header-only removal verifies

/// A header-only file of a game with no row is removed only when its header names this game and
/// this store; a foreign or renamed one stays (FIND-D4R-12). The own header is the control.
#[test]
fn d4r_12_a_header_only_file_with_no_row_is_removed_only_if_it_is_this_games() {
    let (d, s) = world();
    let store = store_uuid(d.path());
    let name = |n: u128| jpath(d.path(), n);
    fs::write(name(70), encode_header(Uuid::from_u128(70), store)).unwrap();
    fs::write(name(71), encode_header(Uuid::from_u128(72), store)).unwrap();
    fs::write(
        name(73),
        encode_header(Uuid::from_u128(73), Uuid::from_u128(0xDEAD)),
    )
    .unwrap();
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(70)),
        Some(&RecoveryOutcome::RemovedEmpty)
    );
    assert!(!name(70).exists());
    for n in [71, 73] {
        assert_eq!(
            opened.report.recovery.outcome_of(gid(n)),
            Some(&RecoveryOutcome::NoGameRow),
            "{n}"
        );
        assert!(
            name(n).exists(),
            "{n}: a file that is not this game's stays"
        );
        assert_eq!(fs::metadata(name(n)).unwrap().len(), HEADER_LEN as u64);
    }
}

// ================================================== FIND-D4R-09/10: tests the review asked for

/// `probe` counts a file it cannot open (not "not found") as held: the lock is not proven free
/// (mutant W04).
#[test]
fn probe_counts_a_file_it_cannot_open_as_held_and_a_missing_one_as_free() {
    let (d, s) = world();
    let journal: MoveJournal = s.journal().unwrap();
    assert_eq!(journal.probe(&gid(90)), LeaseState::Free);
    fs::write(jpath(d.path(), 90), b"x").unwrap();
    fs::set_permissions(jpath(d.path(), 90), fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(journal.probe(&gid(90)), LeaseState::Held);
    fs::set_permissions(jpath(d.path(), 90), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(journal.probe(&gid(90)), LeaseState::Free);
}

/// The synthesized `Start` carries the frozen `rated` and `custom_start` of the row (SDD 3.9.2;
/// mutants R04, R05): an unrated game from a given position, rebuilt from the store.
#[test]
fn the_synthesized_start_carries_rated_and_custom_start_from_the_row() {
    let (d, s) = world();
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let game = NewGame {
        rated: false,
        start_fen: Some(fen.to_owned()),
        ..new_game(1, pid(1), pid(2), Matchup::HumanHuman, Mode::Strict)
    };
    block_on(s.start_game(game)).unwrap();
    let records = write_journal(&s, 1, playing(1, 2));
    apply(&s, 1, records);
    fs::remove_file(jpath(d.path(), 1)).unwrap();
    let (_, recovered) = resume(&s, 1).unwrap();
    let Some(Payload::Start(start)) = recovered.records.first().map(|r| r.payload.clone()) else {
        panic!(
            "the rebuilt journal starts with a Start: {:?}",
            recovered.records
        );
    };
    assert!(!start.rated, "rated is the row's, not a default");
    assert!(start.custom_start, "custom_start is the row's");
    assert_eq!(start.start_fen.as_deref(), Some(fen));
    assert!(!start.self_play);
}

/// A store with moves-free rows and `journal_seq` above zero whose journal header is unreadable
/// is rebuilt, not declared unreadable: the `db_seq == 0` term of the `Unreadable` rule
/// (mutant D23).
#[test]
fn an_unreadable_header_with_a_journal_seq_and_no_moves_is_rebuilt() {
    let (d, s) = live();
    let records = write_journal(&s, 1, vec![jb::start(1)]);
    apply(&s, 1, records);
    let mut bytes = fs::read(jpath(d.path(), 1)).unwrap();
    bytes[..4].copy_from_slice(b"XXXX");
    fs::write(jpath(d.path(), 1), &bytes).unwrap();
    let (writer, recovered) = resume(&s, 1).expect("rebuilt, not Unreadable");
    assert!(writer.is_some());
    assert!(recovered.report.rebased);
    assert!(matches!(
        recovered.records.first().map(|r| &r.payload),
        Some(Payload::Start(_))
    ));
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
}

/// The scan reads names strictly: only `<canonical uuid>.jnl` and `.jnl.rebase` that are regular
/// files (mutants D37, D38). A game with rows and no journal is not rebuilt by the scan because
/// an upper-case or a directory name exists.
#[test]
fn the_scan_lists_only_canonical_names_that_are_files() {
    // (Upper case is not tested: the volume of the development host is case-insensitive, so
    // an upper-case name is the same file.)
    let n = 1;
    let (d, s) = world();
    d4_support::start(&s, n);
    let records = write_journal(&s, n, playing(n, 2));
    apply(&s, n, records);
    fs::remove_file(jpath(d.path(), n)).unwrap();
    let canonical = Uuid::from_u128(n).hyphenated().to_string();
    fs::write(
        jdir(d.path()).join(format!("{}.jnl", Uuid::from_u128(n).simple())),
        b"no hyphens",
    )
    .unwrap();
    fs::create_dir(jdir(d.path()).join(format!("{canonical}.jnl.rebase"))).unwrap();
    let opened = reopen(d.path(), s);
    assert!(
        opened.report.recovery.games.is_empty(),
        "{:?}",
        opened.report.recovery.games
    );
    assert!(!jpath(d.path(), n).exists());
    // And a directory with the canonical journal name is not a journal either.
    fs::remove_dir(jdir(d.path()).join(format!("{canonical}.jnl.rebase"))).unwrap();
    fs::create_dir(jpath(d.path(), n)).unwrap();
    let opened = reopen(d.path(), opened.store);
    assert!(
        opened.report.recovery.games.is_empty(),
        "{:?}",
        opened.report.recovery.games
    );
}

/// A store with no `store.id` has no identity to compare headers with: the scan reports
/// `scan_failed`, lists nothing and touches nothing (mutants D46, R03).
#[test]
fn a_store_without_an_identity_fails_the_scan_and_touches_nothing() {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 2));
    apply(&s, 1, records);
    let before = fs::read(jpath(d.path(), 1)).unwrap();
    drop(s);
    fs::remove_file(d.path().join("store.id")).unwrap();
    let opened = open_in(d.path());
    assert!(opened.report.recovery.scan_failed);
    assert!(opened.report.recovery.games.is_empty());
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), before);
}

// ----------------------------------------------------------------- the aside loop (F12)

/// A crash between the link and the removal of the aside move of an incomplete `.rebase` leaves
/// it under two names: the next settle finishes that move and makes no second copy (mutant F12).
#[test]
fn the_aside_move_of_an_incomplete_rebase_finishes_an_earlier_attempt() {
    let (d, _s) = world();
    let store = store_uuid(d.path());
    let jd = jdir(d.path());
    let game = Uuid::from_u128(1);
    let rebase = rebase_path(d.path(), 1);
    fs::write(&rebase, b"not a rebase").unwrap();
    fs::create_dir_all(damaged_dir(d.path())).unwrap();
    let first = damaged_dir(d.path()).join(format!("{}.1.rebase", game.hyphenated()));
    fs::hard_link(&rebase, &first).unwrap();
    let settled = settle_rebase_files(&jd, game, store).unwrap();
    assert_eq!(settled, Settled::MovedAsideIncomplete { damaged: first });
    assert_eq!(damaged_files(d.path(), 1, ".rebase").len(), 1);
    assert!(!rebase.exists());
    // A different file under `.1.rebase` moves `k` on, and both are kept.
    fs::write(&rebase, b"another cut file").unwrap();
    let settled = settle_rebase_files(&jd, game, store).unwrap();
    let second = damaged_dir(d.path()).join(format!("{}.2.rebase", game.hyphenated()));
    assert_eq!(settled, Settled::MovedAsideIncomplete { damaged: second });
    assert_eq!(damaged_files(d.path(), 1, ".rebase").len(), 2);
}

/// `move_aside_damaged` skips a `damaged/` name that is a symbolic link and never replaces it.
#[test]
fn the_damaged_move_never_follows_or_replaces_a_link_in_damaged() {
    let (d, _s) = world();
    let jd = jdir(d.path());
    let game = Uuid::from_u128(1);
    fs::write(jpath(d.path(), 1), b"journal bytes").unwrap();
    fs::create_dir_all(damaged_dir(d.path())).unwrap();
    let id = game.hyphenated();
    let link = damaged_dir(d.path()).join(format!("{id}.1.jnl"));
    symlink(jpath(d.path(), 1), &link).unwrap();
    let moved = move_aside_damaged(&jd, game).unwrap();
    assert_eq!(moved.k, 2);
    assert!(!moved.finished_earlier_attempt);
    assert!(is_symlink(&link));
    assert_eq!(fs::read(&moved.damaged).unwrap(), b"journal bytes");
    assert!(!jpath(d.path(), 1).exists());
}
