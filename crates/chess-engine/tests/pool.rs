//! CT-EN-030..034 (pool) and CT-EN-040..042 (analysis arbiter), over the scripted engine.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]
mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use chess_engine::{
    AnalysisPriority, ComputerKey, Engine, EngineError, EnginePool, GameKey, Health, Spawner,
};
use chess_rules::Side;
use common::{GoMode, Scripted, ScriptedSpawner, fast_cfg, located, personality, start_req};

fn pool(sp: &Arc<ScriptedSpawner>) -> EnginePool {
    EnginePool::new(
        fast_cfg(),
        located(chess_engine::BinarySource::Override),
        Arc::clone(sp) as Arc<dyn Spawner>,
    )
    .unwrap()
}

fn key(game: u128, side: Side) -> ComputerKey {
    ComputerKey {
        game: GameKey(game),
        side,
    }
}

#[tokio::test]
async fn ct_en_030_nothing_is_spawned_by_pool_creation_or_handle_creation() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let h = p.computer_side(key(1, Side::White), personality()).unwrap();
    let _a = p.analysis(AnalysisPriority::Interactive).unwrap();
    assert_eq!(sp.shared.spawns(), 0);
    assert_eq!(h.health(), Health::NotStarted);
    h.search(start_req(20)).await.unwrap();
    assert_eq!(sp.shared.spawns(), 1);
}

#[tokio::test]
async fn ct_en_031_warm_starts_and_handshakes_ahead_of_the_first_search() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let h = p.computer_side(key(1, Side::White), personality()).unwrap();
    h.warm().await.unwrap();
    assert_eq!(sp.shared.spawns(), 1);
    assert_eq!(h.health(), Health::Ready);
    assert_eq!(sp.shared.count_prefix("go"), 0);
    h.search(start_req(20)).await.unwrap();
    assert_eq!(sp.shared.spawns(), 1, "the search reused the warm process");
    let report = h.handshake_report().await.unwrap();
    assert!(report.info.id_name.unwrap().contains("Stockfish 19"));
}

#[tokio::test]
async fn ct_en_032_self_play_gets_two_processes_and_a_key_is_shared() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let w = p.computer_side(key(1, Side::White), personality()).unwrap();
    let b = p.computer_side(key(1, Side::Black), personality()).unwrap();
    w.search(start_req(20)).await.unwrap();
    b.search(start_req(20)).await.unwrap();
    assert_eq!(sp.shared.spawns(), 2);
    let again = p.computer_side(key(1, Side::White), personality()).unwrap();
    again.search(start_req(20)).await.unwrap();
    assert_eq!(sp.shared.spawns(), 2, "same key, same instance");
    assert_eq!(p.computer_count(), 2);
}

#[tokio::test]
async fn ct_en_033_release_game_shuts_down_only_that_games_instances() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    for (g, s) in [(1, Side::White), (1, Side::Black), (2, Side::White)] {
        p.computer_side(key(g, s), personality())
            .unwrap()
            .warm()
            .await
            .unwrap();
    }
    assert_eq!(sp.shared.alive(), 3);
    p.release_game(GameKey(1)).await;
    assert_eq!(sp.shared.alive(), 1);
    assert_eq!(p.computer_count(), 1);
    assert_eq!(sp.shared.count_prefix("quit"), 2);
}

#[tokio::test]
async fn ct_en_034_shutdown_is_bounded_by_the_grace_period_and_kills_a_stubborn_engine() {
    let sp = ScriptedSpawner::new(Scripted {
        ignore_quit: true,
        ..Scripted::default()
    });
    let p = pool(&sp);
    p.computer_side(key(1, Side::White), personality())
        .unwrap()
        .warm()
        .await
        .unwrap();
    p.analysis(AnalysisPriority::Interactive)
        .unwrap()
        .warm()
        .await
        .unwrap();
    assert_eq!(sp.shared.alive(), 2);
    let grace = fast_cfg().shutdown_grace;
    let t = Instant::now();
    p.shutdown().await;
    assert!(
        t.elapsed() < grace * 2 + Duration::from_millis(1500), // EF-09
        "{:?}",
        t.elapsed()
    );
    assert_eq!(sp.shared.alive(), 0);
    assert_eq!(p.computer_count(), 0);
}

#[tokio::test]
async fn ct_en_035_a_failed_instance_does_not_affect_another() {
    let crash = Scripted {
        go: GoMode::Crash,
        ..Scripted::default()
    };
    let sp = ScriptedSpawner::with_queue(Scripted::default(), vec![crash]);
    let p = pool(&sp);
    let bad = p.computer_side(key(1, Side::White), personality()).unwrap();
    assert!(matches!(
        bad.search(start_req(100)).await,
        Err(EngineError::ProcessExited { .. })
    ));
    let good = p.computer_side(key(2, Side::White), personality()).unwrap();
    assert!(good.search(start_req(20)).await.is_ok());
    assert_eq!(bad.health(), Health::Dead);
    assert_eq!(good.health(), Health::Ready);
}

#[tokio::test]
async fn ct_en_036_hash_accounting_counts_configured_instances() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let cfg = fast_cfg();
    assert_eq!(p.hash_in_use_mb(), 0);
    p.computer_side(key(1, Side::White), personality()).unwrap();
    p.computer_side(key(1, Side::Black), personality()).unwrap();
    p.analysis(AnalysisPriority::Background).unwrap();
    p.analysis(AnalysisPriority::Interactive).unwrap();
    assert_eq!(
        p.hash_in_use_mb(),
        2 * cfg.computer_hash_mb + cfg.analysis_hash_mb
    );
}

#[tokio::test]
async fn ct_en_040_an_interactive_request_preempts_background_work() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let bg = p.analysis(AnalysisPriority::Background).unwrap();
    let ia = p.analysis(AnalysisPriority::Interactive).unwrap();
    bg.warm().await.unwrap();
    let bg2 = bg.clone();
    let background = tokio::spawn(async move { bg2.search(start_req(5000)).await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    let t = Instant::now();
    let r = ia.search(start_req(30)).await.unwrap();
    assert!(!r.stopped_early);
    assert!(
        t.elapsed() < Duration::from_millis(2500), // far below the 5 s background search (EF-09)
        "{:?}",
        t.elapsed()
    );
    match background.await.unwrap() {
        Err(EngineError::Preempted { partial }) => assert!(partial.unwrap().stopped_early),
        other => panic!("{other:?}"),
    }
    assert_eq!(sp.shared.spawns(), 1, "one shared instance for both lanes");
}

#[tokio::test]
async fn ct_en_041_background_work_resumes_after_interactive_work_ends() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let bg = p.analysis(AnalysisPriority::Background).unwrap();
    let ia = p.analysis(AnalysisPriority::Interactive).unwrap();
    ia.warm().await.unwrap();
    let ia2 = ia.clone();
    let interactive = tokio::spawn(async move { ia2.search(start_req(200)).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    // Queued while an interactive request runs: must wait, then run, not starve.
    let queued = tokio::time::timeout(Duration::from_secs(3), bg.search(start_req(30))).await;
    assert!(interactive.await.unwrap().is_ok());
    assert!(queued.expect("background was starved").is_ok());
}

#[tokio::test]
async fn ct_en_042_a_newer_live_analysis_replaces_an_older_one_by_dropping_it() {
    let sp = ScriptedSpawner::new(Scripted {
        bests: vec!["a2a3", "e2e4"],
        ..Scripted::default()
    });
    let p = pool(&sp);
    let ia = p.analysis(AnalysisPriority::Interactive).unwrap();
    let old = tokio::time::timeout(Duration::from_millis(80), ia.search(start_req(5000))).await;
    assert!(old.is_err(), "the older request was dropped");
    let newer = ia.search(start_req(30)).await.unwrap();
    assert_eq!(newer.best.as_str(), "e2e4");
    assert_eq!(sp.shared.spawns(), 1);
}

#[tokio::test]
async fn ct_en_043_stop_on_a_lane_stops_only_its_own_search() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let p = pool(&sp);
    let bg = p.analysis(AnalysisPriority::Background).unwrap();
    let ia = p.analysis(AnalysisPriority::Interactive).unwrap();
    let bg2 = bg.clone();
    let running = tokio::spawn(async move { bg2.search(start_req(400)).await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    ia.stop(); // not the running lane: no effect
    let r = running.await.unwrap().unwrap();
    assert!(!r.stopped_early);
    assert_eq!(sp.shared.count_prefix("stop"), 0);
}
