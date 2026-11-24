//! DD-EN-007: instances per computer side, the shared analysis instance, lifecycle.
//! Nothing is started by `EnginePool::new` (NFR-1). The map lock is never held across `.await`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chess_rules::Side;

use crate::arbiter::{AnalysisPriority, Arbiter};
use crate::config::EngineConfig;
use crate::driver::{HandshakeReport, UciEngine};
use crate::engine::Engine;
use crate::error::EngineError;
use crate::locate::LocatedBinary;
use crate::plan::InstanceProfile;
use crate::spawn::Spawner;
use crate::types::{EnginePersonality, Health, SearchRequest, SearchResult};

/// An opaque game identifier. This crate has no edge to `chess-domain`; `chess-app` converts
/// its `GameId` (for example from the UUID's 128 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameKey(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComputerKey {
    pub game: GameKey,
    pub side: Side,
}

fn locked<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Shuts the instances down in parallel, so that stuck ones do not add up (EF-06).
async fn shutdown_all(engines: Vec<UciEngine>) {
    let mut set = tokio::task::JoinSet::new();
    for e in engines {
        set.spawn(async move { e.shutdown().await });
    }
    while set.join_next().await.is_some() {}
}

#[derive(Clone)]
pub struct EnginePool {
    inner: Arc<PoolInner>,
}

struct PoolInner {
    cfg: EngineConfig,
    located: LocatedBinary,
    spawner: Arc<dyn Spawner>,
    computers: Mutex<HashMap<ComputerKey, UciEngine>>,
    analysis: Mutex<Option<Arc<Arbiter>>>,
}

impl EnginePool {
    pub fn new(
        cfg: EngineConfig,
        located: LocatedBinary,
        spawner: Arc<dyn Spawner>,
    ) -> Result<EnginePool, EngineError> {
        cfg.validate()?;
        Ok(EnginePool {
            inner: Arc::new(PoolInner {
                cfg,
                located,
                spawner,
                computers: Mutex::new(HashMap::new()),
                analysis: Mutex::new(None),
            }),
        })
    }

    fn make(&self, profile: InstanceProfile) -> Result<UciEngine, EngineError> {
        UciEngine::new(
            self.inner.cfg.clone(),
            self.inner.located.clone(),
            Arc::clone(&self.inner.spawner),
            profile,
        )
    }

    /// Creates the instance on first use. A key that already exists keeps its first
    /// personality (PLY-4: parameters are fixed after creation). Self-play uses two keys and
    /// so two processes.
    pub fn computer_side(
        &self,
        key: ComputerKey,
        personality: EnginePersonality,
    ) -> Result<EngineHandle, EngineError> {
        let mut map = locked(&self.inner.computers);
        if let Some(existing) = map.get(&key) {
            return Ok(EngineHandle::computer(existing.clone()));
        }
        let engine = self.make(InstanceProfile::Computer(personality))?;
        map.insert(key, engine.clone());
        Ok(EngineHandle::computer(engine))
    }

    /// A handle onto the single analysis instance, behind the arbiter.
    pub fn analysis(&self, priority: AnalysisPriority) -> Result<EngineHandle, EngineError> {
        let mut slot = locked(&self.inner.analysis);
        let arbiter = match slot.as_ref() {
            Some(a) => Arc::clone(a),
            None => {
                let a = Arc::new(Arbiter::new(self.make(InstanceProfile::Analysis)?));
                *slot = Some(Arc::clone(&a));
                a
            }
        };
        Ok(EngineHandle {
            kind: HandleKind::Analysis { arbiter, priority },
        })
    }

    /// Shuts down that game's instances (game end, suspend, void, detach).
    pub async fn release_game(&self, game: GameKey) {
        let removed: Vec<UciEngine> = {
            let mut map = locked(&self.inner.computers);
            let keys: Vec<ComputerKey> = map.keys().filter(|k| k.game == game).copied().collect();
            keys.iter().filter_map(|k| map.remove(k)).collect()
        };
        shutdown_all(removed).await;
    }

    /// `quit`, grace, kill for every instance (the server's 5 s stop budget leaves room).
    pub async fn shutdown(&self) {
        let computers: Vec<UciEngine> = locked(&self.inner.computers)
            .drain()
            .map(|(_, e)| e)
            .collect();
        let analysis = locked(&self.inner.analysis).take();
        let mut all = computers;
        if let Some(a) = analysis {
            all.push(a.engine().clone());
        }
        shutdown_all(all).await;
    }

    /// Accounting hook (CR-7.4, architecture 4.8): hash megabytes configured for the
    /// instances that exist, whether or not their process is up. No ceiling is enforced.
    pub fn hash_in_use_mb(&self) -> u32 {
        let computers = locked(&self.inner.computers).len();
        let analysis = usize::from(locked(&self.inner.analysis).is_some());
        let c = u32::try_from(computers)
            .unwrap_or(u32::MAX)
            .saturating_mul(self.inner.cfg.computer_hash_mb);
        let a = u32::try_from(analysis)
            .unwrap_or(0)
            .saturating_mul(self.inner.cfg.analysis_hash_mb);
        c.saturating_add(a)
    }

    pub fn computer_count(&self) -> usize {
        locked(&self.inner.computers).len()
    }
}

#[derive(Clone)]
pub struct EngineHandle {
    kind: HandleKind,
}

#[derive(Clone)]
enum HandleKind {
    Computer(UciEngine),
    Analysis {
        arbiter: Arc<Arbiter>,
        priority: AnalysisPriority,
    },
}

impl EngineHandle {
    fn computer(e: UciEngine) -> EngineHandle {
        EngineHandle {
            kind: HandleKind::Computer(e),
        }
    }

    fn engine(&self) -> &UciEngine {
        match &self.kind {
            HandleKind::Computer(e) => e,
            HandleKind::Analysis { arbiter, .. } => arbiter.engine(),
        }
    }

    /// Starts the process and handshakes ahead of the first search, so `GameSession` can hide
    /// the start-up while the human thinks.
    pub async fn warm(&self) -> Result<(), EngineError> {
        self.engine().warm().await
    }

    pub async fn handshake_report(&self) -> Option<HandshakeReport> {
        self.engine().handshake_report().await
    }
}

impl Engine for EngineHandle {
    async fn search(&self, req: SearchRequest) -> Result<SearchResult, EngineError> {
        match &self.kind {
            HandleKind::Computer(e) => e.search(req).await,
            HandleKind::Analysis { arbiter, priority } => {
                req.limits.validate()?; // the instance checks the multipv cap
                arbiter.search(*priority, req).await
            }
        }
    }

    fn stop(&self) {
        match &self.kind {
            HandleKind::Computer(e) => e.stop(),
            HandleKind::Analysis { arbiter, priority } => arbiter.stop(*priority),
        }
    }

    fn health(&self) -> Health {
        self.engine().health()
    }
}
