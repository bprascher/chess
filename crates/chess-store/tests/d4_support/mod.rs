//! Part D4 test helpers: a store with players and games, journals written to disk the way a
//! session writes them, damage of a journal file, and a dump of the database.
#![allow(
    dead_code,
    reason = "each test file uses a different subset of these helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the helpers exist to abort a failing fixture"
)]

use std::fs;
use std::path::{Path, PathBuf};

use chess_domain::{GameId, Matchup, Mode};
use chess_store::journal::format::{
    EndReason, GameEndPayload, HEADER_LEN, JournalRecord, Payload, SeatKind, StartPayload,
    UuidText, VoidReason, VoidedPayload, encode_record,
};
use chess_store::journal::{MoveJournal, scan_bytes};
use chess_store::{Store, StoreOptions};
use rusqlite::Connection;
use tempfile::TempDir;
use uuid::Uuid;

use crate::common::{block_on, computer, gid, human, jb, new_game, open_in, pid};

pub const ENDED: i64 = 9_000_000;
pub const HASH: &str = "sha256:v1:aa";

/// Humans 1 (Ann) and 2 (Bob), computers 10 and 11.
pub fn world() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let s = open_in(dir.path()).store;
    seed_players(&s);
    (dir, s)
}

pub fn seed_players(s: &Store) {
    for (n, name) in [(1, "Ann"), (2, "Bob")] {
        block_on(s.register_player(human(n, name))).unwrap();
    }
    block_on(s.insert_computers(vec![
        computer(10, "Bot A", 1, 0, 3),
        computer(11, "Bot B", 1, 1, 4),
    ]))
    .unwrap();
}

/// `world` plus the live game 1, Ann (white) against Bob, strict, rated, untimed row.
pub fn live() -> (TempDir, Store) {
    let (d, s) = world();
    start(&s, 1);
    (d, s)
}

/// A human-human game `n`, Ann against Bob (one unfinished slot per mode: only one at a time).
pub fn start(s: &Store, n: u128) {
    block_on(s.start_game(new_game(
        n,
        pid(1),
        pid(2),
        Matchup::HumanHuman,
        Mode::Strict,
    )))
    .unwrap();
}

pub fn jdir(dir: &Path) -> PathBuf {
    dir.join("journal")
}

pub fn jpath(dir: &Path, game: u128) -> PathBuf {
    jdir(dir).join(format!("{}.jnl", Uuid::from_u128(game).hyphenated()))
}

pub fn rebase_path(dir: &Path, game: u128) -> PathBuf {
    jdir(dir).join(format!("{}.jnl.rebase", Uuid::from_u128(game).hyphenated()))
}

pub fn damaged_dir(dir: &Path) -> PathBuf {
    jdir(dir).join("damaged")
}

pub fn done_path(dir: &Path, game: u128) -> PathBuf {
    jdir(dir)
        .join("done")
        .join(format!("{}.jnl", Uuid::from_u128(game).hyphenated()))
}

/// Files in `journal/damaged/` of this game whose name ends with `suffix`, sorted.
pub fn damaged_files(dir: &Path, game: u128, suffix: &str) -> Vec<PathBuf> {
    let prefix = format!("{}.", Uuid::from_u128(game).hyphenated());
    let mut out: Vec<PathBuf> = fs::read_dir(damaged_dir(dir))
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.starts_with(&prefix) && name.ends_with(suffix)
                })
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

pub fn rec(seq: u64, payload: Payload) -> JournalRecord {
    JournalRecord {
        seq,
        wall_ms: 1_000 + i64::try_from(seq).unwrap(),
        payload,
    }
}

pub fn game_end(ended_at: i64) -> Payload {
    Payload::GameEnd(GameEndPayload {
        result: "1-0".into(),
        reason: EndReason::Checkmate,
        ended_at,
        eco: None,
        opening_name: None,
        analysis_used: false,
        content_hash: HASH.into(),
        white_time_used_ms: 111,
        black_time_used_ms: 222,
        total_active_ms: 333,
    })
}

pub fn voided(by: Option<u128>) -> Payload {
    Payload::Voided(VoidedPayload {
        reason: VoidReason::Reset,
        reset_player: by.map(|n| UuidText(Uuid::from_u128(n))),
        content_hash: "sha256:v1:void".into(),
        ended_at: 8_000_000,
    })
}

/// A `Start` for game `n`: `white` against `black`, with the kinds given.
pub fn start_for(n: u128, white: u128, black: u128, wk: SeatKind, bk: SeatKind) -> Payload {
    let Payload::Start(base) = jb::start(n) else {
        panic!()
    };
    let matchup = match (wk, bk) {
        (SeatKind::Human, SeatKind::Human) => "hh",
        (SeatKind::Computer, SeatKind::Computer) => "cc",
        _ => "hc",
    };
    Payload::Start(StartPayload {
        white: UuidText(Uuid::from_u128(white)),
        black: UuidText(Uuid::from_u128(black)),
        white_kind: wk,
        black_kind: bk,
        matchup: matchup.into(),
        clock_spec: None,
        ..base
    })
}

/// The journal of game `n` as a session writes it: `create`, one append per record (wall time
/// from `rec`), then the writer is dropped (the lock is released, as after a crash).
pub fn write_journal(s: &Store, n: u128, payloads: Vec<Payload>) -> Vec<JournalRecord> {
    let journal: MoveJournal = s.journal().unwrap();
    let writer = journal.create(&gid(n)).unwrap();
    let mut out = Vec::new();
    for (i, payload) in payloads.into_iter().enumerate() {
        let seq = u64::try_from(i).unwrap() + 1;
        let record = rec(seq, payload);
        let got = block_on(writer.append(record.wall_ms, record.payload.clone())).unwrap();
        assert_eq!(got, seq);
        out.push(record);
    }
    drop(writer);
    out
}

/// Start, `moves` moves (journal ply 0..moves) and one closing `Clock`.
pub fn playing(n: u128, moves: u32) -> Vec<Payload> {
    let mut out = vec![jb::start(n)];
    for ply in 0..moves {
        out.push(jb::mv(ply));
    }
    out
}

pub fn apply(s: &Store, n: u128, records: Vec<JournalRecord>) {
    block_on(s.apply_records(gid(n), records)).unwrap();
}

/// The byte offset at which record `index` (0-based, counting from `Start`) of `bytes` begins.
pub fn record_offset(bytes: &[u8], index: usize, store: u128, game: u128) -> usize {
    let scan = scan_bytes(bytes, Uuid::from_u128(game), Uuid::from_u128(store)).unwrap();
    HEADER_LEN
        + scan.records[..index]
            .iter()
            .map(|r| encode_record(r).unwrap().len())
            .sum::<usize>()
}

/// Flips a byte inside record `index` (its CRC then fails): a damaged middle when records
/// follow it.
pub fn corrupt_record(path: &Path, index: usize, store: u128, game: u128) -> Vec<u8> {
    let mut bytes = fs::read(path).unwrap();
    let at = record_offset(&bytes, index, store, game);
    bytes[at + 14] ^= 0xFF;
    fs::write(path, &bytes).unwrap();
    bytes
}

/// Every table as text lines, sorted (the process registry and its heartbeat are left out).
pub fn dump(dir: &Path) -> Vec<String> {
    let conn = Connection::open(dir.join("chess.db")).unwrap();
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' \
             AND name NOT IN ('process_heartbeat')",
        )
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut out = Vec::new();
    for table in tables {
        let mut st = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let columns = st.column_count();
        let rows: Vec<String> = st
            .query_map([], |r| {
                let mut cells = Vec::new();
                for i in 0..columns {
                    let v: rusqlite::types::Value = r.get(i)?;
                    cells.push(format!("{v:?}"));
                }
                Ok(cells.join("|"))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        out.extend(rows.into_iter().map(|r| format!("{table}: {r}")));
    }
    out.sort();
    out
}

/// Every file under `journal/` as `(relative path, bytes, link count)`, sorted.
pub fn journal_tree(dir: &Path) -> Vec<(String, Vec<u8>, u64)> {
    use std::os::unix::fs::MetadataExt;
    fn walk(root: &Path, at: &Path, out: &mut Vec<(String, Vec<u8>, u64)>) {
        let Ok(rd) = fs::read_dir(at) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((
                    rel,
                    fs::read(&path).unwrap(),
                    fs::metadata(&path).unwrap().nlink(),
                ));
            }
        }
    }
    let root = jdir(dir);
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out.sort();
    out
}

pub fn game_id(n: u128) -> GameId {
    gid(n)
}

pub fn reopen(dir: &Path, s: Store) -> chess_store::OpenedStore {
    drop(s);
    open_in(dir)
}

pub fn options_for(dir: &Path) -> StoreOptions {
    crate::common::options(dir)
}
