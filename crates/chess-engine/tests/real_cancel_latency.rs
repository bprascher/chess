//! EI-EN-005: cancel latency against the real Stockfish 19 (ENG-5: 200 ms, EST TBD-12).
//! In its own test binary with one test, so no other Stockfish runs beside the measurement.
//! `stop()` is called while the engine is searching; the latency is the time from that call
//! until `search` returns the best move so far. The distribution is printed and appended to
//! `<target tmp>/stockfish-integration-status.txt`. Skips visibly when the binary is absent
//! (fails when `CHESS_REQUIRE_STOCKFISH=1`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chess_engine::{
    BinarySource, Engine, EngineConfig, EnginePersonality, InstanceProfile, LocatedBinary,
    ProcessSpawner, SearchLimits, SearchPosition, SearchRequest, Spawner, UciEngine,
};
use chess_rules::Fen;

const RUNS: usize = 100;
const LIMIT: Duration = Duration::from_millis(200);

fn note(line: &str) {
    eprintln!("{line}");
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stockfish-integration-status.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(format!("{line}\n").as_bytes());
    }
}

fn percentile(sorted: &[Duration], pct: usize) -> Duration {
    let idx = (sorted.len().saturating_sub(1) * pct).div_ceil(100);
    sorted.get(idx).copied().unwrap_or_default()
}

#[tokio::test]
async fn ei_en_005_cancel_latency_is_within_200_ms() {
    let path = match std::env::var_os("CHESS_STOCKFISH").filter(|v| !v.is_empty()) {
        Some(v) => PathBuf::from(v),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/stockfish/build/extract/stockfish/stockfish-macos-universal"),
    };
    if !path.is_file() {
        let msg = "EI-EN-005 SKIPPED: the Stockfish binary is absent; not a pass";
        if std::env::var_os("CHESS_REQUIRE_STOCKFISH").is_some_and(|v| v == "1") {
            panic!("{msg}, and CHESS_REQUIRE_STOCKFISH=1");
        }
        note(msg);
        return;
    }
    let personality = EnginePersonality {
        skill_level: 20,
        depth_limit: 0,
        movetime_ms: 1000,
        uci_elo: None,
        error_rate: 0.0,
        error_multipv: 3,
    };
    let e = UciEngine::new(
        EngineConfig::default(),
        LocatedBinary {
            path,
            source: BinarySource::Override,
        },
        Arc::new(ProcessSpawner::new()) as Arc<dyn Spawner>,
        InstanceProfile::Computer(personality),
    )
    .unwrap();
    e.warm().await.unwrap();
    let fen =
        Fen::parse("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    let mut samples = Vec::with_capacity(RUNS);
    for i in 0..RUNS {
        let req = SearchRequest::new(
            SearchPosition::Fen(fen.clone()),
            SearchLimits::new(Duration::from_secs(10), None, None).unwrap(),
        );
        let e2 = e.clone();
        let search = tokio::spawn(async move { e2.search(req).await });
        // Vary when the cancel arrives: 150 to 350 ms into the search.
        tokio::time::sleep(Duration::from_millis(
            150 + u64::try_from(i % 5).unwrap() * 50,
        ))
        .await;
        let t = Instant::now();
        e.stop();
        let r = search
            .await
            .unwrap_or_else(|e| panic!("run {i}: task failed: {e:?}"))
            .unwrap_or_else(|e| {
                panic!(
                    "run {i}: search failed: {e:?} (samples so far {})",
                    samples.len()
                )
            });
        samples.push(t.elapsed());
        assert!(r.stopped_early);
        assert!(!r.best.as_str().is_empty());
    }
    let mut sorted = samples.clone();
    sorted.sort();
    let total: Duration = sorted.iter().sum();
    let mean = total / u32::try_from(RUNS).unwrap();
    let line = format!(
        "EI-EN-005 RAN cancel latency stop()->result, n={RUNS}: min {:?} p50 {:?} p90 {:?} p99 {:?} max {:?} mean {:?} (limit {:?}); cpus {}",
        sorted[0],
        percentile(&sorted, 50),
        percentile(&sorted, 90),
        percentile(&sorted, 99),
        sorted[sorted.len() - 1],
        mean,
        LIMIT,
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0)
    );
    note(&line);
    assert!(*sorted.last().unwrap() <= LIMIT, "{line}");
}
