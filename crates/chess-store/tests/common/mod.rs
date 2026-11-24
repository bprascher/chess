//! Shared test helpers: a current-thread runtime, a fake clock, builders and a raw SQL handle
//! on the store's database for seeding and for checking constraints. Every test uses its own
//! temporary data directory (test-strategy "Determinism").
#![allow(
    dead_code,
    reason = "each test file uses a different subset of these helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

pub mod child;

use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chess_domain::{
    Category, ClockSpec, GameId, Matchup, Mode, Personality, PlayerId, RosterSeed, WallMs,
};
use chess_store::{
    DataDir, NewComputer, NewGame, NewPlayer, OpenedStore, Store, StoreError, StoreOptions,
};
use rusqlite::Connection;
use uuid::Uuid;

/// `block_on` with an outer real-time limit (FIND-D3R-11): a call that depends on a frozen
/// injected clock fails after `limit` instead of hanging CI when its timer or sync code regresses.
pub fn block_on_within<F: Future>(limit: std::time::Duration, future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(limit, future)
                .await
                .expect("the call did not finish within the outer limit")
        })
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

/// A clock that returns `start`, `start + 1`, ... so rows get distinct, predictable times.
pub fn ticking_clock(start: i64) -> chess_store::WallClock {
    let counter = Arc::new(AtomicI64::new(start));
    Arc::new(move || counter.fetch_add(1, Ordering::SeqCst))
}

pub fn options(dir: &Path) -> StoreOptions {
    StoreOptions::new(DataDir::new(dir)).with_clock(ticking_clock(1_000))
}

pub fn open_in(dir: &Path) -> OpenedStore {
    block_on(Store::open(options(dir))).unwrap()
}

pub fn try_open(dir: &Path) -> Result<OpenedStore, StoreError> {
    block_on(Store::open(options(dir)))
}

/// A second connection to the store's database, foreign keys on, for seeding and inspection.
pub fn raw(dir: &Path) -> Connection {
    let conn = Connection::open(dir.join("chess.db")).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    conn
}

pub fn pid(n: u128) -> PlayerId {
    PlayerId::from_uuid(Uuid::from_u128(n))
}

pub fn gid(n: u128) -> GameId {
    GameId::from_uuid(Uuid::from_u128(n))
}

pub fn human(n: u128, name: &str) -> NewPlayer {
    NewPlayer {
        id: pid(n),
        name: name.to_owned(),
        start_category: Category::Novice,
        strict_elo: 1200,
        training_elo: 1200,
        preferences_json: "{\"v\":1}".to_owned(),
    }
}

pub fn personality(skill: u8) -> Personality {
    Personality::new(skill, 8, 500, Some(1500), 20, 2).unwrap()
}

pub fn computer(n: u128, name: &str, seed: u32, index: u32, skill: u8) -> NewComputer {
    NewComputer {
        id: pid(n),
        name: name.to_owned(),
        start_category: Category::Beginner,
        roster_seed: RosterSeed::new(seed),
        roster_index: index,
        personality: personality(skill),
        strict_elo: 800,
        training_elo: 800,
    }
}

pub fn new_game(
    n: u128,
    white: PlayerId,
    black: PlayerId,
    matchup: Matchup,
    mode: Mode,
) -> NewGame {
    NewGame {
        id: gid(n),
        white,
        black,
        matchup,
        mode,
        rated: true,
        start_fen: None,
        clock: None,
        started_at: WallMs(5_000 + i64::try_from(n).unwrap()),
    }
}

pub fn blitz() -> ClockSpec {
    ClockSpec::from_seconds(300, 2).unwrap()
}

/// The migrations plus a synthetic version 2 (SDD 3.11.5 `Migrator::with`).
pub fn v2() -> chess_store::Migrator {
    chess_store::Migrator::with(&[
        chess_store::MIGRATIONS[0],
        chess_store::Migration {
            version: 2,
            sql: "CREATE TABLE note (id INTEGER PRIMARY KEY, body TEXT NOT NULL);",
        },
    ])
    .unwrap()
}

/// A store at schema version 1 with its identity file, made without `Store::open` (an "older
/// build's" store: the registry table exists and is empty).
pub fn fixture_v1() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let conn = Connection::open(dir.path().join("chess.db")).unwrap();
    conn.execute_batch(include_str!("../fixtures/schema-v1.sql"))
        .unwrap();
    drop(conn);
    DataDir::new(dir.path())
        .write_store_id(&chess_store::StoreId {
            store_uuid: Uuid::from_u128(5),
            created_at: 1,
        })
        .unwrap();
    dir
}

/// A pid that existed and is gone.
pub fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

/// Journal builders (part B).
pub mod jb {
    use chess_store::journal::format::{
        ClockPayload, ClockText, MovePayload, Payload, SeatKind, StartPayload, UuidText,
    };
    use uuid::Uuid;

    pub fn start(game: u128) -> Payload {
        Payload::Start(StartPayload {
            game: UuidText(Uuid::from_u128(game)),
            white: UuidText(Uuid::from_u128(1)),
            black: UuidText(Uuid::from_u128(2)),
            white_kind: SeatKind::Human,
            black_kind: SeatKind::Human,
            matchup: "hh".into(),
            mode: "strict".into(),
            custom_start: false,
            start_fen: None,
            clock_spec: ClockText::parse("300+2"),
            rated: true,
            self_play: false,
        })
    }

    /// A `Move` with journal ply `ply` (zero-based), clocks both set.
    pub fn mv(ply: u32) -> Payload {
        Payload::Move(MovePayload {
            ply,
            uci: format!("u{ply}"),
            think_ms: 10 + u64::from(ply),
            white_ms: Some(300_000 - u64::from(ply)),
            black_ms: Some(300_000 + u64::from(ply)),
            san: format!("s{ply}"),
            fen_after: format!("fen{ply}"),
        })
    }

    pub fn untimed_move(ply: u32) -> Payload {
        match mv(ply) {
            Payload::Move(mut m) => {
                m.white_ms = None;
                m.black_ms = None;
                Payload::Move(m)
            }
            other => other,
        }
    }

    pub fn clock(white: u64, black: u64) -> Payload {
        Payload::Clock(ClockPayload {
            white_ms: Some(white),
            black_ms: Some(black),
            running_side: None,
        })
    }
}
