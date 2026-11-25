//! GATE-110 condition 1 (FIND-D3RL-06): the `id` tie-break of the start-order queries
//! (`ORDER BY started_at, id`, FIND-D5R-03 and FIND-F2-03), pinned **without** a planted index.
//!
//! `f2_03_*` in `stats_rows.rs` depends on two expression indexes that steer the planner. This
//! file creates no index. It makes the tie order depend on the data instead: the tied games have
//! mixed seats for the Player asked about, and the seat condition `white_id = ? OR black_id = ?`
//! is answered by SQLite with a multi-index OR (white seats first, then black seats), so without
//! `id` in the `ORDER BY` a tie between a game where the Player is black (lower `id`) and one
//! where the Player is white (higher `id`) comes out in the wrong order. A query with a
//! full-table scan (`list_games` with no player) returns ties in `id` order anyway, so that
//! query is pinned by the source text in `tie_break_is_named_in_every_order_by` below.
//! No feature is needed.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

mod common;
mod d5_support;

use chess_domain::{GameId, WallMs};
use chess_store::{GameFilter, GameRecord, StatsGameRow, Store};
use common::{block_on, gid, pid, raw};
use d5_support::{G, seed, world};
use tempfile::TempDir;

fn ids(rows: Vec<GameRecord>) -> Vec<GameId> {
    rows.iter().map(|g| g.id).collect()
}

fn ms(rows: Vec<StatsGameRow>) -> Vec<u64> {
    rows.iter().map(|r| r.active_ms).collect()
}

fn filter(limit: Option<u32>) -> GameFilter {
    GameFilter {
        player: None,
        status: None,
        from: None,
        limit,
    }
}

/// Games 7, 3 and 5 are inserted in that order, so their row keys (`id`) are 1, 2, 3 and the tie
/// order is 7, 3, 5, which is neither ascending nor descending by game number or uuid. They share
/// `started_at` 5 000;/// game 9 started earlier and game 1 later. `active_ms` is the game number, so a statistics row
/// names its game. Start-then-id order is `9, 7, 3, 5, 1`.
fn seeded(dir: &std::path::Path) -> Store {
    let s = world(dir);
    for (n, start, ann_white) in [
        (7_u128, 5_000_i64, false),
        (3, 5_000, true),
        (5, 5_000, false),
        (9, 1_000, true),
        (1, 9_000, false),
    ] {
        let mut g = if ann_white {
            G::done(n, 1, 2, "1-0")
        } else {
            G::done(n, 2, 1, "0-1")
        };
        g.started_at = start;
        g.active_ms = Some(i64::try_from(n).unwrap());
        seed(dir, &g);
    }
    s
}

fn assert_start_then_id_order(s: &Store) {
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), None, None, None)).unwrap()),
        [9, 7, 3, 5, 1]
    );
    for (limit, want) in [
        (1_u32, vec![9_u64]),
        (2, vec![9, 7]),
        (3, vec![9, 7, 3]),
        (4, vec![9, 7, 3, 5]),
    ] {
        assert_eq!(
            ms(block_on(s.stats_rows(pid(1), None, None, Some(limit))).unwrap()),
            want,
            "stats_rows limit {limit}"
        );
        assert_eq!(
            ms(block_on(s.head_to_head_rows(pid(1), pid(2), None, Some(limit))).unwrap()),
            want,
            "head_to_head_rows limit {limit}"
        );
    }
    // The window cuts game 9, so the first of the tied games is the oldest by `(start, id)`.
    assert_eq!(
        ms(block_on(s.stats_rows(pid(1), Some(WallMs(5_000)), None, Some(2))).unwrap()),
        [7, 3],
        "stats_rows window then limit"
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, None)).unwrap()),
        [gid(9), gid(7), gid(3), gid(5), gid(1)]
    );
    assert_eq!(
        ids(block_on(s.games_for_history(pid(1), None, Some(3))).unwrap()),
        [gid(9), gid(7), gid(3)]
    );
    assert_eq!(
        ids(block_on(s.list_games(filter(None))).unwrap()),
        [gid(9), gid(7), gid(3), gid(5), gid(1)]
    );
    for (limit, want) in [
        (2_u32, vec![gid(9), gid(7)]),
        (3, vec![gid(9), gid(7), gid(3)]),
    ] {
        assert_eq!(
            ids(block_on(s.list_games(filter(Some(limit)))).unwrap()),
            want,
            "list_games limit {limit}"
        );
    }
}

/// Mutants S02 (statistics) and S05 (`list_games`) of review fixes-2 die here, with no planted
/// index: see the module comment.
#[test]
fn equal_start_times_are_ordered_by_id_whichever_seat_the_player_has() {
    let dir = TempDir::new().unwrap();
    let s = seeded(dir.path());
    assert_start_then_id_order(&s);
}

/// The same with the planner's statistics gathered (`ANALYZE`): the answer does not depend on
/// which plan SQLite picks for the seat `OR`.
#[test]
fn equal_start_times_are_ordered_by_id_after_analyze() {
    let dir = TempDir::new().unwrap();
    let s = seeded(dir.path());
    raw(dir.path()).execute_batch("ANALYZE").unwrap();
    assert_start_then_id_order(&s);
}

/// The `ORDER BY` clauses of `text` that sort by `started_at`, each cut at the first `LIMIT`, quote
/// or closing parenthesis after it, with all white space squeezed to single spaces.
fn start_order_clauses(text: &str) -> Vec<String> {
    let squeezed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = Vec::new();
    let mut rest = squeezed.as_str();
    while let Some(at) = rest.find("ORDER BY ") {
        let tail = &rest[at + "ORDER BY ".len()..];
        let end = tail.find(['"', ')']).unwrap_or(tail.len());
        let clause = tail[..end].split(" LIMIT").next().unwrap_or("").trim();
        if clause.contains("started_at") {
            out.push(clause.to_owned());
        }
        rest = tail;
    }
    out
}

fn source(name: &str) -> String {
    let path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "repo", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Planner-independent pin: every `ORDER BY` that sorts by `started_at` in the two repositories
/// ends in `g.id` (the unique row key), so ties have one answer on any plan. `list_games` with no
/// player filter is a full scan that returns ties in `id` order whatever its `ORDER BY` says, and
/// no data can expose a dropped `g.id` there; this test is what pins it.
#[test]
fn tie_break_is_named_in_every_order_by() {
    for (file, expected) in [("stats.rs", 1), ("game.rs", 1)] {
        let clauses = start_order_clauses(&source(file));
        assert_eq!(clauses.len(), expected, "{file}: {clauses:?}");
        for clause in clauses {
            assert!(clause.ends_with(", g.id"), "{file}: ORDER BY {clause}");
        }
    }
}

#[test]
fn the_order_by_scanner_finds_what_it_is_meant_to_find() {
    let sql = "SELECT 1 ORDER BY g.started_at IS NULL, g.started_at, g.id LIMIT ?4";
    assert_eq!(
        start_order_clauses(sql),
        ["g.started_at IS NULL, g.started_at, g.id"]
    );
    let dropped = "FROM g ORDER BY g.started_at LIMIT ?4";
    assert!(!start_order_clauses(dropped)[0].ends_with(", g.id"));
    let multi = "ORDER BY g.started_at,\n  g.id LIMIT 1 ORDER BY ply";
    assert_eq!(start_order_clauses(multi), ["g.started_at, g.id"]);
}
