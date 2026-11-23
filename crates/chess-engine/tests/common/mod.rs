//! Shared test support: a scripted `Spawner` (an in-process "engine" task behind `duplex`
//! pipes) and small request helpers. Real timers with shrunken `EngineConfig` timeouts are used,
//! because the admitted tokio features do not include `test-util` (finding FIND-E2).
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test support: each test binary uses a different subset, and the helpers abort a failing fixture"
)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chess_engine::{
    BinarySource, BoxFuture, EngineConfig, EngineError, EnginePersonality, ExitInfo,
    InstanceProfile, LocatedBinary, PINNED_UCI_ELO_RANGE, ProcessControl, SearchLimits,
    SearchPosition, SearchRequest, SpawnedProcess, Spawner, UciEngine,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, duplex};
use tokio::task::JoinHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoMode {
    /// Think for movetime or until `stop`, then answer.
    Normal,
    /// Never answer, ignore `stop`.
    Hang,
    /// Drop the pipes (process exit).
    Crash,
    Garbage,
    LongLine,
    NoMove,
    /// Emit a burst of 60 info lines at once, then answer.
    Chatty,
    /// MultiPV lines at depth 1 (complete) and depth 2 (line 1 only), then wait like Normal.
    MultiPvTwoDepths,
    /// After `go`: an endless stream of valid `info` lines, never a `bestmove`, `stop` ignored
    /// (EF-01).
    InfoFlood,
    /// After `go`: an endless stream of bare newlines (EF-01, LineSplitter work bound).
    NewlineFlood,
    /// EF-08: `info ... pv e2e4 e7e5`, then a bound line for `d2d4`, then `bestmove d2d4`.
    PvVsBest,
    /// EF-08: only a line for `e2e4`, then `bestmove d2d4`: no line describes the best move.
    PvNoMatch,
    /// EF-01: like `InfoFlood` but answers `bestmove` when it reads `stop`.
    InfoFloodStoppable,
}

#[derive(Clone, Debug)]
pub struct Scripted {
    pub id_name: &'static str,
    pub send_uciok: bool,
    pub send_readyok: bool,
    pub go: GoMode,
    pub stop_delay: Duration,
    /// Moves for successive bestmoves; the last one repeats.
    pub bests: Vec<&'static str>,
    pub ignore_quit: bool,
    pub uci_elo_range: Option<(u32, u32)>,
    /// Capacity of each duplex pipe in bytes (small values make a stuck writer observable).
    pub pipe_bytes: usize,
    /// After the first `readyok` the engine task stops reading its input (EF-07).
    pub deaf_after_handshake: bool,
    /// The engine task ends (process exit) right after this many `bestmove`s (EF-10).
    pub exit_after_bestmoves: Option<usize>,
}

impl Default for Scripted {
    fn default() -> Scripted {
        Scripted {
            id_name: "Stockfish 19 (scripted)",
            send_uciok: true,
            send_readyok: true,
            go: GoMode::Normal,
            stop_delay: Duration::ZERO,
            bests: vec!["e2e4"],
            ignore_quit: false,
            uci_elo_range: Some((PINNED_UCI_ELO_RANGE.min, PINNED_UCI_ELO_RANGE.max)),
            pipe_bytes: 64 * 1024,
            deaf_after_handshake: false,
            exit_after_bestmoves: None,
        }
    }
}

#[derive(Default)]
pub struct Shared {
    pub spawns: AtomicUsize,
    pub alive: AtomicUsize,
    /// Every line the driver wrote, plus engine-side markers `<bestmove sent ...>`, in order.
    pub log: Mutex<Vec<String>>,
}

impl Shared {
    pub fn lines(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
    pub fn spawns(&self) -> usize {
        self.spawns.load(Ordering::SeqCst)
    }
    pub fn alive(&self) -> usize {
        self.alive.load(Ordering::SeqCst)
    }
    pub fn count_prefix(&self, prefix: &str) -> usize {
        self.lines()
            .iter()
            .filter(|l| l.starts_with(prefix))
            .count()
    }
    pub fn position_of(&self, prefix: &str, nth: usize) -> Option<usize> {
        self.lines()
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with(prefix))
            .nth(nth)
            .map(|(i, _)| i)
    }
}

pub struct ScriptedSpawner {
    queue: Mutex<VecDeque<Scripted>>,
    fallback: Scripted,
    pub fail_spawn: AtomicBool,
    pub shared: Arc<Shared>,
    /// Where the scripted engine tasks run. `None`: the runtime of the caller of `spawn`. A
    /// separate runtime thread makes engine and driver run in parallel, like a real process.
    pub engine_rt: Mutex<Option<tokio::runtime::Handle>>,
}

impl ScriptedSpawner {
    pub fn new(fallback: Scripted) -> Arc<ScriptedSpawner> {
        Arc::new(ScriptedSpawner {
            queue: Mutex::new(VecDeque::new()),
            fallback,
            fail_spawn: AtomicBool::new(false),
            shared: Arc::new(Shared::default()),
            engine_rt: Mutex::new(None),
        })
    }

    /// Runs the scripted engine tasks on `handle` from now on.
    pub fn run_engines_on(&self, handle: tokio::runtime::Handle) {
        *self.engine_rt.lock().unwrap() = Some(handle);
    }

    /// Behaviours for the next spawns, in order; `fallback` afterwards.
    pub fn with_queue(fallback: Scripted, queue: Vec<Scripted>) -> Arc<ScriptedSpawner> {
        let s = ScriptedSpawner::new(fallback);
        s.queue.lock().unwrap().extend(queue);
        s
    }
}

struct ScriptedControl {
    handle: Option<JoinHandle<()>>,
}

impl ProcessControl for ScriptedControl {
    fn try_wait(&mut self) -> Option<ExitInfo> {
        match &self.handle {
            Some(h) if h.is_finished() => Some(ExitInfo { code: Some(0) }),
            None => Some(ExitInfo { code: Some(0) }),
            _ => None,
        }
    }
    fn start_kill(&mut self) {
        if let Some(h) = &self.handle {
            h.abort();
        }
    }
    fn wait(&mut self) -> BoxFuture<'_, ExitInfo> {
        Box::pin(async move {
            if let Some(h) = self.handle.take() {
                let _ = h.await;
            }
            ExitInfo { code: Some(0) }
        })
    }
    fn id(&self) -> Option<u32> {
        None
    }
}

impl Spawner for ScriptedSpawner {
    fn spawn(&self, _path: &Path) -> Result<SpawnedProcess, EngineError> {
        if self.fail_spawn.load(Ordering::SeqCst) {
            return Err(EngineError::SpawnFailed);
        }
        let behavior = self
            .queue
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| self.fallback.clone());
        self.shared.spawns.fetch_add(1, Ordering::SeqCst);
        self.shared.alive.fetch_add(1, Ordering::SeqCst);
        let (drv_stdin, eng_in) = duplex(behavior.pipe_bytes);
        let (eng_out, drv_stdout) = duplex(behavior.pipe_bytes);
        let shared = Arc::clone(&self.shared);
        let guard = AliveGuard(Arc::clone(&self.shared));
        let task = async move {
            // Moved in at creation: dropped on normal end and when the task is aborted.
            let _guard = guard;
            engine_task(behavior, eng_in, eng_out, shared).await;
        };
        let rt = self.engine_rt.lock().unwrap().clone();
        let handle = match rt {
            Some(h) => h.spawn(task),
            None => tokio::spawn(task),
        };
        Ok(SpawnedProcess {
            stdin: Box::new(drv_stdin),
            stdout: Box::new(drv_stdout),
            control: Box::new(ScriptedControl {
                handle: Some(handle),
            }),
        })
    }
}

struct AliveGuard(Arc<Shared>);
impl Drop for AliveGuard {
    fn drop(&mut self) {
        self.0.alive.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn send(out: &mut DuplexStream, text: &str) -> bool {
    out.write_all(format!("{text}\n").as_bytes()).await.is_ok()
}

fn options_block(b: &Scripted) -> Vec<String> {
    let mut v = vec![
        format!("id name {}", b.id_name),
        "id author scripted".to_string(),
        "option name Threads type spin default 1 min 1 max 1024".to_string(),
        "option name Hash type spin default 16 min 1 max 33554432".to_string(),
        "option name MultiPV type spin default 1 min 1 max 500".to_string(),
        "option name Skill Level type spin default 20 min 0 max 20".to_string(),
        "option name UCI_LimitStrength type check default false".to_string(),
    ];
    if let Some((min, max)) = b.uci_elo_range {
        v.push(format!(
            "option name UCI_Elo type spin default {min} min {min} max {max}"
        ));
    }
    v
}

async fn engine_task(b: Scripted, input: DuplexStream, mut out: DuplexStream, shared: Arc<Shared>) {
    let mut reader = BufReader::new(input);
    let mut multipv = 1usize;
    let mut best_idx = 0usize;
    let mut sent_bestmoves = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let text = line.trim_end().to_string();
        shared.log.lock().unwrap().push(text.clone());
        let mut words = text.split_whitespace();
        match words.next() {
            Some("uci") => {
                for l in options_block(&b) {
                    if !send(&mut out, &l).await {
                        return;
                    }
                }
                if b.send_uciok && !send(&mut out, "uciok").await {
                    return;
                }
            }
            Some("isready") => {
                if b.send_readyok && !send(&mut out, "readyok").await {
                    return;
                }
                if b.deaf_after_handshake {
                    // Alive but never reads again: the driver's pipe fills up.
                    std::future::pending::<()>().await;
                }
            }
            Some("setoption") => {
                let rest: Vec<&str> = words.collect();
                if rest.get(1) == Some(&"MultiPV") {
                    multipv = rest.get(3).and_then(|v| v.parse().ok()).unwrap_or(1);
                }
            }
            Some("quit") => {
                if !b.ignore_quit {
                    return;
                }
            }
            Some("go") => {
                let args: Vec<&str> = words.collect();
                let ms: u64 = args
                    .iter()
                    .position(|w| *w == "movetime")
                    .and_then(|i| args.get(i + 1))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1000);
                let best = *b.bests.get(best_idx).or(b.bests.last()).unwrap_or(&"e2e4");
                match b.go {
                    GoMode::Crash => return,
                    GoMode::Hang => loop {
                        line.clear();
                        match reader.read_line(&mut line).await {
                            Ok(0) | Err(_) => return,
                            Ok(_) => shared.log.lock().unwrap().push(line.trim_end().to_string()),
                        }
                    },
                    GoMode::InfoFlood => loop {
                        if !send(&mut out, "info depth 1 score cp 1 pv e2e4").await {
                            return;
                        }
                    },
                    GoMode::NewlineFlood => {
                        let chunk = vec![b'\n'; 4096];
                        loop {
                            if out.write_all(&chunk).await.is_err() {
                                return;
                            }
                        }
                    }
                    GoMode::PvVsBest => {
                        send(&mut out, "info depth 5 score cp 10 pv e2e4 e7e5").await;
                        send(&mut out, "info depth 6 score cp 50 lowerbound pv d2d4").await;
                        send(&mut out, "bestmove d2d4").await;
                    }
                    GoMode::PvNoMatch => {
                        send(&mut out, "info depth 5 score cp 10 pv e2e4 e7e5").await;
                        send(&mut out, "bestmove d2d4").await;
                    }
                    GoMode::InfoFloodStoppable => loop {
                        line.clear();
                        tokio::select! {
                            biased;
                            r = reader.read_line(&mut line) => match r {
                                Ok(0) | Err(_) => return,
                                Ok(_) => {
                                    let t = line.trim_end().to_string();
                                    shared.log.lock().unwrap().push(t.clone());
                                    if t == "stop" {
                                        send(&mut out, &format!("bestmove {best}")).await;
                                        break;
                                    }
                                }
                            },
                            ok = send(&mut out, "info depth 1 score cp 1 pv e2e4") => {
                                if !ok { return; }
                            }
                        }
                    },
                    GoMode::Garbage => {
                        send(&mut out, "bestmove zzzz").await;
                    }
                    GoMode::LongLine => {
                        let big = "x".repeat(200_000);
                        send(&mut out, &big).await;
                    }
                    GoMode::NoMove => {
                        send(&mut out, "bestmove (none)").await;
                    }
                    GoMode::Chatty => {
                        for d in 1..=60 {
                            send(
                                &mut out,
                                &format!("info depth {d} multipv 1 score cp {d} pv {best}"),
                            )
                            .await;
                        }
                        send(&mut out, &format!("bestmove {best}")).await;
                        best_idx += 1;
                    }
                    GoMode::Normal | GoMode::MultiPvTwoDepths => {
                        if b.go == GoMode::MultiPvTwoDepths {
                            for n in 1..=multipv {
                                send(
                                    &mut out,
                                    &format!(
                                        "info depth 1 multipv {n} score cp {} pv a2a3",
                                        10 - n
                                    ),
                                )
                                .await;
                            }
                            send(&mut out, "info depth 2 multipv 1 score cp 99 pv a2a3").await;
                        } else {
                            send(&mut out, "info string NNUE evaluation using scripted").await;
                            send(&mut out, "some noise that is not UCI").await;
                            send(
                                &mut out,
                                &format!("info depth 1 seldepth 1 multipv 1 score cp 20 nodes 5 nps 9 hashfull 0 tbhits 0 time 1 pv {best}"),
                            )
                            .await;
                        }
                        let think = tokio::time::sleep(Duration::from_millis(ms));
                        tokio::pin!(think);
                        let mut stopped = false;
                        loop {
                            line.clear();
                            tokio::select! {
                                () = &mut think => break,
                                r = reader.read_line(&mut line) => match r {
                                    Ok(0) | Err(_) => return,
                                    Ok(_) => {
                                        let t = line.trim_end().to_string();
                                        shared.log.lock().unwrap().push(t.clone());
                                        if t == "stop" {
                                            stopped = true;
                                            break;
                                        }
                                    }
                                },
                            }
                        }
                        if stopped && !b.stop_delay.is_zero() {
                            tokio::time::sleep(b.stop_delay).await;
                        }
                        if !send(&mut out, &format!("bestmove {best}")).await {
                            return;
                        }
                        best_idx += 1;
                    }
                }
                shared
                    .log
                    .lock()
                    .unwrap()
                    .push("<bestmove sent>".to_string());
                sent_bestmoves += 1;
                if b.exit_after_bestmoves == Some(sent_bestmoves) {
                    return;
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------

pub fn fast_cfg() -> EngineConfig {
    EngineConfig {
        handshake_timeout: Duration::from_millis(300),
        soft_stop_grace: Duration::from_millis(80),
        hard_kill_grace: Duration::from_millis(300),
        drain_timeout: Duration::from_millis(300),
        shutdown_grace: Duration::from_millis(200),
        ..EngineConfig::default()
    }
}

/// For tests that start real child processes (EF-09, FIND-E13). The first `exec` of an
/// executable that this machine has not run before costs 130 to 200 ms (measured by the
/// reviewer), which is most of `fast_cfg`'s 300 ms handshake timeout: a missed handshake makes
/// the driver restart the child by design, and a test that looks at the first pid then sees
/// the dead one. Real-process tests therefore keep the soft, hard and drain limits short but
/// give the handshake the production value.
pub fn process_cfg() -> EngineConfig {
    EngineConfig {
        handshake_timeout: Duration::from_secs(5),
        ..fast_cfg()
    }
}

pub fn located(source: BinarySource) -> LocatedBinary {
    LocatedBinary {
        path: PathBuf::from("/scripted/engine"),
        source,
    }
}

pub fn personality() -> EnginePersonality {
    EnginePersonality {
        skill_level: 5,
        depth_limit: 0,
        movetime_ms: 1000,
        uci_elo: None,
        error_rate: 0.0,
        error_multipv: 3,
    }
}

pub fn computer() -> InstanceProfile {
    InstanceProfile::Computer(personality())
}

pub fn engine_with(spawner: &Arc<ScriptedSpawner>, cfg: EngineConfig) -> UciEngine {
    UciEngine::new(
        cfg,
        located(BinarySource::Override),
        Arc::clone(spawner) as Arc<dyn Spawner>,
        computer(),
    )
    .unwrap()
}

pub fn start_req(movetime_ms: u64) -> SearchRequest {
    SearchRequest::new(
        SearchPosition::Moves {
            start: None,
            moves: vec![],
        },
        SearchLimits::new(Duration::from_millis(movetime_ms), None, None).unwrap(),
    )
}

// ---------------------------------------------------------------------------------------
// A spawner that wraps the real one and remembers every child pid.

pub struct RecordingSpawner {
    inner: chess_engine::ProcessSpawner,
    pub pids: Arc<Mutex<Vec<u32>>>,
}

impl RecordingSpawner {
    pub fn new(inner: chess_engine::ProcessSpawner) -> Arc<RecordingSpawner> {
        Arc::new(RecordingSpawner {
            inner,
            pids: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn pids(&self) -> Vec<u32> {
        self.pids.lock().unwrap().clone()
    }
}

impl Spawner for RecordingSpawner {
    fn spawn(&self, path: &Path) -> Result<SpawnedProcess, EngineError> {
        let p = self.inner.spawn(path)?;
        if let Some(pid) = p.control.id() {
            self.pids.lock().unwrap().push(pid);
        }
        Ok(p)
    }
}

pub fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .unwrap()
        .status
        .success()
}

/// A runtime that runs on its own OS thread until the returned guard is dropped. `rt-multi-thread`
/// is not an admitted tokio feature, so parallelism comes from std threads.
pub struct RuntimeThread {
    pub handle: tokio::runtime::Handle,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

pub fn runtime_thread() -> RuntimeThread {
    let (htx, hrx) = std::sync::mpsc::channel();
    let (stop, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        htx.send(rt.handle().clone()).unwrap();
        rt.block_on(async {
            let _ = stop_rx.await;
        });
    });
    RuntimeThread {
        handle: hrx.recv().unwrap(),
        stop: Some(stop),
        thread: Some(thread),
    }
}

impl Drop for RuntimeThread {
    fn drop(&mut self) {
        if let Some(s) = self.stop.take() {
            let _ = s.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
