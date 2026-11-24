//! BM-ST-02: open, `quick_check` and the journal scan on a seeded database (the store's share of
//! NFR-1, which is budgeted in `chess-app`). The fixture: 2 000 finished games with 40 plies
//! each, 20 journals of interrupted games (30 plies) and 5 journals of games that ended and were
//! not finalized. The test prints the timings and asserts only what was recovered and a generous
//! sanity ceiling, so a loaded machine does not make it flaky. Run
//! `cargo test -p chess-store --release --test startup_bench -- --nocapture` for the release
//! figures.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d4_support;
mod d5_support;

use std::time::{Duration, Instant};

use chess_store::Store;
use chess_store::journal::RecoveryOutcome;
use common::{block_on, open_in, raw};
use d4_support::{ENDED, game_end, playing, seed_players, write_journal};
use d5_support::{G, seed_on};

const CEILING: Duration = Duration::from_secs(10);

#[test]
fn bm_st_02_open_quick_check_and_scan_on_a_seeded_database() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    seed_players(&s);
    {
        let mut conn = raw(dir.path());
        let tx = conn.transaction().unwrap();
        for n in 1..=2_000u128 {
            let mut g = G::done(n + 1_000, 1, 2, "1-0");
            g.plies = 40;
            g.started_at = 1_000 + n as i64;
            seed_on(&tx, &g);
        }
        tx.commit().unwrap();
    }
    // 25 in-progress rows (seeded directly: the slot rule allows one human game per mode).
    {
        let mut conn = raw(dir.path());
        let tx = conn.transaction().unwrap();
        for n in 1..=25u128 {
            let mut g = G::done(n, 1, 2, "1-0");
            g.status = "in_progress";
            g.result = None;
            g.active_ms = None;
            g.plies = 0;
            g.started_at = 5_000 + n as i64;
            seed_on(&tx, &g);
        }
        tx.commit().unwrap();
    }
    let started = 25usize;
    let mut finishing = 0;
    for n in 1..=25u128 {
        let mut payloads = playing(n, 8);
        if n <= 5 {
            payloads.push(game_end(ENDED + n as i64));
            finishing += 1;
        }
        write_journal(&s, n, payloads);
    }
    drop(s);
    let first = Instant::now();
    let opened = block_on(Store::open(common::options(dir.path()))).unwrap();
    let took = first.elapsed();
    eprintln!(
        "BM-ST-02 open + quick_check + scan of {started} journals ({finishing} finalized): {took:?}"
    );
    assert!(took < CEILING, "{took:?}");
    let report = &opened.report.recovery;
    assert_eq!(report.games.len(), started);
    assert_eq!(
        report
            .games
            .iter()
            .filter(|g| g.outcome == RecoveryOutcome::Finalized)
            .count(),
        finishing
    );
    drop(opened);
    // The second start has nothing to finalize: the same open on the settled store.
    let again = Instant::now();
    let opened = block_on(Store::open(common::options(dir.path()))).unwrap();
    eprintln!(
        "BM-ST-02 second open ({} journals scanned): {:?}",
        opened.report.recovery.games.len(),
        again.elapsed()
    );
}
