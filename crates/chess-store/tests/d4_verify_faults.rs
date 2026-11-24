//! Part D4 verification, failure injection (feature `test-support`): a start whose database step
//! cannot commit leaves the journal where it is and reports `Queued` (mutant 25).
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

use chess_domain::GameStatus;
use chess_store::fault::FaultPlan;
use chess_store::journal::RecoveryOutcome;
use common::{block_on, gid, open_in};
use d4_support::{ENDED, game_end, jpath, live, playing, write_journal};

/// The journal ends in a `GameEnd`; the finalize cannot commit (an I/O error before the commit,
/// a transient one). The scan reports `Queued`, keeps the journal in `journal/`, and the game is
/// still in progress; once the fault is gone the next start finalizes it.
#[test]
fn a_finalize_that_cannot_commit_at_startup_is_queued_and_the_journal_stays() {
    let (d, s) = live();
    let mut payloads = playing(1, 2);
    payloads.push(game_end(ENDED));
    write_journal(&s, 1, payloads);
    drop(s);
    let guard = FaultPlan::new()
        .errno("apply.before_commit", 5)
        .install(d.path())
        .unwrap();
    let opened = open_in(d.path());
    assert_eq!(
        opened.report.recovery.outcome_of(gid(1)),
        Some(&RecoveryOutcome::Queued)
    );
    assert!(jpath(d.path(), 1).exists(), "the journal stays");
    assert_eq!(
        block_on(opened.store.get_game(gid(1))).unwrap().status,
        GameStatus::InProgress
    );
    drop(guard);
    drop(opened);
    let again = open_in(d.path());
    let outcome = again.report.recovery.outcome_of(gid(1)).cloned();
    assert!(
        matches!(
            outcome,
            Some(RecoveryOutcome::Finalized | RecoveryOutcome::ClosedLeftover)
        ),
        "{outcome:?}"
    );
    assert_eq!(
        block_on(again.store.get_game(gid(1))).unwrap().status,
        GameStatus::Completed
    );
}
