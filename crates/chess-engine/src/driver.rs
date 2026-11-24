//! DD-EN-004: `UciEngine`, the driver actor of one child process (SDD 3.8 to 3.10).
//!
//! One tokio task owns the child's stdin and stdout, so UCI ordering cannot be violated by
//! concurrent callers. The public handle sends commands over an unbounded channel (so `stop`
//! never blocks and is never lost) and gets replies on `oneshot` channels. A dropped
//! `search` future drops its `oneshot` receiver; the driver sees that, sends `stop` and
//! drains the abandoned `bestmove` before it accepts the next `go` (cancel safety, 3.10.3).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::Instant;

use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio::time::{Instant as TokioInstant, sleep_until, timeout};

use crate::config::EngineConfig;
use crate::engine::Engine;
use crate::error::EngineError;
use crate::locate::{BinarySource, LocatedBinary};
use crate::plan::{InstanceProfile, OptionPlan, PlanReport};
use crate::spawn::{ExitInfo, ProcessControl, SpawnedProcess, Spawner};
use crate::types::{Health, PvLine, Score, SearchInfo, SearchRequest, SearchResult};
use crate::uci::{
    InfoLine, LineSplitter, OptionValue, SetOption, UciMessage, UciOptions, format_go,
    format_isready, format_position, format_quit, format_setoption, format_stop, format_uci,
    format_ucinewgame, parse_line,
};

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// What the `uci` handshake told us (SDD 3.10.7: EngineInfo), for EI-EN-001, EI-EN-002 and
/// the clamp report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfo {
    pub id_name: Option<String>,
    pub uci_elo_range: Option<(u32, u32)>,
    pub options: UciOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeReport {
    pub info: EngineInfo,
    pub plan: PlanReport,
}

type SearchReply = oneshot::Sender<Result<SearchResult, EngineError>>;

enum Cmd {
    Search {
        req: Box<SearchRequest>,
        reply: SearchReply,
    },
    Stop,
    Warm {
        reply: oneshot::Sender<Result<(), EngineError>>,
    },
    Info {
        reply: oneshot::Sender<Option<HandshakeReport>>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

const H_NOT_STARTED: u8 = 0;
const H_READY: u8 = 1;
const H_SEARCHING: u8 = 2;
const H_DEAD: u8 = 3;

fn health_code(h: Health) -> u8 {
    match h {
        Health::NotStarted => H_NOT_STARTED,
        Health::Ready => H_READY,
        Health::Searching => H_SEARCHING,
        Health::Dead => H_DEAD,
    }
}

fn health_from(code: u8) -> Health {
    match code {
        H_READY => Health::Ready,
        H_SEARCHING => Health::Searching,
        H_DEAD => Health::Dead,
        _ => Health::NotStarted,
    }
}

/// Set by `UciEngine::shutdown` the moment it is called, so that a driver that is busy with
/// start-up, preparation, a write or a drain (none of which reads the command channel) still
/// notices (EF-06). `Cmd::Shutdown` follows on the channel and carries the acknowledgement.
struct Closing {
    flag: AtomicBool,
    notify: Notify,
}

impl Closing {
    fn new() -> Closing {
        Closing {
            flag: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    fn request(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    fn is_set(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Completes once `request` has been called (immediately if it already was).
    async fn wait(&self) {
        let notified = self.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if self.is_set() {
            return;
        }
        notified.await;
    }
}

/// `Some(output)`, or `None` as soon as shutdown was requested (the future is dropped).
async fn until_closing<T>(
    closing: &Closing,
    fut: impl std::future::Future<Output = T>,
) -> Option<T> {
    tokio::select! {
        biased;
        () = closing.wait() => None,
        v = fut => Some(v),
    }
}

/// The public handle. Cloning shares the instance. Create it inside a tokio runtime: it
/// spawns the driver task (no process is started until the first search or `warm`).
#[derive(Clone)]
pub struct UciEngine {
    inner: Arc<Inner>,
}

struct Inner {
    tx: mpsc::UnboundedSender<Cmd>,
    health: Arc<AtomicU8>,
    cfg: Arc<EngineConfig>,
    closing: Arc<Closing>,
}

impl UciEngine {
    pub fn new(
        cfg: EngineConfig,
        located: LocatedBinary,
        spawner: Arc<dyn Spawner>,
        profile: InstanceProfile,
    ) -> Result<UciEngine, EngineError> {
        cfg.validate()?;
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| EngineError::Closed)?;
        let cfg = Arc::new(cfg);
        let health = Arc::new(AtomicU8::new(H_NOT_STARTED));
        let (tx, rx) = mpsc::unbounded_channel();
        let closing = Arc::new(Closing::new());
        let driver = Driver {
            closing: Arc::clone(&closing),
            cfg: Arc::clone(&cfg),
            located,
            spawner,
            profile,
            health: Arc::clone(&health),
            id: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            proc: None,
            report: None,
        };
        runtime.spawn(driver.run(rx));
        Ok(UciEngine {
            inner: Arc::new(Inner {
                tx,
                health,
                cfg,
                closing,
            }),
        })
    }

    /// Starts the process and runs the handshake ahead of the first search (NFR-1).
    pub async fn warm(&self) -> Result<(), EngineError> {
        let (reply, rx) = oneshot::channel();
        self.inner
            .tx
            .send(Cmd::Warm { reply })
            .map_err(|_| EngineError::Closed)?;
        rx.await.map_err(|_| EngineError::Closed)?
    }

    /// The last successful handshake, or `None` if the process has not started.
    pub async fn handshake_report(&self) -> Option<HandshakeReport> {
        let (reply, rx) = oneshot::channel();
        self.inner.tx.send(Cmd::Info { reply }).ok()?;
        rx.await.ok()?
    }

    /// `quit`, the grace period, then kill. The instance is closed afterwards.
    pub async fn shutdown(&self) {
        let (reply, rx) = oneshot::channel();
        if self.inner.tx.send(Cmd::Shutdown { reply }).is_ok() {
            self.inner.closing.request();
            let _ = rx.await;
        }
    }
}

impl Engine for UciEngine {
    async fn search(&self, req: SearchRequest) -> Result<SearchResult, EngineError> {
        req.validate(self.inner.cfg.multipv_cap)?;
        let (reply, rx) = oneshot::channel();
        self.inner
            .tx
            .send(Cmd::Search {
                req: Box::new(req),
                reply,
            })
            .map_err(|_| EngineError::Closed)?;
        // Dropping this future drops `rx`; the driver notices and stops the search.
        rx.await.map_err(|_| EngineError::Closed)?
    }

    fn stop(&self) {
        // Unbounded: never blocks, never dropped. An error means the instance is closed.
        let _ = self.inner.tx.send(Cmd::Stop);
    }

    fn health(&self) -> Health {
        health_from(self.inner.health.load(Ordering::Relaxed))
    }
}

// ---------------------------------------------------------------------------------------
// The driver actor.

enum ReadFail {
    Eof,
    TooLong,
}

struct Proc {
    stdin: Box<dyn AsyncWrite + Send + Unpin>,
    stdout: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    splitter: LineSplitter,
    control: Box<dyn ProcessControl>,
    advertised: UciOptions,
    applied: HashMap<String, String>,
}

impl Proc {
    fn new(spawned: SpawnedProcess, line_cap: usize) -> Proc {
        Proc {
            stdin: spawned.stdin,
            stdout: spawned.stdout,
            splitter: LineSplitter::new(line_cap),
            control: spawned.control,
            advertised: UciOptions::new(),
            applied: HashMap::new(),
        }
    }

    async fn write_line(&mut self, line: &str) -> bool {
        let mut data = String::with_capacity(line.len() + 1);
        data.push_str(line);
        data.push('\n');
        self.stdin.write_all(data.as_bytes()).await.is_ok() && self.stdin.flush().await.is_ok()
    }

    /// Cancel-safe: the partial line lives in `splitter`, and `read` itself is cancel-safe.
    async fn next_msg(&mut self) -> Result<UciMessage, ReadFail> {
        let mut chunk = [0u8; 4096];
        loop {
            if let Some(line) = self.splitter.next_line() {
                return line.map(|l| parse_line(&l)).map_err(|_| ReadFail::TooLong);
            }
            let n = self
                .stdout
                .read(&mut chunk)
                .await
                .map_err(|_| ReadFail::Eof)?;
            if n == 0 {
                return Err(ReadFail::Eof);
            }
            self.splitter.push(chunk.get(..n).unwrap_or(&[]));
        }
    }
}

fn value_text(v: &OptionValue) -> String {
    match v {
        OptionValue::Bool(b) => b.to_string(),
        OptionValue::Int(i) => i.to_string(),
        OptionValue::Text(t) => t.clone(),
    }
}

enum Outcome {
    Done(Result<SearchResult, EngineError>),
    Abandoned,
    Shutdown(oneshot::Sender<()>),
    /// Shutdown was requested while the search was being set up; the process is already dead
    /// and `Cmd::Shutdown` is still queued.
    Closing,
    HandlesGone,
}

/// What `begin_search` achieved.
enum Begun {
    /// `go` has been written.
    Going,
    /// The caller dropped its future before `go`.
    Abandoned,
}

struct Driver {
    closing: Arc<Closing>,
    cfg: Arc<EngineConfig>,
    located: LocatedBinary,
    spawner: Arc<dyn Spawner>,
    profile: InstanceProfile,
    health: Arc<AtomicU8>,
    id: u64,
    proc: Option<Proc>,
    report: Option<HandshakeReport>,
}

impl Driver {
    fn set_health(&self, h: Health) {
        self.health.store(health_code(h), Ordering::Relaxed);
    }

    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Cmd>) {
        loop {
            let Some(cmd) = rx.recv().await else {
                self.die().await;
                return;
            };
            if self.closing.is_set() && !matches!(cmd, Cmd::Shutdown { .. }) {
                // `Cmd::Shutdown` is already queued behind this one; nothing may restart.
                self.die().await;
                match cmd {
                    Cmd::Search { reply, .. } => {
                        let _ = reply.send(Err(EngineError::Closed));
                    }
                    Cmd::Warm { reply } => {
                        let _ = reply.send(Err(EngineError::Closed));
                    }
                    Cmd::Info { reply } => {
                        let _ = reply.send(self.report.clone());
                    }
                    Cmd::Stop | Cmd::Shutdown { .. } => {}
                }
                continue;
            }
            let closing = Arc::clone(&self.closing);
            match cmd {
                // Nothing is running: `stop` does nothing.
                Cmd::Stop => {}
                Cmd::Warm { reply } => {
                    let r = match until_closing(&closing, self.ensure_started()).await {
                        Some(r) => r,
                        None => {
                            self.die().await;
                            Err(EngineError::Closed)
                        }
                    };
                    self.settle_health();
                    let _ = reply.send(r);
                }
                Cmd::Info { reply } => {
                    let _ = reply.send(self.report.clone());
                }
                Cmd::Shutdown { reply } => {
                    self.shutdown().await;
                    let _ = reply.send(());
                    return;
                }
                Cmd::Search { req, mut reply } => {
                    let outcome = if reply.is_closed() {
                        Outcome::Abandoned
                    } else {
                        self.run_search(&mut rx, &req, &mut reply).await
                    };
                    self.settle_health();
                    match outcome {
                        Outcome::Done(r) => {
                            let _ = reply.send(r);
                        }
                        Outcome::Abandoned => {}
                        Outcome::Closing => {
                            let _ = reply.send(Err(EngineError::Closed));
                        }
                        Outcome::Shutdown(ack) => {
                            let _ = reply.send(Err(EngineError::Closed));
                            self.shutdown().await;
                            let _ = ack.send(());
                            return;
                        }
                        Outcome::HandlesGone => {
                            self.die().await;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// `Ready` when a process is up and idle; a dead or never-started instance keeps its state.
    fn settle_health(&self) {
        if self.proc.is_some() {
            self.set_health(Health::Ready);
        }
    }

    /// Kills and reaps the child (every kill is followed by `wait`, no zombie remains).
    async fn die(&mut self) -> ExitInfo {
        let Some(mut p) = self.proc.take() else {
            return ExitInfo { code: None };
        };
        p.control.start_kill();
        let info = timeout(self.cfg.shutdown_grace, p.control.wait())
            .await
            .unwrap_or(ExitInfo { code: None });
        self.set_health(Health::Dead);
        info
    }

    async fn shutdown(&mut self) {
        if let Some(mut p) = self.proc.take() {
            let quit_written = timeout(self.cfg.shutdown_grace, p.write_line(&format_quit()))
                .await
                .unwrap_or(false);
            if quit_written {
                let exited = timeout(self.cfg.shutdown_grace, p.control.wait()).await;
                if exited.is_ok() {
                    self.set_health(Health::Dead);
                    return;
                }
            }
            p.control.start_kill();
            let _ = timeout(self.cfg.shutdown_grace, p.control.wait()).await;
        }
        self.set_health(Health::Dead);
    }

    async fn read_fail(&mut self, f: ReadFail) -> EngineError {
        let info = self.die().await;
        tracing::warn!(instance = self.id, "engine output failure");
        match f {
            ReadFail::TooLong => EngineError::BadOutput,
            ReadFail::Eof => EngineError::ProcessExited { code: info.code },
        }
    }

    async fn write(&mut self, line: &str) -> Result<(), EngineError> {
        // Every write has a deadline (EF-07): an engine that stops reading its input must not
        // park the driver where no timer can reach it. The handshake timeout is the bound.
        let outcome = match self.proc.as_mut() {
            Some(p) => timeout(self.cfg.handshake_timeout, p.write_line(line))
                .await
                .map_err(|_| ()),
            None => Ok(false),
        };
        match outcome {
            Ok(true) => Ok(()),
            Ok(false) => {
                let info = self.die().await;
                Err(EngineError::ProcessExited { code: info.code })
            }
            Err(()) => {
                self.die().await;
                tracing::warn!(instance = self.id, "engine does not read its input");
                Err(EngineError::SearchTimeout)
            }
        }
    }

    async fn read_msg(&mut self) -> Result<UciMessage, ReadFail> {
        match self.proc.as_mut() {
            Some(p) => p.next_msg().await,
            None => Err(ReadFail::Eof),
        }
    }

    /// Reads until `done` returns true, within the handshake timeout. Kills on any failure.
    async fn expect(
        &mut self,
        mut done: impl FnMut(UciMessage) -> bool,
    ) -> Result<(), EngineError> {
        let to = self.cfg.handshake_timeout;
        let result = timeout(to, async {
            loop {
                match self.read_msg().await {
                    Ok(m) => {
                        if done(m) {
                            return Ok(());
                        }
                    }
                    Err(f) => return Err(f),
                }
            }
        })
        .await;
        match result {
            Err(_elapsed) => {
                self.die().await;
                Err(EngineError::HandshakeTimeout)
            }
            Ok(Err(f)) => Err(self.read_fail(f).await),
            Ok(Ok(())) => Ok(()),
        }
    }

    async fn ensure_started(&mut self) -> Result<(), EngineError> {
        // A child that exited between searches is restarted transparently (3.10.4).
        if self
            .proc
            .as_mut()
            .is_some_and(|p| p.control.try_wait().is_some())
        {
            self.die().await;
        }
        if self.proc.is_none() {
            self.start().await?;
        }
        Ok(())
    }

    /// `Starting`: one automatic restart on a handshake timeout, then the error.
    async fn start(&mut self) -> Result<(), EngineError> {
        let mut attempt = 0u8;
        loop {
            match self.start_once().await {
                Ok(()) => return Ok(()),
                Err(EngineError::HandshakeTimeout) if attempt == 0 => {
                    attempt += 1;
                    tracing::warn!(instance = self.id, "handshake timeout, restarting once");
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn start_once(&mut self) -> Result<(), EngineError> {
        let spawned = match self.spawner.spawn(&self.located.path) {
            Ok(s) => s,
            Err(e) => {
                self.set_health(Health::Dead);
                return Err(e);
            }
        };
        self.proc = Some(Proc::new(spawned, self.cfg.line_cap));
        match self.handshake().await {
            Ok(()) => {
                self.set_health(Health::Ready);
                Ok(())
            }
            Err(e) => {
                self.die().await;
                Err(e)
            }
        }
    }

    async fn handshake(&mut self) -> Result<(), EngineError> {
        self.write(&format_uci()).await?;
        let mut id_name: Option<String> = None;
        let mut options = UciOptions::new();
        self.expect(|m| match m {
            UciMessage::IdName(n) => {
                id_name = Some(n);
                false
            }
            UciMessage::Option(spec) => {
                options.insert(spec);
                false
            }
            UciMessage::UciOk => true,
            _ => false,
        })
        .await?;

        if self.located.source == BinarySource::Bundled {
            let ok = id_name
                .as_deref()
                .is_some_and(|n| n.contains(&self.cfg.pinned_id_text));
            if !ok {
                let found: String = id_name.unwrap_or_default().chars().take(64).collect();
                return Err(EngineError::WrongVersion { found });
            }
        }

        let plan = OptionPlan::build(&self.profile, &self.cfg, &options)?;
        if plan.report.clamped || plan.report.differs_from_pinned {
            tracing::warn!(
                instance = self.id,
                clamped = plan.report.clamped,
                differs_from_pinned = plan.report.differs_from_pinned,
                "installation defect: UCI_Elo range"
            );
        }
        let mut applied: HashMap<String, String> = HashMap::new();
        for spec in options.iter() {
            if let Some(d) = spec.default_text() {
                applied.insert(spec.name.clone(), d);
            }
        }
        for set in &plan.options {
            let Some(spec) = options.get(&set.name) else {
                continue;
            };
            let text = value_text(&set.value);
            if applied.get(&set.name) == Some(&text) {
                continue;
            }
            let line = format_setoption(spec, set)?;
            self.write(&line).await?;
            applied.insert(set.name.clone(), text);
        }
        self.write(&format_isready()).await?;
        self.expect(|m| m == UciMessage::ReadyOk).await?;

        self.report = Some(HandshakeReport {
            info: EngineInfo {
                id_name,
                uci_elo_range: options.uci_elo_range(),
                options: options.clone(),
            },
            plan: plan.report,
        });
        if let Some(p) = self.proc.as_mut() {
            p.advertised = options;
            p.applied = applied;
        }
        Ok(())
    }

    /// MultiPV, `ucinewgame`, and an `isready` barrier when anything was sent.
    async fn prepare(&mut self, req: &SearchRequest) -> Result<(), EngineError> {
        let mut sent = false;
        let multipv_line = {
            let p = self.proc.as_mut().ok_or(EngineError::Closed)?;
            let wanted = i64::from(req.multipv);
            match p.advertised.get("MultiPV").cloned() {
                Some(spec) => {
                    let text = wanted.to_string();
                    if p.applied.get("MultiPV") == Some(&text) {
                        None
                    } else {
                        let set = SetOption {
                            name: "MultiPV".to_string(),
                            value: OptionValue::Int(wanted),
                        };
                        let line = format_setoption(&spec, &set).map_err(|_| {
                            EngineError::InvalidRequest {
                                reason: "multipv out of the engine's range",
                            }
                        })?;
                        p.applied.insert("MultiPV".to_string(), text);
                        Some(line)
                    }
                }
                None if req.multipv > 1 => {
                    return Err(EngineError::InvalidRequest {
                        reason: "engine has no MultiPV option",
                    });
                }
                None => None,
            }
        };
        if let Some(line) = multipv_line {
            self.write(&line).await?;
            sent = true;
        }
        if req.new_game {
            self.write(&format_ucinewgame()).await?;
            sent = true;
        }
        if sent {
            self.write(&format_isready()).await?;
            self.expect(|m| m == UciMessage::ReadyOk).await?;
        }
        Ok(())
    }

    /// Everything before `go` returns: format, start the process if needed, prepare, write.
    /// Shutdown can interrupt it (the caller wraps it in `until_closing`).
    async fn begin_search(
        &mut self,
        req: &SearchRequest,
        reply: &SearchReply,
    ) -> Result<Begun, EngineError> {
        let position = format_position(&req.position)?;
        if Self::deadlines(&self.cfg, req.limits.movetime).is_none() {
            return Err(EngineError::InvalidRequest {
                reason: "movetime exceeds the maximum",
            });
        }
        self.ensure_started().await?;
        self.prepare(req).await?;
        if reply.is_closed() {
            return Ok(Begun::Abandoned);
        }
        self.write(&position).await?;
        self.write(&format_go(&req.limits)).await?;
        Ok(Begun::Going)
    }

    /// The soft and hard deadlines of a search that starts now, or `None` if the arithmetic
    /// would overflow `Instant` (validation makes that unreachable; this is the second line).
    fn deadlines(
        cfg: &EngineConfig,
        movetime: std::time::Duration,
    ) -> Option<(TokioInstant, TokioInstant)> {
        let base = TokioInstant::now().checked_add(movetime)?;
        Some((
            base.checked_add(cfg.soft_stop_grace)?,
            base.checked_add(cfg.hard_kill_grace)?,
        ))
    }

    async fn run_search(
        &mut self,
        rx: &mut mpsc::UnboundedReceiver<Cmd>,
        req: &SearchRequest,
        reply: &mut SearchReply,
    ) -> Outcome {
        let closing = Arc::clone(&self.closing);
        match until_closing(&closing, self.begin_search(req, reply)).await {
            None => {
                self.die().await;
                Outcome::Closing
            }
            Some(Err(e)) => Outcome::Done(Err(e)),
            Some(Ok(Begun::Abandoned)) => Outcome::Abandoned,
            Some(Ok(Begun::Going)) => {
                self.set_health(Health::Searching);
                self.search_loop(rx, req, reply, Instant::now()).await
            }
        }
    }

    async fn search_loop(
        &mut self,
        rx: &mut mpsc::UnboundedReceiver<Cmd>,
        req: &SearchRequest,
        reply: &mut SearchReply,
        go_at: Instant,
    ) -> Outcome {
        // Validated before `go`; if it cannot be computed now the deadlines fire at once.
        let (soft_at, hard_at) = Self::deadlines(&self.cfg, req.limits.movetime)
            .unwrap_or_else(|| (TokioInstant::now(), TokioInstant::now()));
        let soft = sleep_until(soft_at);
        let hard = sleep_until(hard_at);
        tokio::pin!(soft);
        tokio::pin!(hard);
        let mut soft_done = false;
        let mut stopping = false;
        let mut acc = Accum::new(req.multipv);
        let mut last_emit: HashMap<u8, Instant> = HashMap::new();

        loop {
            tokio::select! {
                // Biased. A dropped future is seen first, so a search that the same caller
                // sends right after the drop waits for the drain instead of being answered
                // `Busy` (3.10.3). The two timers come next and BEFORE the engine's output:
                // a `select!` stops at the first ready branch, so with the timers behind
                // `read` an engine that never stops printing would starve them for ever
                // (EF-01). Commands and output follow.
                biased;
                () = reply.closed() => {
                    if !stopping {
                        // A failed write is a dead process; `drain` then fails fast.
                        let _ = self.write(&format_stop()).await;
                    }
                    self.drain().await;
                    return Outcome::Abandoned;
                },
                () = &mut hard => {
                    self.die().await;
                    return Outcome::Done(Err(EngineError::SearchTimeout));
                },
                () = &mut soft, if !soft_done => {
                    soft_done = true;
                    if !stopping {
                        if let Err(e) = self.write(&format_stop()).await {
                            return Outcome::Done(Err(e));
                        }
                        stopping = true;
                    }
                },
                cmd = rx.recv() => match cmd {
                    Some(Cmd::Stop) => {
                        if !stopping {
                            if let Err(e) = self.write(&format_stop()).await {
                                return Outcome::Done(Err(e));
                            }
                            stopping = true;
                        }
                    }
                    Some(Cmd::Search { reply: other, .. }) => {
                        let _ = other.send(Err(EngineError::Busy));
                    }
                    Some(Cmd::Warm { reply: r }) => {
                        let _ = r.send(Ok(()));
                    }
                    Some(Cmd::Info { reply: r }) => {
                        let _ = r.send(self.report.clone());
                    }
                    Some(Cmd::Shutdown { reply: ack }) => return Outcome::Shutdown(ack),
                    None => return Outcome::HandlesGone,
                },
                msg = self.read_msg() => match msg {
                    Err(f) => {
                        let e = self.read_fail(f).await;
                        return Outcome::Done(Err(e));
                    }
                    Ok(UciMessage::Info(info)) => {
                        if let Some(line) = acc.push(&info) {
                            emit_update(req, &line, &mut last_emit, self.cfg.update_interval);
                        }
                    }
                    Ok(UciMessage::BestMove(best)) => {
                        let result = acc.finish(best, stopping, go_at.elapsed());
                        return Outcome::Done(Ok(result));
                    }
                    Ok(UciMessage::NoMove | UciMessage::MalformedBestMove) => {
                        // `bestmove (none)` or a bestmove that is not a syntactic move.
                        self.die().await;
                        return Outcome::Done(Err(EngineError::BadOutput));
                    }
                    Ok(_) => {}
                },
            }
        }
    }

    /// Discards output until the abandoned search's `bestmove`; no `go` is sent meanwhile.
    async fn drain(&mut self) {
        let to = self.cfg.drain_timeout;
        let closing = Arc::clone(&self.closing);
        let result = until_closing(
            &closing,
            timeout(to, async {
                loop {
                    match self.read_msg().await? {
                        UciMessage::BestMove(_)
                        | UciMessage::NoMove
                        | UciMessage::MalformedBestMove => return Ok::<(), ReadFail>(()),
                        _ => {}
                    }
                }
            }),
        )
        .await;
        match result {
            Some(Ok(Ok(()))) => {}
            // A read failure, the drain timeout, or shutdown: the process cannot be trusted
            // to be idle, so it is killed (`Draining --> Dead`).
            Some(Ok(Err(_)) | Err(_)) | None => {
                self.die().await;
            }
        }
    }
}

fn emit_update(
    req: &SearchRequest,
    line: &PvLine,
    last_emit: &mut HashMap<u8, Instant>,
    interval: std::time::Duration,
) {
    let Some(tx) = req.updates.as_ref() else {
        return;
    };
    let now = Instant::now();
    if last_emit
        .get(&line.multipv)
        .is_some_and(|t| now.duration_since(*t) < interval)
    {
        return;
    }
    last_emit.insert(line.multipv, now);
    // A full channel drops the update (SDD 3.6.1).
    let _ = tx.try_send(SearchInfo {
        depth: line.depth,
        multipv: line.multipv,
        score: line.score,
        pv: line.pv.clone(),
    });
}

/// Collects `info` lines into "all MultiPV lines of the last completed depth".
struct Accum {
    want: usize,
    cur_depth: u32,
    cur: Vec<PvLine>,
    last: Vec<PvLine>,
    /// The latest bound line (`lowerbound` or `upperbound`) of line 1. Not part of `lines`; it
    /// is only used when it is the line that names the returned best move (EF-08).
    bound_main: Option<PvLine>,
}

impl Accum {
    fn new(multipv: u8) -> Accum {
        Accum {
            want: usize::from(multipv),
            cur_depth: 0,
            cur: Vec::new(),
            last: Vec::new(),
            bound_main: None,
        }
    }

    fn push(&mut self, info: &InfoLine) -> Option<PvLine> {
        let (depth, score) = (info.depth?, info.score?);
        if info.bound {
            if info.multipv.unwrap_or(1) == 1 && !info.pv.is_empty() {
                self.bound_main = Some(PvLine {
                    multipv: 1,
                    score,
                    depth,
                    pv: info.pv.clone(),
                });
            }
            return None;
        }
        if depth != self.cur_depth {
            if self.cur.len() >= self.want {
                self.last = std::mem::take(&mut self.cur);
            } else {
                self.cur.clear();
            }
            self.cur_depth = depth;
        }
        let line = PvLine {
            multipv: info.multipv.unwrap_or(1),
            score,
            depth,
            pv: info.pv.clone(),
        };
        match self.cur.iter_mut().find(|l| l.multipv == line.multipv) {
            Some(slot) => *slot = line.clone(),
            None => self.cur.push(line.clone()),
        }
        Some(line)
    }

    fn finish(
        mut self,
        best: crate::types::UciMove,
        stopped_early: bool,
        elapsed: std::time::Duration,
    ) -> SearchResult {
        let mut lines = if self.cur.len() >= self.want || self.last.is_empty() {
            std::mem::take(&mut self.cur)
        } else {
            std::mem::take(&mut self.last)
        };
        lines.sort_by_key(|l| l.multipv);
        let main = lines.iter().find(|l| l.multipv == 1).or(lines.first());
        // `pv`, `score` and `depth` describe `best` (EF-08): the completed line when it starts
        // with `best`; else the latest bound line when it does (its score is a bound); else
        // nothing is known about this move and the result says so (`Cp(0)`, depth 0).
        let names_best = |l: &PvLine| l.pv.first() == Some(&best);
        let (score, depth, pv) = match main {
            Some(l) if l.pv.is_empty() => (l.score, l.depth, vec![best.clone()]),
            Some(l) if names_best(l) => (l.score, l.depth, l.pv.clone()),
            _ => match self.bound_main.as_ref().filter(|l| names_best(l)) {
                Some(l) => (l.score, l.depth, l.pv.clone()),
                None => (Score::Cp(0), 0, vec![best.clone()]),
            },
        };
        SearchResult {
            best,
            pv,
            score,
            depth,
            lines,
            stopped_early,
            elapsed,
        }
    }
}
