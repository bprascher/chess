//! BM-ST-03: `stats_rows` and `games_for_history` on a 10 000-game fixture (the store half of
//! FF-5; the NFR budget applies to the whole statistics path in `chess-app`, with
//! `chess-domain::aggregate`, which this crate does not call). The test prints the timings and
//! asserts only the row counts and a generous sanity ceiling, so a loaded machine does not
//! make it flaky. Run `cargo test -p chess-store --release --test stats_bench -- --nocapture`
//! for the release figures.
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
mod d5_support;

use std::time::{Duration, Instant};

use chess_domain::GameStatus;
use chess_store::StatsSide;
use common::{block_on, pid, raw};
use d5_support::{G, player_key, seed_on, world};
use tempfile::TempDir;

const GAMES: u128 = 10_000;
const PLIES: u32 = 40;
/// A sanity ceiling per query, in debug builds on a loaded machine; not the NFR budget.
const CEILING: Duration = Duration::from_secs(10);

#[test]
fn bm_st_03_ten_thousand_game_fixture() {
    let dir = TempDir::new().unwrap();
    let s = world(dir.path());
    {
        let mut conn = raw(dir.path());
        let tx = conn.transaction().unwrap();
        let _ = player_key(&tx, 1);
        for n in 1..=GAMES {
            let mut g = G::done(n, 1, if n % 2 == 0 { 2 } else { 3 }, "1-0");
            g.plies = PLIES;
            g.started_at = 1_000 + n as i64;
            match n % 10 {
                0 => {
                    g.status = "voided";
                    g.result = None;
                    g.active_ms = None;
                }
                1 => g.custom = true,
                2 => {
                    // Ann black.
                    g.white = g.black;
                    g.black = Some(1);
                }
                _ => {}
            }
            seed_on(&tx, &g);
        }
        tx.commit().unwrap();
    }
    let timed = |name: &str, f: &mut dyn FnMut() -> usize| {
        let started = Instant::now();
        let n = f();
        let took = started.elapsed();
        eprintln!("BM-ST-03 {name}: {n} rows in {took:?}");
        assert!(took < CEILING, "{name} took {took:?}");
        n
    };
    let all = timed("stats_rows (no key)", &mut || {
        block_on(s.stats_rows(pid(1), None, None, None))
            .unwrap()
            .len()
    });
    assert_eq!(all, GAMES as usize);
    let white = timed("stats_rows (side white)", &mut || {
        block_on(s.stats_rows(pid(1), None, Some(StatsSide::White), None))
            .unwrap()
            .len()
    });
    assert_eq!(white, 9_000, "n % 10 == 2 are Ann-black games");
    timed("stats_rows (window, limit 100)", &mut || {
        let rows =
            block_on(s.stats_rows(pid(1), Some(chess_domain::WallMs(6_000)), None, Some(100)))
                .unwrap();
        assert_eq!(rows[0].started_at.0, 6_000);
        rows.len()
    });
    timed("head_to_head_rows", &mut || {
        block_on(s.head_to_head_rows(pid(1), pid(2), None, None))
            .unwrap()
            .len()
    });
    let history = timed("games_for_history", &mut || {
        let rows = block_on(s.games_for_history(pid(1), None, None)).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|g| g.status == GameStatus::Voided)
                .count(),
            1_000
        );
        rows.len()
    });
    assert_eq!(history, GAMES as usize);
}
