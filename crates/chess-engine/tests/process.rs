//! CT-EN-020 (process hygiene), CT-EN-051 (the real driver over the real `fake_uci` child) and
//! the fault modes of the child. Real processes, real pipes, real timers.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]
mod common;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chess_engine::{
    BinarySource, Engine, EngineConfig, EngineError, Health, InstanceProfile, LocatedBinary,
    ProcessSpawner, SearchPosition, Spawner, UciEngine, UciMove,
};
use chess_rules::Fen;
use common::{computer, fast_cfg, personality, process_cfg, start_req};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake_uci"))
}

fn engine(mode: Option<&str>, source: BinarySource) -> UciEngine {
    let args: Vec<OsString> = mode.map(OsString::from).into_iter().collect();
    // Handshake-fault modes need a short timeout; every other mode gets the production one.
    let cfg = match mode {
        Some("no-uciok" | "no-readyok") => EngineConfig {
            handshake_timeout: Duration::from_millis(1000),
            ..process_cfg()
        },
        _ => process_cfg(),
    };
    UciEngine::new(
        cfg,
        LocatedBinary {
            path: fake(),
            source,
        },
        Arc::new(ProcessSpawner::with_args(args)) as Arc<dyn Spawner>,
        computer(),
    )
    .unwrap()
}

fn mv(t: &str) -> UciMove {
    UciMove::parse(t).unwrap()
}

#[tokio::test]
async fn ct_en_051_the_real_driver_plays_the_first_legal_move_from_a_start_position() {
    let e = engine(None, BinarySource::Override);
    let r = e.search(start_req(40)).await.unwrap();
    assert_eq!(
        r.best.as_str(),
        "a2a3",
        "smallest legal move in UCI text order"
    );
    assert_eq!(e.health(), Health::Ready);
}

#[tokio::test]
async fn ct_en_051b_moves_and_fen_positions_reach_the_child() {
    let e = engine(None, BinarySource::Override);
    let mut req = start_req(30);
    req.position = SearchPosition::Moves {
        start: None,
        moves: vec![mv("a2a3"), mv("a7a6")],
    };
    assert_eq!(
        e.search(req).await.unwrap().best.as_str(),
        "a1a2",
        "after a3 a6 the rook can go a2"
    );
    let fen = Fen::parse("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let mut req = start_req(30);
    req.position = SearchPosition::Fen(fen.clone());
    assert_eq!(e.search(req).await.unwrap().best.as_str(), "e1d1");
    let mut req = start_req(30);
    req.position = SearchPosition::Moves {
        start: Some(fen),
        moves: vec![mv("e1e2")],
    };
    assert_eq!(e.search(req).await.unwrap().best.as_str(), "e8d7");
}

#[tokio::test]
async fn ct_en_051c_stop_and_multipv_work_through_a_real_pipe() {
    let e = engine(None, BinarySource::Override);
    let e2 = e.clone();
    let mut req = start_req(5000);
    req.multipv = 3;
    let s = tokio::spawn(async move { e2.search(req).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let t = Instant::now();
    e.stop();
    let r = s.await.unwrap().unwrap();
    assert!(r.stopped_early);
    assert!(
        t.elapsed() < Duration::from_millis(2000), // EF-09: far below the 5 s search
        "{:?}",
        t.elapsed()
    );
    assert_eq!(r.lines.len(), 3);
}

#[tokio::test]
async fn ct_en_052_fault_modes_of_the_child_map_to_the_documented_errors() {
    let e = engine(Some("exit-on-go"), BinarySource::Override);
    assert_eq!(
        e.search(start_req(200)).await.unwrap_err(),
        EngineError::ProcessExited { code: Some(3) }
    );
    assert_eq!(e.health(), Health::Dead);

    let e = engine(Some("hang-go"), BinarySource::Override);
    assert_eq!(
        e.search(start_req(50)).await.unwrap_err(),
        EngineError::SearchTimeout
    );

    let e = engine(Some("garbage-bestmove"), BinarySource::Override);
    assert_eq!(
        e.search(start_req(50)).await.unwrap_err(),
        EngineError::BadOutput
    );

    let e = engine(Some("long-line"), BinarySource::Override);
    assert_eq!(
        e.search(start_req(50)).await.unwrap_err(),
        EngineError::BadOutput
    );

    let e = engine(Some("no-uciok"), BinarySource::Override);
    assert_eq!(
        e.search(start_req(50)).await.unwrap_err(),
        EngineError::HandshakeTimeout
    );

    let e = engine(Some("no-readyok"), BinarySource::Override);
    assert_eq!(
        e.search(start_req(50)).await.unwrap_err(),
        EngineError::HandshakeTimeout
    );

    let e = engine(Some("wrong-id"), BinarySource::Bundled);
    assert!(matches!(
        e.search(start_req(50)).await.unwrap_err(),
        EngineError::WrongVersion { .. }
    ));
}

#[tokio::test]
async fn ct_en_053_spawn_of_a_missing_path_is_spawn_failed() {
    let e = UciEngine::new(
        fast_cfg(),
        LocatedBinary {
            path: PathBuf::from("/nonexistent/dir/engine"),
            source: BinarySource::Override,
        },
        Arc::new(ProcessSpawner::new()) as Arc<dyn Spawner>,
        InstanceProfile::Computer(personality()),
    )
    .unwrap();
    assert_eq!(
        e.search(start_req(30)).await.unwrap_err(),
        EngineError::SpawnFailed
    );
}

#[tokio::test]
async fn ct_en_020_the_child_has_an_empty_environment_its_own_process_group_and_is_reaped() {
    let spawned = ProcessSpawner::new().spawn(&fake()).unwrap();
    let mut control = spawned.control;
    let (mut stdin, stdout) = (spawned.stdin, spawned.stdout);
    let pid = control.id().unwrap();
    stdin.write_all(b"fakecmd env\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let l = tokio::time::timeout(Duration::from_secs(5), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // env_clear: no variable of this process reaches the engine. Under `cargo llvm-cov` the
    // instrumented child itself adds an LLVM_* variable, which is not ours.
    let names = l
        .strip_prefix("info string env [")
        .and_then(|r| r.strip_suffix(']'))
        .unwrap();
    for n in names.split(',').filter(|n| !n.is_empty()) {
        assert!(
            n.starts_with("LLVM_") || n.starts_with("__LLVM_"),
            "leaked variable {n}: {l}"
        );
    }
    for forbidden in ["PATH", "HOME", "USER", "CARGO"] {
        assert!(!names.split(',').any(|n| n.starts_with(forbidden)), "{l}");
    }

    let ps = |field: &str, p: u32| {
        String::from_utf8(
            Command::new("ps")
                .args(["-o", field, "-p", &p.to_string()])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    };
    let child_pgid = ps("pgid=", pid);
    let my_pgid = ps("pgid=", std::process::id());
    assert_eq!(
        child_pgid,
        pid.to_string(),
        "the child leads its own process group"
    );
    assert_ne!(child_pgid, my_pgid);

    control.start_kill();
    let info = control.wait().await;
    assert_ne!(info.code, Some(0), "killed, not a clean exit");
    assert!(
        !common::pid_alive(pid),
        "reaped: no process and no zombie remains"
    );
    assert_eq!(ps("stat=", pid), "");
}

#[tokio::test]
async fn ct_en_020b_closing_stdin_makes_the_child_exit() {
    let spawned = ProcessSpawner::new().spawn(&fake()).unwrap();
    let mut control = spawned.control;
    drop(spawned.stdin);
    let info = tokio::time::timeout(Duration::from_secs(3), control.wait())
        .await
        .expect("the child must exit on end of file");
    assert_eq!(info.code, Some(0));
    // Not tested here: a kill -9 of the parent. That needs a separate parent process, and the
    // end-of-file mechanism it relies on is what this test and EI-EN-006 exercise.
}

#[tokio::test]
async fn ct_en_054_dropping_every_handle_kills_and_reaps_the_child() {
    let sp = common::RecordingSpawner::new(ProcessSpawner::new());
    let e = UciEngine::new(
        process_cfg(),
        LocatedBinary {
            path: fake(),
            source: BinarySource::Override,
        },
        Arc::clone(&sp) as Arc<dyn Spawner>,
        computer(),
    )
    .unwrap();
    e.warm().await.unwrap();
    let pid = *sp.pids().last().unwrap(); // the live child, even if an earlier one was replaced
    assert!(common::pid_alive(pid));
    drop(e);
    let deadline = Instant::now() + Duration::from_secs(3);
    while common::pid_alive(pid) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        !common::pid_alive(pid),
        "the child must be gone once the handle is dropped"
    );
}

#[tokio::test]
async fn ct_en_055_an_external_kill_is_reported_as_a_crash_and_the_next_request_restarts() {
    let sp = common::RecordingSpawner::new(ProcessSpawner::new());
    let e = UciEngine::new(
        process_cfg(),
        LocatedBinary {
            path: fake(),
            source: BinarySource::Override,
        },
        Arc::clone(&sp) as Arc<dyn Spawner>,
        computer(),
    )
    .unwrap();
    let e2 = e.clone();
    let s = tokio::spawn(async move { e2.search(start_req(3000)).await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let pid = *sp.pids().last().unwrap(); // the live child, even if an earlier one was replaced
    Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .unwrap();
    let err = s.await.unwrap().unwrap_err();
    assert!(matches!(err, EngineError::ProcessExited { .. }), "{err:?}");
    assert_eq!(e.health(), Health::Dead);
    assert!(!common::pid_alive(pid), "reaped, no zombie");
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(r.best.as_str(), "a2a3");
    assert_eq!(sp.pids().len(), 2);
}

/// SDD 3.10.5 (EF-10): the child's standard error is the null device. `fake_uci` in
/// `stderr-probe` mode answers `h2h3` instead of its normal first move when its own stderr is
/// something else. (If this test binary itself runs with stderr redirected to /dev/null the
/// probe cannot tell the two apart for an `inherit` mutant: run it with a visible stderr.)
#[tokio::test]
async fn ef_10_the_childs_stderr_is_the_null_device() {
    let e = engine(Some("stderr-probe"), BinarySource::Override);
    let r = e.search(start_req(30)).await.unwrap();
    assert_eq!(
        r.best.as_str(),
        "a2a3",
        "h2h3 means the child's stderr is not /dev/null"
    );
}
