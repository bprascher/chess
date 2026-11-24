//! Part D4 failure injection (feature `test-support`): the failpoints D4 owns are wired
//! (`journal.rebase.db_committed`, `journal.close_final.linked`), an error at each leaves a state
//! that the next recovery finishes, and the whole journal reaches the applier in one call
//! (D3-10, D-87: one `apply.before_commit` per journal).
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
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use chess_domain::GameStatus;
use chess_store::fault::{self, FaultPlan};
use chess_store::journal::{RecoveryOutcome, ResumeError, scan_file};
use chess_store::{DataDir, JournalError, Store};
use common::{block_on, gid, jb};
use d4_support::{
    ENDED, apply, corrupt_record, damaged_files, done_path, dump, game_end, jpath, live, playing,
    rebase_path, reopen, write_journal,
};
use uuid::Uuid;

const EIO: i32 = 5;

fn store_uuid(dir: &Path) -> Uuid {
    DataDir::new(dir)
        .read_store_id()
        .unwrap()
        .unwrap()
        .store_uuid
}

fn resume(s: &Store, n: u128) -> Result<chess_store::journal::Recovered, ResumeError> {
    let journal = s.journal().unwrap();
    block_on(journal.open_for_resume(s, &gid(n))).map(|(_, recovered)| recovered)
}

/// A live game with four moves in the database and a damaged middle in its journal.
fn damaged() -> (tempfile::TempDir, Store, Vec<u8>) {
    let (d, s) = live();
    let records = write_journal(&s, 1, playing(1, 4));
    apply(&s, 1, records);
    let original = corrupt_record(&jpath(d.path(), 1), 3, store_uuid(d.path()).as_u128(), 1);
    (d, s, original)
}

/// The D4 names are registered and each has a production call site: a plan that delays them
/// counts hits when the code path runs.
#[test]
fn the_failpoints_d4_owns_are_registered_and_wired() {
    let registered: Vec<&str> = fault::all().collect();
    for name in ["journal.rebase.db_committed", "journal.close_final.linked"] {
        assert!(registered.contains(&name), "{name}");
    }
    // db_committed: once per rebase.
    let (d, s, _) = damaged();
    let guard = FaultPlan::new()
        .delay("journal.rebase.db_committed", std::time::Duration::ZERO)
        .delay("journal.close_final.linked", std::time::Duration::ZERO)
        .install(d.path())
        .unwrap();
    let journal = s.journal().unwrap();
    let (writer, _) = block_on(journal.open_for_resume(&s, &gid(1))).unwrap();
    assert_eq!(guard.hits("journal.rebase.db_committed"), 1);
    assert_eq!(guard.hits("journal.close_final.linked"), 0);
    writer.unwrap().close_final().unwrap();
    assert_eq!(guard.hits("journal.close_final.linked"), 1);
}

/// An error at `journal.rebase.db_committed` (window (b)): the database says `N`, the file is
/// still the damaged one, the `.rebase` is complete beside it. The next resume builds the same
/// file, step 2 changes nothing, and one `damaged/` file results.
#[test]
fn an_error_after_the_database_step_is_finished_by_the_next_resume() {
    let (d, s, original) = damaged();
    let guard = FaultPlan::new()
        .errno("journal.rebase.db_committed", EIO)
        .once()
        .install(d.path())
        .unwrap();
    let err = resume(&s, 1).unwrap_err();
    assert!(
        matches!(err, ResumeError::Journal(JournalError::WriteFailed { .. })),
        "{err:?}"
    );
    assert_eq!(guard.fired("journal.rebase.db_committed"), 1);
    assert_eq!(
        fs::read(jpath(d.path(), 1)).unwrap(),
        original,
        "still the damaged file"
    );
    assert!(rebase_path(d.path(), 1).exists());
    assert!(damaged_files(d.path(), 1, "").is_empty());
    let seq_after_step_2: i64 = common::raw(d.path())
        .query_row("SELECT journal_seq FROM game", [], |r| r.get(0))
        .unwrap();
    let rows = dump(d.path());
    // The next resume finishes it.
    let recovered = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased);
    assert_eq!(damaged_files(d.path(), 1, ".jnl").len(), 1);
    assert!(!rebase_path(d.path(), 1).exists());
    let scan = scan_file(
        &jpath(d.path(), 1),
        Uuid::from_u128(1),
        store_uuid(d.path()),
    )
    .unwrap();
    assert_eq!(i64::try_from(scan.last_seq()).unwrap(), seq_after_step_2);
    let _ = rows;
}

/// A failed database step (the plan applier is refused before its commit) removes the `.rebase`
/// it had written and leaves the damaged journal in place (window (a)).
#[test]
fn a_failed_database_step_leaves_the_damaged_journal_and_no_rebase() {
    let (d, s, original) = damaged();
    let guard = FaultPlan::new()
        .errno("apply.before_commit", EIO)
        .once()
        .install(d.path())
        .unwrap();
    let err = resume(&s, 1).unwrap_err();
    assert!(matches!(err, ResumeError::Store(_)), "{err:?}");
    assert_eq!(guard.fired("apply.before_commit"), 1);
    assert_eq!(fs::read(jpath(d.path(), 1)).unwrap(), original);
    assert!(!rebase_path(d.path(), 1).exists());
    assert!(damaged_files(d.path(), 1, "").is_empty());
    // The retry succeeds.
    let recovered = resume(&s, 1).unwrap();
    assert!(recovered.report.rebased);
}

/// An error at the first directory sync of the swap leaves nothing half-moved.
#[test]
fn an_error_inside_the_swap_is_finished_by_the_next_recovery() {
    for point in [
        "journal.rebase.old_linked",
        "journal.rebase.old_aside",
        "journal.rebase.new_linked",
    ] {
        let (d, s, original) = damaged();
        let guard = FaultPlan::new()
            .errno(point, EIO)
            .once()
            .install(d.path())
            .unwrap();
        assert!(resume(&s, 1).is_err(), "{point}");
        assert_eq!(guard.fired(point), 1);
        drop(guard);
        // Through the startup scan this time.
        let opened = reopen(d.path(), s);
        assert!(
            matches!(
                opened.report.recovery.outcome_of(gid(1)),
                Some(RecoveryOutcome::Rebased { .. } | RecoveryOutcome::Reconciled { .. })
            ),
            "{point}: {:?}",
            opened.report.recovery
        );
        let aside = damaged_files(d.path(), 1, ".jnl");
        assert_eq!(aside.len(), 1, "{point}");
        assert_eq!(fs::read(&aside[0]).unwrap(), original, "{point}");
        assert!(!rebase_path(d.path(), 1).exists(), "{point}");
        for (name, _, links) in d4_support::journal_tree(d.path()) {
            assert_eq!(links, 1, "{point}: {name}");
        }
    }
}

/// B-R03 with an injected error instead of a kill: `close_final` fails between the link and the
/// removal; the journal has two names; the next start finishes the move.
#[test]
fn b_r03_an_error_between_the_link_and_the_removal_is_repaired_by_the_next_start() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    let records = write_journal(&s, 1, payloads);
    apply(&s, 1, records);
    let journal = s.journal().unwrap();
    let (writer, _) = block_on(journal.open_for_resume(&s, &gid(1))).unwrap();
    let guard = FaultPlan::new()
        .errno("journal.close_final.linked", EIO)
        .once()
        .install(d.path())
        .unwrap();
    assert!(writer.unwrap().close_final().is_err());
    assert_eq!(fs::metadata(jpath(d.path(), 1)).unwrap().nlink(), 2);
    drop(guard);
    let opened = reopen(d.path(), s);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::FinishedClose)
    );
    assert_eq!(fs::metadata(done_path(d.path(), 1)).unwrap().nlink(), 1);
    assert!(!jpath(d.path(), 1).exists());
}

/// D3-10 and D-87: the whole journal reaches the applier in ONE call. One transaction commit
/// per resume of an intact journal, whatever its length, with a `GameEnd` and a `Voided`
/// in it (a split call would finalize the `GameEnd` first).
#[test]
fn the_whole_journal_reaches_the_applier_in_one_call() {
    let (d, s) = live();
    let mut payloads = playing(1, 6);
    payloads.push(game_end(ENDED));
    payloads.push(d4_support::voided(Some(1)));
    write_journal(&s, 1, payloads);
    let guard = FaultPlan::new()
        .delay("apply.before_commit", std::time::Duration::ZERO)
        .install(d.path())
        .unwrap();
    let recovered = resume(&s, 1).unwrap();
    assert_eq!(recovered.records.len(), 9);
    assert_eq!(
        guard.hits("apply.before_commit"),
        1,
        "one apply_records call, one commit"
    );
    assert_eq!(
        block_on(s.get_game(gid(1))).unwrap().status,
        GameStatus::Voided
    );
    // The same at startup: one call per journal.
    let (d2, s2) = live();
    let mut payloads = playing(1, 5);
    payloads.push(game_end(ENDED));
    write_journal(&s2, 1, payloads);
    let guard = FaultPlan::new()
        .delay("apply.before_commit", std::time::Duration::ZERO)
        .install(d2.path())
        .unwrap();
    let opened = reopen(d2.path(), s2);
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Finalized)
    );
    assert_eq!(guard.hits("apply.before_commit"), 1);
    let _ = (io::ErrorKind::Other, dump(d2.path()), jb::mv(0));
}
