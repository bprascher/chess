//! CT-EN-001..006, 010..017, 021, 060 (part), 070..074: the real `UciEngine` driver against a
//! scripted in-process engine. Timers are real and the configuration shrinks them (FIND-E2).
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
    BinarySource, Engine, EngineConfig, EngineError, ErrorClass, Health, InstanceProfile,
    PINNED_UCI_ELO_RANGE, Score, Spawner, UciEngine,
};
use common::{
    GoMode, Scripted, ScriptedSpawner, computer, engine_with, fast_cfg, located, personality,
    start_req,
};

#[tokio::test]
async fn ct_en_001_handshake_order_and_options() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    assert_eq!(
        e.health(),
        Health::NotStarted,
        "nothing spawns at construction"
    );
    assert_eq!(sp.shared.spawns(), 0);
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4");
    let log = sp.shared.lines();
    let uci = sp.shared.position_of("uci", 0).unwrap();
    let skill = sp
        .shared
        .position_of("setoption name Skill Level value 5", 0)
        .unwrap();
    let ready = sp.shared.position_of("isready", 0).unwrap();
    let pos = sp.shared.position_of("position", 0).unwrap();
    let go = sp.shared.position_of("go", 0).unwrap();
    assert!(
        uci < skill && skill < ready && ready < pos && pos < go,
        "{log:?}"
    );
    assert_eq!(log[pos], "position startpos");
    assert_eq!(log[go], "go movetime 30");
    assert_eq!(e.health(), Health::Ready);
}

#[tokio::test]
async fn ct_en_002_search_returns_best_pv_score_depth_and_ignores_noise() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4");
    assert_eq!(r.score, Score::Cp(20));
    assert_eq!(r.depth, 1);
    assert_eq!(r.pv.len(), 1);
    assert_eq!(r.lines.len(), 1);
    assert!(!r.stopped_early, "finished by itself");
}

#[tokio::test]
async fn ct_en_003_stop_reaches_the_engine_and_returns_the_best_so_far() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    let e2 = e.clone();
    let search = tokio::spawn(async move { e2.search(start_req(5000)).await });
    tokio::time::sleep(Duration::from_millis(60)).await;
    let t = Instant::now();
    e.stop();
    let r = search.await.unwrap().unwrap();
    assert!(r.stopped_early);
    assert!(
        t.elapsed() < Duration::from_millis(2500), // the stop took effect long before the 5 s search ended (EF-09: wide margin)
        "{:?}",
        t.elapsed()
    );
    assert_eq!(sp.shared.count_prefix("stop"), 1);
}

#[tokio::test]
async fn ct_en_004_drop_then_drain_and_the_next_search_is_not_given_the_stale_bestmove() {
    let sp = ScriptedSpawner::new(Scripted {
        bests: vec!["a2a3", "e2e4"],
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    // First search abandoned by dropping its future.
    let abandoned =
        tokio::time::timeout(Duration::from_millis(60), e.search(start_req(5000))).await;
    assert!(abandoned.is_err(), "the search future was dropped");
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4", "the stale a2a3 must be discarded");
    assert_eq!(sp.shared.count_prefix("stop"), 1);
    assert_eq!(
        sp.shared.spawns(),
        1,
        "the instance was reused, not restarted"
    );
}

#[tokio::test]
async fn ct_en_005_no_go_is_sent_before_the_abandoned_bestmove_arrives() {
    let sp = ScriptedSpawner::new(Scripted {
        stop_delay: Duration::from_millis(120),
        bests: vec!["a2a3", "e2e4"],
        ..Scripted::default()
    });
    let mut cfg = fast_cfg();
    cfg.drain_timeout = Duration::from_millis(600);
    cfg.soft_stop_grace = Duration::from_millis(100);
    cfg.hard_kill_grace = Duration::from_millis(900);
    let e = engine_with(&sp, cfg);
    let _ = tokio::time::timeout(Duration::from_millis(60), e.search(start_req(5000))).await;
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4");
    let sent = sp.shared.position_of("<bestmove sent>", 0).unwrap();
    let second_go = sp.shared.position_of("go", 1).unwrap();
    assert!(sent < second_go, "{:?}", sp.shared.lines());
}

#[tokio::test]
async fn ct_en_006_stop_with_no_search_does_nothing_and_does_not_leak_into_the_next() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    e.stop();
    e.stop();
    let r = e.search(start_req(40)).await.unwrap();
    assert!(!r.stopped_early);
    e.stop();
    let r2 = e.search(start_req(40)).await.unwrap();
    assert!(!r2.stopped_early);
    assert_eq!(sp.shared.count_prefix("stop"), 0);
}

#[tokio::test]
async fn ct_en_010_spawn_failure_is_reported_and_not_retried() {
    let sp = ScriptedSpawner::new(Scripted::default());
    sp.fail_spawn
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let e = engine_with(&sp, fast_cfg());
    let err = e.search(start_req(30)).await.unwrap_err();
    assert_eq!(err, EngineError::SpawnFailed);
    assert_eq!(err.class(), ErrorClass::Unavailable("spawn_failed"));
    assert_eq!(e.health(), Health::Dead);
    // The next request tries again (RetryEngine).
    sp.fail_spawn
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert!(e.search(start_req(30)).await.is_ok());
}

#[tokio::test]
async fn ct_en_011_handshake_timeout_restarts_once_then_reports() {
    let silent = Scripted {
        send_uciok: false,
        ..Scripted::default()
    };
    let sp = ScriptedSpawner::new(silent);
    let e = engine_with(&sp, fast_cfg());
    let err = e.search(start_req(30)).await.unwrap_err();
    assert_eq!(err, EngineError::HandshakeTimeout);
    assert_eq!(err.class(), ErrorClass::Unavailable("handshake_timeout"));
    assert_eq!(sp.shared.spawns(), 2, "one automatic restart, no more");
    assert_eq!(
        sp.shared.alive(),
        0,
        "both silent processes were killed and reaped"
    );
    assert_eq!(e.health(), Health::Dead);
}

#[tokio::test]
async fn ct_en_011b_a_slow_first_start_is_hidden_by_the_restart() {
    let silent = Scripted {
        send_readyok: false,
        ..Scripted::default()
    };
    let sp = ScriptedSpawner::with_queue(Scripted::default(), vec![silent]);
    let e = engine_with(&sp, fast_cfg());
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4");
    assert_eq!(sp.shared.spawns(), 2);
}

#[tokio::test]
async fn ct_en_012_crash_mid_search_is_reported_and_021_the_next_request_restarts() {
    let crash = Scripted {
        go: GoMode::Crash,
        ..Scripted::default()
    };
    let sp = ScriptedSpawner::with_queue(Scripted::default(), vec![crash]);
    let e = engine_with(&sp, fast_cfg());
    let err = e.search(start_req(500)).await.unwrap_err();
    assert!(matches!(err, EngineError::ProcessExited { .. }), "{err:?}");
    assert_eq!(err.class(), ErrorClass::Unavailable("crashed"));
    assert_eq!(e.health(), Health::Dead);
    assert_eq!(
        sp.shared.spawns(),
        1,
        "a failed in-flight search is not silently retried"
    );
    // CT-EN-021: RetryEngine is exactly the next request.
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4");
    assert_eq!(sp.shared.spawns(), 2);
    assert_eq!(e.health(), Health::Ready);
}

#[tokio::test]
async fn ct_en_013_hang_gets_a_soft_stop_then_a_hard_kill_at_the_limit_plus_grace() {
    let hang = Scripted {
        go: GoMode::Hang,
        ..Scripted::default()
    };
    let sp = ScriptedSpawner::new(hang);
    let cfg = fast_cfg();
    let hard = cfg.hard_kill_grace;
    let e = engine_with(&sp, cfg);
    let t = Instant::now();
    let err = e.search(start_req(100)).await.unwrap_err();
    let took = t.elapsed();
    assert_eq!(err, EngineError::SearchTimeout);
    assert_eq!(err.class(), ErrorClass::Unavailable("timeout"));
    assert!(
        took >= Duration::from_millis(100) + hard - Duration::from_millis(20),
        "{took:?}"
    );
    assert!(
        took < Duration::from_millis(100) + hard + Duration::from_millis(1500), // EF-09
        "{took:?}"
    );
    assert_eq!(
        sp.shared.count_prefix("stop"),
        1,
        "the soft stop was written once"
    );
    assert_eq!(sp.shared.alive(), 0, "killed and reaped");
    assert_eq!(e.health(), Health::Dead);
}

#[tokio::test]
async fn ct_en_014_garbage_bestmove_is_bad_output() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::Garbage,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    let err = e.search(start_req(100)).await.unwrap_err();
    assert_eq!(err, EngineError::BadOutput);
    assert_eq!(err.class(), ErrorClass::Unavailable("bad_output"));
    assert_eq!(sp.shared.alive(), 0);
}

#[tokio::test]
async fn ct_en_015_a_line_over_the_cap_is_bad_output() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::LongLine,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    let err = e.search(start_req(100)).await.unwrap_err();
    assert_eq!(err, EngineError::BadOutput);
    assert_eq!(e.health(), Health::Dead);
}

#[tokio::test]
async fn ct_en_016_bestmove_none_is_bad_output() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::NoMove,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    assert_eq!(
        e.search(start_req(100)).await.unwrap_err(),
        EngineError::BadOutput
    );
}

#[tokio::test]
async fn ct_en_017_wrong_id_name_is_rejected_for_a_bundled_path_only() {
    let wrong = Scripted {
        id_name: "Other Engine 1",
        ..Scripted::default()
    };
    // Bundled: checked.
    let sp = ScriptedSpawner::new(wrong.clone());
    let e = UciEngine::new(
        fast_cfg(),
        located(BinarySource::Bundled),
        Arc::clone(&sp) as Arc<dyn Spawner>,
        computer(),
    )
    .unwrap();
    let err = e.search(start_req(30)).await.unwrap_err();
    assert!(matches!(err, EngineError::WrongVersion { .. }), "{err:?}");
    assert_eq!(err.class(), ErrorClass::Unavailable("wrong_version"));
    assert!(
        !err.to_string().contains("Other"),
        "Display must not show engine output"
    );
    assert_eq!(sp.shared.spawns(), 1, "a packaging defect is not retried");
    // Override: skipped so that the fake can run.
    let sp2 = ScriptedSpawner::new(wrong);
    let e2 = engine_with(&sp2, fast_cfg());
    assert!(e2.search(start_req(30)).await.is_ok());
}

#[tokio::test]
async fn ct_en_070_a_second_concurrent_search_gets_busy() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    let e1 = e.clone();
    let first = tokio::spawn(async move { e1.search(start_req(300)).await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    let err = e.search(start_req(30)).await.unwrap_err();
    assert_eq!(err, EngineError::Busy);
    assert_eq!(err.class(), ErrorClass::Internal);
    assert!(
        first.await.unwrap().is_ok(),
        "the first search is undisturbed"
    );
}

#[tokio::test]
async fn ct_en_071_updates_are_throttled_and_a_full_channel_never_blocks() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::Chatty,
        ..Scripted::default()
    });
    let mut cfg = fast_cfg();
    cfg.update_interval = Duration::from_millis(100);
    let e = engine_with(&sp, cfg);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let mut req = start_req(100);
    req.updates = Some(tx);
    let r = e.search(req).await.unwrap();
    assert_eq!(r.depth, 60);
    let mut got = 0;
    while rx.try_recv().is_ok() {
        got += 1;
    }
    assert_eq!(
        got, 1,
        "60 lines in a burst: one update per interval and a channel of one"
    );
}

#[tokio::test]
async fn ct_en_072_multipv_is_sent_once_and_lines_are_those_of_the_last_complete_depth() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::MultiPvTwoDepths,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    let mut req = start_req(20);
    req.multipv = 3;
    let r = e.search(req.clone()).await.unwrap();
    assert_eq!(
        r.lines.len(),
        3,
        "depth 2 only has line 1: depth 1 is the last complete depth"
    );
    assert_eq!(
        r.lines.iter().map(|l| l.multipv).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(r.lines.iter().all(|l| l.depth == 1));
    e.search(req).await.unwrap();
    assert_eq!(
        sp.shared.count_prefix("setoption name MultiPV value 3"),
        1,
        "not re-sent"
    );
    let mut one = start_req(20);
    one.multipv = 1;
    e.search(one).await.unwrap();
    assert_eq!(sp.shared.count_prefix("setoption name MultiPV value 1"), 1);
}

#[tokio::test]
async fn ct_en_073_new_game_sends_ucinewgame_then_an_isready_barrier_before_the_position() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    let mut req = start_req(20);
    req.new_game = true;
    e.search(req).await.unwrap();
    let new_game = sp.shared.position_of("ucinewgame", 0).unwrap();
    let ready = sp.shared.position_of("isready", 1).unwrap();
    let pos = sp.shared.position_of("position", 0).unwrap();
    assert!(new_game < ready && ready < pos);
}

#[tokio::test]
async fn ct_en_074_only_the_difference_from_the_defaults_is_sent() {
    // Threads 1, Hash 16 and Skill Level 20 equal the advertised defaults: not sent.
    let mut p = personality();
    p.skill_level = 20;
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = UciEngine::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
        InstanceProfile::Computer(p),
    )
    .unwrap();
    e.search(start_req(20)).await.unwrap();
    assert_eq!(
        sp.shared.count_prefix("setoption"),
        0,
        "{:?}",
        sp.shared.lines()
    );
}

#[tokio::test]
async fn ct_en_075_elo_is_clamped_for_the_search_and_reported() {
    let mut p = personality();
    p.uci_elo = Some(PINNED_UCI_ELO_RANGE.max + 500);
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = UciEngine::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
        InstanceProfile::Computer(p.clone()),
    )
    .unwrap();
    e.warm().await.unwrap();
    let report = e.handshake_report().await.unwrap();
    assert!(report.plan.clamped);
    assert!(!report.plan.differs_from_pinned);
    let want = format!("setoption name UCI_Elo value {}", PINNED_UCI_ELO_RANGE.max);
    assert_eq!(sp.shared.count_prefix(&want), 1, "{:?}", sp.shared.lines());
    assert_eq!(
        sp.shared
            .count_prefix("setoption name UCI_LimitStrength value true"),
        1
    );
    assert_eq!(
        p.uci_elo,
        Some(PINNED_UCI_ELO_RANGE.max + 500),
        "stored value untouched"
    );
}

#[tokio::test]
async fn ct_en_076_invalid_personality_is_internal_and_not_retried() {
    let mut p = personality();
    p.uci_elo = Some(PINNED_UCI_ELO_RANGE.min);
    let sp = ScriptedSpawner::new(Scripted {
        uci_elo_range: None,
        ..Scripted::default()
    });
    let e = UciEngine::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
        InstanceProfile::Computer(p),
    )
    .unwrap();
    let err = e.search(start_req(20)).await.unwrap_err();
    assert!(
        matches!(err, EngineError::InvalidPersonality { .. }),
        "{err:?}"
    );
    assert_eq!(err.class(), ErrorClass::Internal);
    assert_eq!(sp.shared.alive(), 0);
}

#[tokio::test]
async fn ct_en_077_request_validation_happens_before_the_process_is_touched() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(
        &sp,
        EngineConfig {
            multipv_cap: 4,
            ..fast_cfg()
        },
    );
    let mut req = start_req(20);
    req.multipv = 0;
    assert!(matches!(
        e.search(req.clone()).await,
        Err(EngineError::InvalidRequest { .. })
    ));
    req.multipv = 5;
    assert!(matches!(
        e.search(req).await,
        Err(EngineError::InvalidRequest { .. })
    ));
    assert_eq!(sp.shared.spawns(), 0);
}

#[tokio::test]
async fn ct_en_078_shutdown_sends_quit_and_closes_the_instance() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    e.warm().await.unwrap();
    e.shutdown().await;
    assert_eq!(sp.shared.count_prefix("quit"), 1);
    assert_eq!(sp.shared.alive(), 0);
    assert_eq!(
        e.search(start_req(20)).await.unwrap_err(),
        EngineError::Closed
    );
}
