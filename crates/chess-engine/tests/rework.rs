//! Rework of chess-a0r.3.10 after the review REV-chess-engine-code (EF-01 .. EF-12, EF-14):
//! regression tests that failed on the reviewed code. Scripted engine, real timers; margins
//! are wide on purpose (EF-09).
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

use chess_engine::uci::LineSplitter;
use chess_engine::{
    AnalysisPriority, BinarySource, ComputerKey, Engine, EngineConfig, EngineError, EnginePool,
    GameKey, Health, InstanceProfile, Score, SearchLimits, SearchPosition, SearchRequest, Spawner,
    UciEngine, UciMove,
};
use chess_rules::Side;
use common::{
    GoMode, Scripted, ScriptedSpawner, engine_with, fast_cfg, located, personality, start_req,
};

/// `rt-multi-thread` is not an admitted tokio feature, so concurrency across threads is made
/// from std threads that each own a current-thread runtime.
fn mt() -> tokio::runtime::Builder {
    let mut b = tokio::runtime::Builder::new_current_thread();
    b.enable_all();
    b
}

// ---------------------------------------------------------------- EF-01

/// The reviewed code never reached the timers while output kept flowing.
#[test]
fn ef_01_a_flood_of_info_lines_is_killed_at_the_hard_limit() {
    mt().build().unwrap().block_on(async {
        let sp = ScriptedSpawner::new(Scripted {
            go: GoMode::InfoFlood,
            ..Scripted::default()
        });
        let eng = common::runtime_thread();
        sp.run_engines_on(eng.handle.clone());
        let e = engine_with(&sp, fast_cfg());
        let t = Instant::now();
        let r = tokio::time::timeout(Duration::from_secs(5), e.search(start_req(100)))
            .await
            .expect("the search must end by itself: the hard timer is not starved");
        assert_eq!(r.unwrap_err(), EngineError::SearchTimeout);
        // movetime 100 + hard grace 300, wide margin for a loaded machine.
        assert!(
            t.elapsed() < Duration::from_millis(3000),
            "{:?}",
            t.elapsed()
        );
        assert_eq!(sp.shared.alive(), 0, "killed and reaped");
        assert_eq!(e.health(), Health::Dead);
    });
}

#[test]
fn ef_01_a_flood_of_info_lines_is_soft_stopped_at_movetime_plus_grace() {
    mt().build().unwrap().block_on(async {
        let sp = ScriptedSpawner::new(Scripted {
            go: GoMode::InfoFloodStoppable,
            ..Scripted::default()
        });
        let mut cfg = fast_cfg();
        cfg.hard_kill_grace = Duration::from_millis(2500);
        let eng = common::runtime_thread();
        sp.run_engines_on(eng.handle.clone());
        let e = engine_with(&sp, cfg);
        let t = Instant::now();
        let r = tokio::time::timeout(Duration::from_secs(5), e.search(start_req(100)))
            .await
            .expect("soft stop must fire while output flows")
            .unwrap();
        assert!(
            r.stopped_early,
            "the soft stop (100 + 80 ms) reached the engine"
        );
        assert!(
            t.elapsed() < Duration::from_millis(2400),
            "{:?}",
            t.elapsed()
        );
        assert_eq!(sp.shared.count_prefix("stop"), 1);
    });
}

#[test]
fn ef_01_a_flood_of_bare_newlines_is_killed_at_the_hard_limit() {
    mt().build().unwrap().block_on(async {
        let sp = ScriptedSpawner::new(Scripted {
            go: GoMode::NewlineFlood,
            ..Scripted::default()
        });
        let eng = common::runtime_thread();
        sp.run_engines_on(eng.handle.clone());
        let e = engine_with(&sp, fast_cfg());
        let r = tokio::time::timeout(Duration::from_secs(8), e.search(start_req(100)))
            .await
            .expect("the hard timer is not starved by empty lines");
        assert_eq!(r.unwrap_err(), EngineError::SearchTimeout);
        assert_eq!(sp.shared.alive(), 0);
    });
}

/// 1 MiB of newlines in one chunk: the reviewed splitter moved the rest of the buffer for
/// every line (quadratic, seconds); the fixed one is linear (milliseconds).
#[test]
fn ef_01_line_splitter_work_per_chunk_is_linear_in_bare_newlines() {
    let mut s = LineSplitter::new(64 * 1024);
    s.push(&vec![b'\n'; 1024 * 1024]);
    let t = Instant::now();
    let mut n = 0usize;
    while let Some(line) = s.next_line() {
        assert_eq!(line.unwrap(), "");
        n += 1;
    }
    assert_eq!(n, 1024 * 1024);
    assert!(
        t.elapsed() < Duration::from_secs(1),
        "quadratic splitter: {:?}",
        t.elapsed()
    );
}

// ---------------------------------------------------------------- EF-02

fn limits_unchecked(movetime: Duration) -> SearchLimits {
    SearchLimits {
        movetime,
        depth: None,
        nodes: None,
    }
}

#[test]
fn ef_02_validate_rejects_durations_that_overflow_instant_arithmetic() {
    for d in [
        Duration::MAX,
        Duration::from_secs(u64::MAX),
        Duration::from_secs(u64::MAX / 2),
        Duration::from_secs(10_000_000_000_u64),
    ] {
        let l = limits_unchecked(d);
        assert!(
            matches!(l.validate(), Err(EngineError::InvalidRequest { .. })),
            "{d:?}"
        );
        assert!(SearchLimits::new(d, None, None).is_err(), "{d:?}");
    }
    assert!(limits_unchecked(Duration::from_secs(60)).validate().is_ok());
}

#[test]
fn ef_02_a_huge_movetime_is_refused_and_the_instance_survives() {
    mt().build().unwrap().block_on(async {
        let sp = ScriptedSpawner::new(Scripted::default());
        let e = engine_with(&sp, fast_cfg());
        for d in [Duration::MAX, Duration::from_secs(u64::MAX / 4)] {
            let mut req = start_req(30);
            req.limits = limits_unchecked(d);
            let err = e.search(req).await.unwrap_err();
            assert!(
                matches!(err, EngineError::InvalidRequest { .. }),
                "{err:?} (Closed means the driver task panicked)"
            );
        }
        assert!(e.search(start_req(30)).await.is_ok(), "still usable");
    });
}

#[test]
fn ef_02_a_huge_grace_in_the_config_is_refused() {
    for cfg in [
        EngineConfig {
            hard_kill_grace: Duration::MAX,
            ..EngineConfig::default()
        },
        EngineConfig {
            handshake_timeout: Duration::MAX,
            ..EngineConfig::default()
        },
        EngineConfig {
            shutdown_grace: Duration::MAX,
            ..EngineConfig::default()
        },
        EngineConfig {
            drain_timeout: Duration::MAX,
            ..EngineConfig::default()
        },
    ] {
        assert!(
            matches!(cfg.validate(), Err(EngineError::InvalidConfig { .. })),
            "{cfg:?}"
        );
    }
}

// ---------------------------------------------------------------- EF-05

#[tokio::test]
async fn ef_05_an_inverted_uci_elo_range_is_a_typed_error_and_the_instance_survives() {
    let sp = ScriptedSpawner::new(Scripted {
        uci_elo_range: Some((2000, 1000)),
        ..Scripted::default()
    });
    let mut p = personality();
    p.uci_elo = Some(1500);
    let e = UciEngine::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
        InstanceProfile::Computer(p),
    )
    .unwrap();
    for _ in 0..2 {
        let err = e.warm().await.unwrap_err();
        assert_eq!(
            err,
            EngineError::InvalidPersonality {
                option: "UCI_Elo".to_string()
            },
            "Closed would mean the driver task panicked"
        );
    }
}

// ---------------------------------------------------------------- EF-06

fn slow_handshake_cfg() -> EngineConfig {
    let mut cfg = fast_cfg();
    cfg.handshake_timeout = Duration::from_secs(2);
    cfg.shutdown_grace = Duration::from_millis(200);
    cfg
}

#[tokio::test]
async fn ef_06_shutdown_during_a_hung_start_is_bounded_by_the_grace() {
    let sp = ScriptedSpawner::new(Scripted {
        send_uciok: false,
        ..Scripted::default()
    });
    let e = engine_with(&sp, slow_handshake_cfg());
    let e2 = e.clone();
    let warm = tokio::spawn(async move { e2.warm().await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let t = Instant::now();
    e.shutdown().await;
    assert!(
        t.elapsed() < Duration::from_millis(1500),
        "reviewed code: about 4 s (two handshake attempts): {:?}",
        t.elapsed()
    );
    assert_eq!(sp.shared.alive(), 0);
    assert_eq!(e.health(), Health::Dead);
    assert!(warm.await.unwrap().is_err());
    assert_eq!(sp.shared.spawns(), 1, "no restart after shutdown");
}

#[tokio::test]
async fn ef_06_pool_shutdown_of_several_stuck_instances_does_not_add_up() {
    let sp = ScriptedSpawner::new(Scripted {
        send_uciok: false,
        ..Scripted::default()
    });
    let pool = EnginePool::new(
        slow_handshake_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
    )
    .unwrap();
    let mut warms = Vec::new();
    for g in 0..4u128 {
        let h = pool
            .computer_side(
                ComputerKey {
                    game: GameKey(g),
                    side: Side::White,
                },
                personality(),
            )
            .unwrap();
        warms.push(tokio::spawn(async move { h.warm().await }));
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    let t = Instant::now();
    pool.shutdown().await;
    assert!(
        t.elapsed() < Duration::from_millis(1500),
        "{:?}",
        t.elapsed()
    );
    assert_eq!(sp.shared.alive(), 0);
    for w in warms {
        assert!(w.await.unwrap().is_err());
    }
}

// ---------------------------------------------------------------- EF-07

fn deaf_engine() -> (Arc<ScriptedSpawner>, UciEngine) {
    let sp = ScriptedSpawner::new(Scripted {
        deaf_after_handshake: true,
        pipe_bytes: 1024,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    (sp, e)
}

fn long_request(plies: usize) -> SearchRequest {
    let mut req = start_req(100);
    req.position = SearchPosition::Moves {
        start: None,
        moves: vec![UciMove::parse("a2a3").unwrap(); plies],
    };
    req
}

#[test]
fn ef_07_a_write_to_an_engine_that_does_not_read_ends_in_an_error() {
    mt().build().unwrap().block_on(async {
        let (sp, e) = deaf_engine();
        let r = tokio::time::timeout(Duration::from_secs(5), e.search(long_request(1000)))
            .await
            .expect("every wait is bounded, writes included");
        assert!(r.is_err());
        assert_eq!(sp.shared.alive(), 0, "killed and reaped");
    });
}

#[test]
fn ef_07_shutdown_during_a_blocked_write_is_prompt() {
    mt().build().unwrap().block_on(async {
        let (sp, e) = deaf_engine();
        let e2 = e.clone();
        let s = tokio::spawn(async move { e2.search(long_request(1000)).await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        let t = Instant::now();
        tokio::time::timeout(Duration::from_secs(3), e.shutdown())
            .await
            .expect("shutdown must not wait for a write that cannot finish");
        assert!(
            t.elapsed() < Duration::from_millis(1500),
            "{:?}",
            t.elapsed()
        );
        assert_eq!(sp.shared.alive(), 0);
        assert!(s.await.unwrap().is_err());
    });
}

#[tokio::test]
async fn ef_07_a_position_with_too_many_moves_is_refused() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let e = engine_with(&sp, fast_cfg());
    let err = e.search(long_request(1_000_000)).await.unwrap_err();
    assert!(matches!(err, EngineError::InvalidRequest { .. }), "{err:?}");
    assert_eq!(sp.shared.spawns(), 0, "refused before any process started");
}

// ---------------------------------------------------------------- EF-08

#[tokio::test]
async fn ef_08_pv_score_and_depth_describe_the_returned_best_move() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::PvVsBest,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    let r = e.search(start_req(50)).await.unwrap();
    assert_eq!(r.best.as_str(), "d2d4");
    assert_eq!(r.pv.first().map(UciMove::as_str), Some("d2d4"));
    assert_eq!(r.score, Score::Cp(50));
    assert_eq!(r.depth, 6);
}

#[tokio::test]
async fn ef_08_when_no_line_names_the_best_move_the_result_says_so() {
    let sp = ScriptedSpawner::new(Scripted {
        go: GoMode::PvNoMatch,
        ..Scripted::default()
    });
    let e = engine_with(&sp, fast_cfg());
    let r = e.search(start_req(50)).await.unwrap();
    assert_eq!(r.best.as_str(), "d2d4");
    assert_eq!(r.pv, vec![r.best.clone()]);
    assert_eq!(
        (r.score, r.depth),
        (Score::Cp(0), 0),
        "no info about this move"
    );
}

// ---------------------------------------------------------------- EF-04

/// 2000 races of an interactive request against a background search that has just been
/// started on another thread. The reviewed arbiter lost the preemption in about 0.4 %.
/// A race is "lost" when the engine saw the background `go`, then the interactive `go`, with
/// no `stop` in between (the stop was queued before the search and discarded). The engine
/// driver has its own runtime thread; each caller has a persistent thread of its own.
#[test]
fn ef_04_interactive_always_preempts_a_background_search_2000_races() {
    use std::sync::Barrier;
    const N: u64 = 2000;
    let home = common::runtime_thread();
    let sp = ScriptedSpawner::new(Scripted::default());
    sp.run_engines_on(home.handle.clone());
    let sp_dyn = Arc::clone(&sp) as Arc<dyn Spawner>;
    let (bg, int) = block_on_other(&home.handle, async move {
        let pool = EnginePool::new(fast_cfg(), located(BinarySource::Override), sp_dyn).unwrap();
        let bg = pool.analysis(AnalysisPriority::Background).unwrap();
        let int = pool.analysis(AnalysisPriority::Interactive).unwrap();
        bg.warm().await.unwrap();
        (bg, int)
    });
    let barrier = Arc::new(Barrier::new(3));
    let bgt = {
        let (b, bar) = (bg.clone(), Arc::clone(&barrier));
        std::thread::spawn(move || {
            let rt = mt().build().unwrap();
            let mut out = Vec::new();
            for _ in 0..N {
                bar.wait();
                out.push(rt.block_on(b.search(start_req(20))));
                bar.wait();
            }
            out
        })
    };
    let intt = {
        let (i, bar) = (int.clone(), Arc::clone(&barrier));
        std::thread::spawn(move || {
            let rt = mt().build().unwrap();
            let mut out = Vec::new();
            for n in 0..N {
                bar.wait();
                let spin = Duration::from_micros(n % 60);
                let s = Instant::now();
                while s.elapsed() < spin {
                    std::hint::spin_loop();
                }
                out.push(rt.block_on(i.search(start_req(5))));
                bar.wait();
            }
            out
        })
    };
    let (mut lost, mut preempted) = (0usize, 0usize);
    let mut seen = 0usize;
    for _ in 0..N {
        barrier.wait();
        barrier.wait();
        let lines: Vec<String> = sp.shared.lines().split_off(seen);
        seen += lines.len();
        let go_bg = lines.iter().position(|l| l == "go movetime 20");
        let go_int = lines.iter().position(|l| l == "go movetime 5");
        let stop = lines.iter().position(|l| l == "stop");
        if let (Some(b), Some(i)) = (go_bg, go_int)
            && b < i
            && stop.is_none_or(|s| s < b || s > i)
        {
            lost += 1;
        }
    }
    for r in intt.join().unwrap() {
        assert!(r.is_ok(), "{r:?}");
    }
    for r in bgt.join().unwrap() {
        if matches!(r, Err(EngineError::Preempted { .. })) {
            preempted += 1;
        }
    }
    assert_eq!(lost, 0, "lost preemptions");
    assert!(
        preempted > 100,
        "the race was exercised: {preempted} preemptions"
    );
}

/// Runs `fut` to completion from a non-runtime thread, on the given runtime.
fn block_on_other<T: Send + 'static>(
    h: &tokio::runtime::Handle,
    fut: impl std::future::Future<Output = T> + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    h.spawn(async move {
        let _ = tx.send(fut.await);
    });
    rx.recv().unwrap()
}

// ---------------------------------------------------------------- EF-10

#[tokio::test]
async fn ef_10_a_drain_that_fails_kills_the_child_and_the_next_search_restarts() {
    let sp = ScriptedSpawner::with_queue(
        Scripted::default(),
        vec![Scripted {
            go: GoMode::Hang, // never answers, ignores `stop`: the drain cannot finish
            ..Scripted::default()
        }],
    );
    let e = engine_with(&sp, fast_cfg()); // drain_timeout 300 ms
    let abandoned =
        tokio::time::timeout(Duration::from_millis(60), e.search(start_req(5000))).await;
    assert!(abandoned.is_err());
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(
        sp.shared.alive(),
        0,
        "Draining --> Dead: the child that never answered is killed"
    );
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "e2e4");
    assert_eq!(sp.shared.spawns(), 2);
}

#[tokio::test]
async fn ef_10_a_child_that_exited_between_searches_is_restarted_transparently() {
    let sp = ScriptedSpawner::with_queue(
        Scripted::default(),
        vec![Scripted {
            exit_after_bestmoves: Some(1),
            ..Scripted::default()
        }],
    );
    let e = engine_with(&sp, fast_cfg());
    assert!(e.search(start_req(20)).await.is_ok());
    let second = e.search(start_req(20)).await;
    assert!(second.is_ok(), "no error for the caller: {second:?}");
    assert_eq!(sp.shared.spawns(), 2, "one transparent restart");
}

#[tokio::test]
async fn ef_10_a_caller_stop_of_a_background_search_is_not_a_preemption() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let pool = EnginePool::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
    )
    .unwrap();
    let bg = pool.analysis(AnalysisPriority::Background).unwrap();
    let bg2 = bg.clone();
    let s = tokio::spawn(async move { bg2.search(start_req(5000)).await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    bg.stop();
    let r = s
        .await
        .unwrap()
        .expect("a stop by the caller is a normal result, not Preempted");
    assert!(r.stopped_early);
}

#[tokio::test]
async fn ef_10_background_waits_for_every_queued_interactive_request() {
    let sp = ScriptedSpawner::new(Scripted::default());
    let pool = EnginePool::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
    )
    .unwrap();
    let int = pool.analysis(AnalysisPriority::Interactive).unwrap();
    let bg = pool.analysis(AnalysisPriority::Background).unwrap();
    let (i1, i2, b) = (int.clone(), int.clone(), bg.clone());
    let a = tokio::spawn(async move { i1.search(start_req(101)).await });
    tokio::time::sleep(Duration::from_millis(40)).await;
    let background = tokio::spawn(async move { b.search(start_req(103)).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    let queued = tokio::spawn(async move { i2.search(start_req(102)).await });
    a.await.unwrap().unwrap();
    queued.await.unwrap().unwrap();
    background.await.unwrap().unwrap();
    let go: Vec<String> = sp
        .shared
        .lines()
        .into_iter()
        .filter(|l| l.starts_with("go "))
        .collect();
    assert_eq!(
        go,
        ["go movetime 101", "go movetime 102", "go movetime 103"],
        "the background search arrived before the second interactive one and still ran last"
    );
}

/// A recording engine for `select_move`: returns `lines` MultiPV lines of distinct moves.
struct Rec {
    multipv_seen: std::sync::Mutex<Vec<u8>>,
    lines: usize,
}

const MOVES: [&str; 8] = [
    "a2a3", "b2b3", "c2c3", "d2d3", "e2e3", "f2f3", "g2g3", "h2h3",
];

impl Engine for Rec {
    async fn search(&self, req: SearchRequest) -> Result<chess_engine::SearchResult, EngineError> {
        self.multipv_seen.lock().unwrap().push(req.multipv);
        let lines: Vec<chess_engine::PvLine> = (0..self.lines)
            .map(|i| chess_engine::PvLine {
                multipv: u8::try_from(i + 1).unwrap(),
                score: Score::Cp(10),
                depth: 5,
                pv: vec![UciMove::parse(MOVES[i]).unwrap()],
            })
            .collect();
        Ok(chess_engine::SearchResult {
            best: UciMove::parse(MOVES[0]).unwrap(),
            pv: vec![UciMove::parse(MOVES[0]).unwrap()],
            score: Score::Cp(10),
            depth: 5,
            lines,
            stopped_early: false,
            elapsed: Duration::ZERO,
        })
    }
    fn stop(&self) {}
    fn health(&self) -> Health {
        Health::Ready
    }
}

fn pers(error_rate: f64, error_multipv: u8) -> chess_engine::EnginePersonality {
    chess_engine::EnginePersonality {
        error_rate,
        error_multipv,
        ..personality()
    }
}

#[tokio::test]
async fn ef_10_error_multipv_of_exactly_two_is_an_error_candidate() {
    let rec = Rec {
        multipv_seen: Default::default(),
        lines: 2,
    };
    let c = chess_engine::select_move(&rec, start_req(10), &pers(1.0, 2))
        .await
        .unwrap();
    assert_eq!(c.kind, chess_engine::MoveKind::Error);
    assert_eq!(
        c.mv.as_str(),
        MOVES[1],
        "with two lines the only error pick is line 2"
    );
    assert_eq!(*rec.multipv_seen.lock().unwrap(), vec![2]);
}

#[tokio::test]
async fn ef_10_without_an_error_roll_the_search_asks_for_one_line_whatever_the_caller_set() {
    let rec = Rec {
        multipv_seen: Default::default(),
        lines: 1,
    };
    let mut req = start_req(10);
    req.multipv = 5;
    let c = chess_engine::select_move(&rec, req, &pers(0.0, 3))
        .await
        .unwrap();
    assert_eq!(c.kind, chess_engine::MoveKind::Best);
    assert_eq!(*rec.multipv_seen.lock().unwrap(), vec![1]);
}

// ---------------------------------------------------------------- EF-11

const PICK_VECTORS: &str = include_str!("data/error_pick_vectors.txt");

#[test]
fn ef_11_next_f64_is_the_top_53_bits_of_the_first_draw() {
    let mut seen = 0;
    for line in PICK_VECTORS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let f: Vec<&str> = line.split_whitespace().collect();
        let seed: u64 = f[0].parse().unwrap();
        let roll53 = u64::from_str_radix(f[1], 16).unwrap();
        let u = chess_engine::SplitMix64::new(seed).next_f64();
        assert!((0.0..1.0).contains(&u));
        assert_eq!(u, roll53 as f64 / 9_007_199_254_740_992.0, "seed {seed}");
        seen += 1;
    }
    assert_eq!(seen, 32);
}

/// The replay contract end to end: seed -> roll -> which line is picked. `error_rate` just
/// above the roll must be an error move, equal to the roll must not (`u < error_rate`).
#[tokio::test]
async fn ef_11_select_move_replays_the_committed_rolls_and_picks() {
    for line in PICK_VECTORS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let f: Vec<&str> = line.split_whitespace().collect();
        let seed: u64 = f[0].parse().unwrap();
        let roll = u64::from_str_radix(f[1], 16).unwrap() as f64 / 9_007_199_254_740_992.0;
        let count: usize = f[2].parse().unwrap();
        let pick: usize = f[3].parse().unwrap();
        let rec = Rec {
            multipv_seen: Default::default(),
            lines: count,
        };
        let mut req = start_req(10);
        req.seed = seed;
        let above = f64::from_bits(roll.to_bits() + 1);
        let c = chess_engine::select_move(
            &rec,
            req.clone(),
            &pers(above, u8::try_from(count).unwrap()),
        )
        .await
        .unwrap();
        assert_eq!(c.kind, chess_engine::MoveKind::Error, "seed {seed}");
        assert_eq!(c.mv.as_str(), MOVES[pick], "seed {seed} count {count}");
        let c = chess_engine::select_move(&rec, req, &pers(roll, u8::try_from(count).unwrap()))
            .await
            .unwrap();
        assert_eq!(
            c.kind,
            chess_engine::MoveKind::Best,
            "seed {seed}: roll == rate is not an error"
        );
    }
}

// ---------------------------------------------------------------- EF-14

#[test]
fn ef_14_debug_of_an_error_never_prints_engine_text() {
    let secret = "SECRET-ENGINE-TEXT";
    for e in [
        EngineError::WrongVersion {
            found: secret.to_string(),
        },
        EngineError::InvalidPersonality {
            option: secret.to_string(),
        },
    ] {
        let d = format!("{e:?} {e:#?} {e}");
        assert!(!d.contains(secret), "{d}");
    }
    assert_eq!(format!("{:?}", EngineError::Busy), "Busy");
    assert!(format!("{:?}", EngineError::ProcessExited { code: Some(3) }).contains("Some(3)"));
}

// ---------------------------------------------------------------- boundaries

#[test]
fn ef_02_the_movetime_cap_boundary() {
    let max = SearchLimits::MAX_MOVETIME;
    assert!(limits_unchecked(max).validate().is_ok());
    assert!(SearchLimits::new(max, None, None).is_ok());
    assert_eq!(max, Duration::from_secs(3600), "ruled value: 1 hour");
    assert!(SearchLimits::new(Duration::MAX, None, None).is_err());
    assert!(matches!(
        limits_unchecked(max + Duration::from_millis(1)).validate(),
        Err(EngineError::InvalidRequest { .. })
    ));
    assert!(
        EngineConfig {
            hard_kill_grace: EngineConfig::MAX_DURATION,
            ..EngineConfig::default()
        }
        .validate()
        .is_ok()
    );
    assert!(
        EngineConfig {
            hard_kill_grace: EngineConfig::MAX_DURATION + Duration::from_nanos(1),
            ..EngineConfig::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn ef_07_the_move_count_boundary() {
    let at = long_request(SearchRequest::MAX_MOVES);
    assert!(at.validate(8).is_ok());
    assert!(
        long_request(SearchRequest::MAX_MOVES + 1)
            .validate(8)
            .is_err()
    );
}

#[tokio::test]
async fn ef_05_a_single_value_uci_elo_range_is_usable() {
    let sp = ScriptedSpawner::new(Scripted {
        uci_elo_range: Some((1500, 1500)),
        ..Scripted::default()
    });
    let mut p = personality();
    p.uci_elo = Some(1500);
    let e = UciEngine::new(
        fast_cfg(),
        located(BinarySource::Override),
        Arc::clone(&sp) as Arc<dyn Spawner>,
        InstanceProfile::Computer(p),
    )
    .unwrap();
    assert!(e.warm().await.is_ok());
}
