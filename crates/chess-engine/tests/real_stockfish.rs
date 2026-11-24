//! EI-EN-001..004, 006..008: the real Stockfish 19 binary (engine integration).
//!
//! Skip policy. If the binary is absent each test prints a visible `SKIPPED` line, appends
//! `<id> SKIPPED: ...` to `<target tmp>/stockfish-integration-status.txt`, and returns. A run
//! that ran appends `<id> RAN`. CI that must not skip sets `CHESS_REQUIRE_STOCKFISH=1`, and
//! the test then fails instead. The binary is the untracked, git-ignored build output of
//! `third_party/stockfish/fetch.sh`; `CHESS_STOCKFISH` (absolute path) overrides the default.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]
mod common;

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chess_engine::{
    BinarySource, Engine, EngineConfig, EnginePersonality, Health, InstanceProfile, LocatedBinary,
    PINNED_UCI_ELO_RANGE, ProcessSpawner, Score, SearchLimits, SearchPosition, SearchRequest,
    Spawner, UciEngine, select_move,
};
use chess_rules::{Fen, Move, Position};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

fn binary() -> Option<PathBuf> {
    let p = match std::env::var_os("CHESS_STOCKFISH").filter(|v| !v.is_empty()) {
        Some(v) => PathBuf::from(v),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/stockfish/build/extract/stockfish/stockfish-macos-universal"),
    };
    p.is_file().then_some(p)
}

fn record(id: &str, status: &str) {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stockfish-integration-status.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        // One write per line: parallel tests append to the same file.
        let _ = f.write_all(format!("{id} {status}\n").as_bytes());
    }
}

/// `Some(path)` to run, `None` to skip (visibly), panic when the run is required.
fn require(id: &str) -> Option<PathBuf> {
    match binary() {
        Some(p) => {
            record(id, "RAN");
            Some(p)
        }
        None => {
            let msg = "SKIPPED: the Stockfish binary is absent (third_party/stockfish/build/...); not a pass";
            if std::env::var_os("CHESS_REQUIRE_STOCKFISH").is_some_and(|v| v == "1") {
                panic!("{id} {msg}, and CHESS_REQUIRE_STOCKFISH=1");
            }
            eprintln!("{id} {msg}");
            record(id, msg);
            None
        }
    }
}

fn cfg() -> EngineConfig {
    EngineConfig::default()
}

fn engine_for(path: PathBuf, profile: InstanceProfile) -> UciEngine {
    UciEngine::new(
        cfg(),
        LocatedBinary {
            path,
            source: BinarySource::Override,
        },
        Arc::new(ProcessSpawner::new()) as Arc<dyn Spawner>,
        profile,
    )
    .unwrap()
}

fn full_strength() -> EnginePersonality {
    EnginePersonality {
        skill_level: 20,
        depth_limit: 0,
        movetime_ms: 1000,
        uci_elo: None,
        error_rate: 0.0,
        error_multipv: 3,
    }
}

fn req(fen: &str, movetime_ms: u64, depth: Option<u32>) -> SearchRequest {
    SearchRequest::new(
        SearchPosition::Fen(Fen::parse(fen).unwrap()),
        SearchLimits::new(Duration::from_millis(movetime_ms), depth, None).unwrap(),
    )
}

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[tokio::test]
async fn ei_en_001_handshake_reports_stockfish_19_and_it_answers_legal_moves() {
    let Some(path) = require("EI-EN-001") else {
        return;
    };
    let e = engine_for(path, InstanceProfile::Computer(full_strength()));
    e.warm().await.unwrap();
    let report = e.handshake_report().await.unwrap();
    let name = report.info.id_name.clone().unwrap();
    assert!(name.contains(&cfg().pinned_id_text), "id name: {name}");
    assert_eq!(e.health(), Health::Ready);
    for fen in [
        START,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2",
    ] {
        let r = e.search(req(fen, 150, Some(8))).await.unwrap();
        let pos = Position::from_fen(&Fen::parse(fen).unwrap());
        let mv = Move::from_uci(r.best.as_str()).unwrap();
        assert!(pos.is_legal(mv), "{fen}: {} is not legal", r.best);
        assert!(!r.pv.is_empty() && r.depth >= 1);
    }
}

#[tokio::test]
async fn ei_en_002_the_pinned_range_equals_the_live_uci_elo_option() {
    let Some(path) = require("EI-EN-002") else {
        return;
    };
    let e = engine_for(path, InstanceProfile::Analysis);
    e.warm().await.unwrap();
    let report = e.handshake_report().await.unwrap();
    assert_eq!(
        report.info.uci_elo_range,
        Some((PINNED_UCI_ELO_RANGE.min, PINNED_UCI_ELO_RANGE.max)),
        "engine pin changed without changing PINNED_UCI_ELO_RANGE (FF-7)"
    );
    assert!(!report.plan.differs_from_pinned);
}

/// Independent oracle (chess-rules, not the engine): can the side to move force mate within
/// `plies` plies (1 or 3)?
fn forced_mate(p: &Position, plies: u32) -> bool {
    p.legal_moves().into_iter().any(|m| {
        let Ok(q) = p.play(m) else { return false };
        if q.is_checkmate() {
            return true;
        }
        if plies < 3 || q.is_stalemate() {
            return false;
        }
        let replies = q.legal_moves();
        !replies.is_empty()
            && replies
                .into_iter()
                .all(|r| q.play(r).is_ok_and(|n| forced_mate(&n, plies - 2)))
    })
}

#[tokio::test]
async fn ei_en_003_mate_in_one_and_two_are_found_with_one_thread_and_a_fixed_depth() {
    let Some(path) = require("EI-EN-003") else {
        return;
    };
    let e = engine_for(path, InstanceProfile::Computer(full_strength()));
    // Back-rank mate in one: Ra8#.
    let r = e
        .search(req("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1", 5000, Some(10)))
        .await
        .unwrap();
    assert_eq!(r.best.as_str(), "a1a8");
    assert_eq!(r.score, Score::Mate(1));
    // Mate in two. The oracle proves that each position is a mate in two and not in one, so
    // the expectation does not come from the engine's own output.
    for fen in [
        "6k1/8/5K2/8/8/8/8/R7 w - - 0 1",
        "7k/R7/8/6K1/8/8/8/8 w - - 0 1",
    ] {
        let pos = Position::from_fen(&Fen::parse(fen).unwrap());
        assert!(
            !forced_mate(&pos, 1) && forced_mate(&pos, 3),
            "oracle: {fen}"
        );
        let r = e.search(req(fen, 5000, Some(12))).await.unwrap();
        assert_eq!(r.score, Score::Mate(2), "{fen}: pv {:?}", r.pv);
        let first = Move::from_uci(r.best.as_str()).unwrap();
        assert!(pos.is_legal(first));
    }
}

#[tokio::test]
async fn ei_en_004_think_time_stays_within_the_requested_movetime() {
    let Some(path) = require("EI-EN-004") else {
        return;
    };
    let e = engine_for(path, InstanceProfile::Computer(full_strength()));
    e.warm().await.unwrap();
    let mut worst = Duration::ZERO;
    for _ in 0..8 {
        let t = Instant::now();
        let r = e.search(req(START, 200, None)).await.unwrap();
        let wall = t.elapsed();
        worst = worst.max(wall);
        assert!(!r.stopped_early);
    }
    eprintln!("EI-EN-004 movetime 200 ms, n=8, worst wall {worst:?}");
    // Margin for process scheduling; the SDD's own limit is movetime plus 500 ms (soft stop).
    assert!(worst < Duration::from_millis(200 + 500), "{worst:?}");
}

#[tokio::test]
async fn ei_en_006_stockfish_exits_by_itself_when_stdin_reaches_end_of_file() {
    let Some(path) = require("EI-EN-006") else {
        return;
    };
    let spawned = ProcessSpawner::new().spawn(&path).unwrap();
    let mut control = spawned.control;
    let (mut stdin, stdout) = (spawned.stdin, spawned.stdout);
    stdin.write_all(b"uci\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut lines = BufReader::new(stdout).lines();
    loop {
        let l = tokio::time::timeout(Duration::from_secs(5), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if l == "uciok" {
            break;
        }
    }
    drop(stdin);
    let info = tokio::time::timeout(Duration::from_secs(3), control.wait())
        .await
        .expect("Stockfish must exit when stdin closes (this is what protects against an orphaned child)");
    assert_eq!(info.code, Some(0));
}

#[tokio::test]
async fn ei_en_007_a_killed_stockfish_is_a_crash_and_the_next_request_restarts_it() {
    let Some(path) = require("EI-EN-007") else {
        return;
    };
    let sp = common::RecordingSpawner::new(ProcessSpawner::new());
    let e = UciEngine::new(
        cfg(),
        LocatedBinary {
            path,
            source: BinarySource::Override,
        },
        Arc::clone(&sp) as Arc<dyn Spawner>,
        InstanceProfile::Computer(full_strength()),
    )
    .unwrap();
    let e2 = e.clone();
    let s = tokio::spawn(async move { e2.search(req(START, 10_000, None)).await });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let pid = *sp.pids().first().unwrap();
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .unwrap();
    let err = s.await.unwrap().unwrap_err();
    assert!(
        matches!(err, chess_engine::EngineError::ProcessExited { .. }),
        "{err:?}"
    );
    assert!(!common::pid_alive(pid));
    let r = e.search(req(START, 100, Some(6))).await.unwrap();
    assert!(!r.best.as_str().is_empty());
    assert_eq!(sp.pids().len(), 2, "restarted by the next request");
}

#[tokio::test]
async fn ei_en_008_the_elo_limited_personality_and_error_pick_work_end_to_end() {
    let Some(path) = require("EI-EN-008") else {
        return;
    };
    // An out-of-range Elo is clamped by the plan; Stockfish itself would have accepted it silently.
    let p = EnginePersonality {
        skill_level: 20,
        depth_limit: 0,
        movetime_ms: 200,
        uci_elo: Some(PINNED_UCI_ELO_RANGE.max + 1000),
        error_rate: 1.0,
        error_multipv: 3,
    };
    let e = engine_for(path, InstanceProfile::Computer(p.clone()));
    e.warm().await.unwrap();
    assert!(e.handshake_report().await.unwrap().plan.clamped);
    let mut r = req(START, 200, Some(6));
    r.seed = 7;
    let choice = select_move(&e, r, &p).await.unwrap();
    let pos = Position::from_fen(&Fen::parse(START).unwrap());
    assert!(pos.is_legal(Move::from_uci(choice.mv.as_str()).unwrap()));
    assert_eq!(choice.kind, chess_engine::MoveKind::Error);
    assert!(choice.result.lines.len() >= 2);
    assert_ne!(
        Some(&choice.mv),
        choice.result.lines.first().and_then(|l| l.pv.first())
    );
}
