//! Shared helpers of the part D5 tests: a world of Players, raw game seeding (the finalize
//! command is part D2, so the tests write finished rows with SQL) and a controllable clock.
#![allow(
    dead_code,
    reason = "each D5 test file uses a different subset of these helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers here exist to abort a failing fixture"
)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chess_store::{Store, StoreOptions};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::common::{block_on, computer, human, raw};

/// Humans 1 (Ann), 2 (Bob), 3 (Cy); computers 10 and 11 (Strict rating 800: band Novice).
pub fn world(dir: &Path) -> Store {
    let store = crate::common::open_in(dir).store;
    populate(&store);
    store
}

fn populate(store: &Store) {
    for (n, name) in [(1, "Ann"), (2, "Bob"), (3, "Cy")] {
        block_on(store.register_player(human(n, name))).unwrap();
    }
    block_on(store.insert_computers(vec![
        computer(10, "Bot A", 1, 0, 3),
        computer(11, "Bot B", 1, 1, 4),
    ]))
    .unwrap();
}

/// A store whose clock is the returned counter: the test sets the time.
pub fn world_with_clock(dir: &Path, start: i64) -> (Store, Arc<AtomicI64>) {
    let now = Arc::new(AtomicI64::new(start));
    let clock = Arc::clone(&now);
    let options = StoreOptions::new(chess_store::DataDir::new(dir))
        .with_clock(Arc::new(move || clock.load(Ordering::SeqCst)));
    let store = block_on(Store::open(options)).unwrap().store;
    populate(&store);
    (store, now)
}

/// Opens the same directory again with the clock (a new start of the process).
pub fn reopen_with_clock(dir: &Path, now: &Arc<AtomicI64>) -> Store {
    let clock = Arc::clone(now);
    let options = StoreOptions::new(chess_store::DataDir::new(dir))
        .with_clock(Arc::new(move || clock.load(Ordering::SeqCst)));
    block_on(Store::open(options)).unwrap().store
}

#[derive(Clone)]
pub struct G {
    pub n: u128,
    /// `None` for an imported game.
    pub white: Option<u128>,
    pub black: Option<u128>,
    pub matchup: &'static str,
    pub mode: &'static str,
    pub status: &'static str,
    pub result: Option<&'static str>,
    pub custom: bool,
    pub started_at: i64,
    pub plies: u32,
    pub active_ms: Option<i64>,
}

impl G {
    /// A completed, rated, strict human-human game, 10 plies, 60 000 ms.
    pub fn done(n: u128, white: u128, black: u128, result: &'static str) -> G {
        G {
            n,
            white: Some(white),
            black: Some(black),
            matchup: "hh",
            mode: "strict",
            status: "completed",
            result: Some(result),
            custom: false,
            started_at: 10_000 + i64::try_from(n).unwrap(),
            plies: 10,
            active_ms: Some(60_000),
        }
    }
}

pub fn player_key(conn: &Connection, n: u128) -> i64 {
    conn.query_row(
        "SELECT id FROM player WHERE uuid = ?1",
        [Uuid::from_u128(n).hyphenated().to_string()],
        |r| r.get(0),
    )
    .unwrap()
}

pub fn game_key(conn: &Connection, n: u128) -> i64 {
    conn.query_row(
        "SELECT id FROM game WHERE uuid = ?1",
        [Uuid::from_u128(n).hyphenated().to_string()],
        |r| r.get(0),
    )
    .unwrap()
}

/// Inserts the game row and `plies` move rows. Constraint-consistent for every status used.
pub fn seed(dir: &Path, game: &G) {
    let conn = raw(dir);
    seed_on(&conn, game);
}

pub fn seed_on(conn: &Connection, game: &G) {
    let white = game.white.map(|n| player_key(conn, n));
    let black = game.black.map(|n| player_key(conn, n));
    let imported = game.status == "imported";
    let self_play = game.white.is_some() && game.white == game.black;
    conn.execute(
        "INSERT INTO game (uuid, white_id, black_id, matchup, mode, rated, self_play, custom_start, \
         start_fen, status, result, started_at, total_active_ms, content_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            Uuid::from_u128(game.n).hyphenated().to_string(),
            white,
            black,
            (!imported).then_some(game.matchup),
            (!imported).then_some(game.mode),
            i64::from(!imported && !game.custom),
            i64::from(self_play),
            i64::from(game.custom),
            game.custom.then_some("8/8/8/8/8/8/8/K6k w - - 0 1"),
            game.status,
            game.result,
            game.started_at,
            game.active_ms,
            (game.status != "in_progress").then_some("hash"),
        ],
    )
    .unwrap();
    let key = conn.last_insert_rowid();
    for ply in 1..=game.plies {
        conn.execute(
            "INSERT INTO move (game_id, ply, san, uci, fen_after) VALUES (?1, ?2, 'x', 'a1a2', 'f')",
            params![key, ply],
        )
        .unwrap();
    }
}
