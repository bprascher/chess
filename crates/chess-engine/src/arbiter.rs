//! DD-EN-010: priority between interactive and background analysis on the one shared
//! instance (SDD 3.10.9). Scheduling by preemption lanes, not by OS priority (UR-39).

use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use tokio::sync::{Mutex, watch};

use crate::driver::UciEngine;
use crate::engine::Engine;
use crate::error::EngineError;
use crate::types::{SearchRequest, SearchResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisPriority {
    /// Training live analysis, what-if, post-game on demand. First in first out among
    /// themselves; a caller that wants "newer replaces older" drops the older future.
    Interactive,
    /// Per-move evaluation. Runs only when no interactive request waits; preempted by one.
    Background,
}

const NONE: u8 = 0;
const INTERACTIVE: u8 = 1;
const BACKGROUND: u8 = 2;

fn code(p: AnalysisPriority) -> u8 {
    match p {
        AnalysisPriority::Interactive => INTERACTIVE,
        AnalysisPriority::Background => BACKGROUND,
    }
}

pub(crate) struct Arbiter {
    engine: UciEngine,
    /// tokio's mutex is fair (FIFO).
    lock: Mutex<()>,
    waiting: watch::Sender<usize>,
    running: AtomicU8,
    preempts: AtomicU64,
    /// One stop signal per lane (a counter that is bumped to ask for a stop). A lane's search
    /// listens to its own signal and calls `engine.stop()` itself, *after* its `Search` command
    /// is on the driver's channel. `UciEngine::stop` has no memory: a `Stop` that reaches an idle
    /// driver before the `Search` is discarded (EF-04). Sending the stop from the search's own
    /// task, in order behind its `Search`, closes that window.
    stop_signal: [watch::Sender<u64>; 2],
}

struct CountGuard<'a>(&'a watch::Sender<usize>);

impl<'a> CountGuard<'a> {
    fn new(tx: &'a watch::Sender<usize>) -> CountGuard<'a> {
        tx.send_modify(|n| *n += 1);
        CountGuard(tx)
    }
}

impl Drop for CountGuard<'_> {
    fn drop(&mut self) {
        self.0.send_modify(|n| *n = n.saturating_sub(1));
    }
}

/// Publishes which lane runs. It clears the slot only if the slot still holds its own code, so
/// a guard that outlives its lock can never wipe the next owner's value (EF-04, the suspected
/// clobber: the old guard stored `NONE` unconditionally).
struct RunningGuard<'a> {
    slot: &'a AtomicU8,
    mine: u8,
}

impl<'a> RunningGuard<'a> {
    fn set(slot: &'a AtomicU8, mine: u8) -> RunningGuard<'a> {
        slot.store(mine, Ordering::SeqCst);
        RunningGuard { slot, mine }
    }
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        let _ = self
            .slot
            .compare_exchange(self.mine, NONE, Ordering::SeqCst, Ordering::SeqCst);
    }
}

impl Arbiter {
    pub(crate) fn new(engine: UciEngine) -> Arbiter {
        let (waiting, _rx) = watch::channel(0usize);
        Arbiter {
            engine,
            lock: Mutex::new(()),
            waiting,
            running: AtomicU8::new(NONE),
            preempts: AtomicU64::new(0),
            stop_signal: [watch::channel(0u64).0, watch::channel(0u64).0],
        }
    }

    pub(crate) fn engine(&self) -> &UciEngine {
        &self.engine
    }

    fn lane(&self, code: u8) -> &watch::Sender<u64> {
        // INTERACTIVE is 1 and BACKGROUND is 2.
        let idx = usize::from(code == BACKGROUND);
        self.stop_signal.get(idx).unwrap_or(&self.stop_signal[0])
    }

    pub(crate) async fn search(
        &self,
        priority: AnalysisPriority,
        req: SearchRequest,
    ) -> Result<SearchResult, EngineError> {
        match priority {
            AnalysisPriority::Interactive => self.interactive(req).await,
            AnalysisPriority::Background => self.background(req).await,
        }
    }

    /// Stops the shared search only if it belongs to this lane.
    pub(crate) fn stop(&self, priority: AnalysisPriority) {
        let c = code(priority);
        if self.running.load(Ordering::SeqCst) == c {
            self.lane(c).send_modify(|n| *n = n.wrapping_add(1));
        }
    }

    /// Runs the search of lane `c`, which must already be published in `running` with
    /// `signal` subscribed before that (so no bump can be missed). Polls the search first, so
    /// its `Search` command is queued before any `stop` this lane sends.
    async fn run_lane(
        &self,
        mut signal: watch::Receiver<u64>,
        req: SearchRequest,
    ) -> Result<SearchResult, EngineError> {
        let search = self.engine.search(req);
        tokio::pin!(search);
        let mut stopped = false;
        loop {
            tokio::select! {
                biased;
                r = &mut search => return r,
                changed = signal.changed(), if !stopped => {
                    stopped = true;
                    if changed.is_ok() {
                        self.engine.stop();
                    }
                }
            }
        }
    }

    async fn interactive(&self, req: SearchRequest) -> Result<SearchResult, EngineError> {
        let _count = CountGuard::new(&self.waiting);
        if self.running.load(Ordering::SeqCst) == BACKGROUND {
            self.preempts.fetch_add(1, Ordering::SeqCst);
            self.lane(BACKGROUND)
                .send_modify(|n| *n = n.wrapping_add(1));
        }
        let _lock = self.lock.lock().await;
        let signal = self.lane(INTERACTIVE).subscribe();
        let _running = RunningGuard::set(&self.running, INTERACTIVE);
        self.run_lane(signal, req).await
    }

    async fn background(&self, req: SearchRequest) -> Result<SearchResult, EngineError> {
        let (lock, signal) = loop {
            let mut rx = self.waiting.subscribe();
            rx.wait_for(|n| *n == 0)
                .await
                .map_err(|_| EngineError::Closed)?;
            let lock = self.lock.lock().await;
            // Subscribe BEFORE publishing `running`: an interactive request that sees
            // `BACKGROUND` bumps the signal afterwards, and the subscription then sees it.
            let signal = self.lane(BACKGROUND).subscribe();
            self.running.store(BACKGROUND, Ordering::SeqCst);
            // Re-check after publishing `running`: an interactive request that arrived in
            // between either saw `running` (and signalled us) or is counted here.
            if *self.waiting.borrow() > 0 {
                let _ = self.running.compare_exchange(
                    BACKGROUND,
                    NONE,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                drop(lock);
                continue;
            }
            break (lock, signal);
        };
        // `_lock` is declared first, so it drops last: the next owner of the lock publishes its
        // own lane only after this guard has finished.
        let _lock = lock;
        let _running = RunningGuard {
            slot: &self.running,
            mine: BACKGROUND,
        };
        let before = self.preempts.load(Ordering::SeqCst);
        let result = self.run_lane(signal, req).await;
        match result {
            Ok(res) if res.stopped_early && self.preempts.load(Ordering::SeqCst) != before => {
                Err(EngineError::Preempted {
                    partial: Some(Box::new(res)),
                })
            }
            other => other,
        }
    }
}
