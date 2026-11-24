//! DD-EN-011: an in-process, deterministic `Engine` for tests of the gateway, the core and the
//! front-ends. Behaviour is set per instance (a default plus a one-shot queue). Every call is
//! recorded, so a test can assert "no call was made".

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use chess_rules::{Fen, Move, Position};
use tokio::sync::watch;

use crate::config::EngineConfig;
use crate::engine::Engine;
use crate::error::EngineError;
use crate::types::{
    Health, PvLine, Score, SearchLimits, SearchPosition, SearchRequest, SearchResult, UciMove,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeReply {
    Move(UciMove),
    Result(SearchResult),
    Error(EngineError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeBehavior {
    /// The smallest legal move in UCI text order, computed from the request's position.
    FirstLegalMove,
    Fixed(UciMove),
    /// One reply per search, in order; `FirstLegalMove` after the last.
    Scripted(Vec<FakeReply>),
    /// The process dies: `ProcessExited`, health `Dead`.
    Crash,
    /// Answers only after `stop()`, with `stopped_early = true`.
    Hang,
    /// `SearchTimeout`, health `Dead`.
    Timeout,
    /// A syntactically valid but (presumably) illegal move, to test the gateway's check.
    IllegalMove(UciMove),
    /// `BadOutput`, health `Dead`.
    Garbage,
    BinaryMissing,
}

/// What a call looked like (the `updates` sender is not recorded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeCall {
    Search {
        position: Box<SearchPosition>,
        limits: SearchLimits,
        multipv: u8,
        new_game: bool,
        seed: u64,
    },
    Stop,
}

#[derive(Clone)]
pub struct FakeEngine {
    inner: Arc<Inner>,
}

struct Inner {
    default: Mutex<FakeBehavior>,
    queue: Mutex<VecDeque<FakeBehavior>>,
    calls: Mutex<Vec<FakeCall>>,
    latency: Mutex<Duration>,
    health: AtomicU8,
    stops: watch::Sender<u64>,
}

fn locked<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeEngine {
    pub fn new(default: FakeBehavior) -> FakeEngine {
        let (stops, _rx) = watch::channel(0u64);
        FakeEngine {
            inner: Arc::new(Inner {
                default: Mutex::new(default),
                queue: Mutex::new(VecDeque::new()),
                calls: Mutex::new(Vec::new()),
                latency: Mutex::new(Duration::ZERO),
                health: AtomicU8::new(0),
                stops,
            }),
        }
    }

    /// The deterministic default: always the first legal move.
    pub fn first_legal() -> FakeEngine {
        FakeEngine::new(FakeBehavior::FirstLegalMove)
    }

    /// A one-shot behaviour for the next search that has none queued before it.
    pub fn push(&self, behavior: FakeBehavior) {
        locked(&self.inner.queue).push_back(behavior);
    }

    pub fn set_default(&self, behavior: FakeBehavior) {
        *locked(&self.inner.default) = behavior;
    }

    pub fn set_latency(&self, latency: Duration) {
        *locked(&self.inner.latency) = latency;
    }

    pub fn calls(&self) -> Vec<FakeCall> {
        locked(&self.inner.calls).clone()
    }

    pub fn search_count(&self) -> usize {
        locked(&self.inner.calls)
            .iter()
            .filter(|c| matches!(c, FakeCall::Search { .. }))
            .count()
    }

    fn next_behavior(&self) -> FakeBehavior {
        if let Some(b) = locked(&self.inner.queue).pop_front() {
            return b;
        }
        let mut default = locked(&self.inner.default);
        if let FakeBehavior::Scripted(replies) = &mut *default {
            if replies.is_empty() {
                return FakeBehavior::FirstLegalMove;
            }
            let reply = replies.remove(0);
            return FakeBehavior::Scripted(vec![reply]);
        }
        default.clone()
    }

    fn set_health(&self, h: Health) {
        let code = match h {
            Health::NotStarted => 0,
            Health::Ready => 1,
            Health::Searching => 2,
            Health::Dead => 3,
        };
        self.inner.health.store(code, Ordering::Relaxed);
    }
}

/// Smallest-first legal moves of the request position, as UCI text.
fn legal_moves(position: &SearchPosition) -> Result<Vec<UciMove>, EngineError> {
    let (start, moves): (Position, &[UciMove]) = match position {
        SearchPosition::Fen(fen) => (Position::from_fen(fen), &[]),
        SearchPosition::Moves { start, moves } => (
            start
                .as_ref()
                .map_or_else(Position::standard, |f: &Fen| Position::from_fen(f)),
            moves.as_slice(),
        ),
    };
    let mut pos = start;
    for m in moves {
        let mv = Move::from_uci(m.as_str()).map_err(|_| EngineError::BadOutput)?;
        pos = pos.play(mv).map_err(|_| EngineError::BadOutput)?;
    }
    let mut texts: Vec<String> = pos.legal_moves().iter().map(Move::to_uci).collect();
    texts.sort();
    texts.iter().map(|t| UciMove::parse(t)).collect()
}

fn result_from_lines(
    best: UciMove,
    candidates: &[UciMove],
    multipv: u8,
    stopped_early: bool,
) -> SearchResult {
    let lines: Vec<PvLine> = candidates
        .iter()
        .take(usize::from(multipv))
        .enumerate()
        .map(|(i, m)| {
            let n = u8::try_from(i + 1).unwrap_or(u8::MAX);
            let drop = i32::try_from(i).unwrap_or(0).saturating_mul(10);
            PvLine {
                multipv: n,
                score: Score::Cp(50 - drop),
                depth: 10,
                pv: vec![m.clone()],
            }
        })
        .collect();
    SearchResult {
        pv: vec![best.clone()],
        best,
        score: Score::Cp(50),
        depth: 10,
        lines,
        stopped_early,
        elapsed: Duration::ZERO,
    }
}

/// Puts the health back to `Ready` when a search future is dropped mid-flight, as the real
/// driver does after its drain (EF-12). A fault that already set `Dead` stays `Dead`.
struct SearchingGuard<'a>(&'a FakeEngine);

impl Drop for SearchingGuard<'_> {
    fn drop(&mut self) {
        let _ = self
            .0
            .inner
            .health
            .compare_exchange(2, 1, Ordering::Relaxed, Ordering::Relaxed);
    }
}

impl Engine for FakeEngine {
    /// Differences from `UciEngine` that a test must know about (EF-12): the fake never sleeps
    /// for `movetime`; the configured latency is capped by it. `elapsed` is the capped latency,
    /// never a clock reading, so equal requests give equal results. The `multipv` cap is the
    /// default `EngineConfig` one. `stop()` ends a latency wait or a `Hang` early, like the
    /// real driver ends a search.
    async fn search(&self, req: SearchRequest) -> Result<SearchResult, EngineError> {
        req.validate(EngineConfig::default().multipv_cap)?;
        locked(&self.inner.calls).push(FakeCall::Search {
            position: Box::new(req.position.clone()),
            limits: req.limits.clone(),
            multipv: req.multipv,
            new_game: req.new_game,
            seed: req.seed,
        });
        let mut stops = self.inner.stops.subscribe();
        let latency = (*locked(&self.inner.latency)).min(req.limits.movetime);
        self.set_health(Health::Searching);
        let _searching = SearchingGuard(self);
        let mut stopped_early = false;
        let mut elapsed = latency;
        if !latency.is_zero() {
            tokio::select! {
                () = tokio::time::sleep(latency) => {}
                _ = stops.changed() => {
                    stopped_early = true;
                    elapsed = Duration::ZERO;
                }
            }
        }
        let behavior = self.next_behavior();
        let outcome = match behavior {
            FakeBehavior::Crash => {
                self.set_health(Health::Dead);
                return Err(EngineError::ProcessExited { code: None });
            }
            FakeBehavior::Timeout => {
                self.set_health(Health::Dead);
                return Err(EngineError::SearchTimeout);
            }
            FakeBehavior::Garbage => {
                self.set_health(Health::Dead);
                return Err(EngineError::BadOutput);
            }
            FakeBehavior::BinaryMissing => {
                self.set_health(Health::NotStarted);
                return Err(EngineError::BinaryMissing);
            }
            FakeBehavior::Fixed(mv) | FakeBehavior::IllegalMove(mv) => {
                Ok(result_from_lines(mv.clone(), &[mv], req.multipv, false))
            }
            FakeBehavior::Scripted(replies) => match replies.into_iter().next() {
                Some(FakeReply::Move(mv)) => {
                    Ok(result_from_lines(mv.clone(), &[mv], req.multipv, false))
                }
                Some(FakeReply::Result(r)) => Ok(r),
                Some(FakeReply::Error(e)) => Err(e),
                None => first_legal(&req, false),
            },
            FakeBehavior::FirstLegalMove => first_legal(&req, false),
            FakeBehavior::Hang => {
                // Answers only after `stop()`. A dropped future simply ends here.
                if !stopped_early {
                    let _ = stops.changed().await;
                }
                stopped_early = true;
                elapsed = Duration::ZERO;
                first_legal(&req, true)
            }
        };
        self.set_health(Health::Ready);
        outcome.map(|mut r| {
            r.elapsed = elapsed;
            r.stopped_early |= stopped_early;
            r
        })
    }

    fn stop(&self) {
        locked(&self.inner.calls).push(FakeCall::Stop);
        self.inner.stops.send_modify(|n| *n += 1);
    }

    fn health(&self) -> Health {
        match self.inner.health.load(Ordering::Relaxed) {
            1 => Health::Ready,
            2 => Health::Searching,
            3 => Health::Dead,
            _ => Health::NotStarted,
        }
    }
}

fn first_legal(req: &SearchRequest, stopped_early: bool) -> Result<SearchResult, EngineError> {
    let legal = legal_moves(&req.position)?;
    let best = legal.first().cloned().ok_or(EngineError::BadOutput)?;
    Ok(result_from_lines(best, &legal, req.multipv, stopped_early))
}

#[cfg(test)]
mod tests {
    //! CT-EN-050: `FakeEngine` fault modes and determinism.

    use super::*;
    use crate::types::SearchLimits;
    use std::time::Instant;

    fn req(multipv: u8) -> SearchRequest {
        let mut r = SearchRequest::new(
            SearchPosition::Moves {
                start: None,
                moves: vec![],
            },
            SearchLimits::new(Duration::from_millis(50), None, None).unwrap(),
        );
        r.multipv = multipv;
        r
    }

    fn mv(t: &str) -> UciMove {
        UciMove::parse(t).unwrap()
    }

    #[tokio::test]
    async fn ct_en_050_first_legal_move_is_deterministic_and_computed_from_the_position() {
        let a = FakeEngine::first_legal();
        let b = FakeEngine::first_legal();
        for _ in 0..3 {
            let (ra, rb) = (
                a.search(req(3)).await.unwrap(),
                b.search(req(3)).await.unwrap(),
            );
            assert_eq!(ra.best.as_str(), "a2a3");
            assert_eq!(
                (
                    ra.best.clone(),
                    ra.pv.clone(),
                    ra.score,
                    ra.depth,
                    ra.lines.clone()
                ),
                (rb.best, rb.pv, rb.score, rb.depth, rb.lines)
            );
            assert_eq!(ra.lines.len(), 3);
            assert_eq!(ra.lines.first().map(|l| l.multipv), Some(1));
        }
        let mut r = req(1);
        r.position = SearchPosition::Moves {
            start: None,
            moves: vec![mv("a2a3"), mv("a7a6")],
        };
        assert_eq!(a.search(r).await.unwrap().best.as_str(), "a1a2");
        let mut r = req(1);
        r.position =
            SearchPosition::Fen(chess_rules::Fen::parse("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap());
        assert_eq!(
            a.search(r).await,
            Err(EngineError::BadOutput),
            "stalemate: no legal move"
        );
    }

    #[tokio::test]
    async fn ct_en_050_fixed_illegal_and_scripted_replies() {
        let e = FakeEngine::new(FakeBehavior::Fixed(mv("h2h4")));
        assert_eq!(e.search(req(1)).await.unwrap().best.as_str(), "h2h4");
        let e = FakeEngine::new(FakeBehavior::IllegalMove(mv("e2e5")));
        assert_eq!(
            e.search(req(1)).await.unwrap().best.as_str(),
            "e2e5",
            "the gateway must reject it"
        );
        let e = FakeEngine::new(FakeBehavior::Scripted(vec![
            FakeReply::Move(mv("b1c3")),
            FakeReply::Error(EngineError::SearchTimeout),
        ]));
        assert_eq!(e.search(req(1)).await.unwrap().best.as_str(), "b1c3");
        assert_eq!(e.search(req(1)).await, Err(EngineError::SearchTimeout));
        assert_eq!(
            e.search(req(1)).await.unwrap().best.as_str(),
            "a2a3",
            "after the script: first legal"
        );
    }

    #[tokio::test]
    async fn ct_en_050_injected_faults_set_the_health() {
        let e = FakeEngine::first_legal();
        assert_eq!(e.health(), Health::NotStarted);
        e.push(FakeBehavior::Crash);
        assert_eq!(
            e.search(req(1)).await,
            Err(EngineError::ProcessExited { code: None })
        );
        assert_eq!(e.health(), Health::Dead);
        e.push(FakeBehavior::Timeout);
        assert_eq!(e.search(req(1)).await, Err(EngineError::SearchTimeout));
        e.push(FakeBehavior::Garbage);
        assert_eq!(e.search(req(1)).await, Err(EngineError::BadOutput));
        e.push(FakeBehavior::BinaryMissing);
        assert_eq!(e.search(req(1)).await, Err(EngineError::BinaryMissing));
        assert_eq!(e.health(), Health::NotStarted);
        assert!(
            e.search(req(1)).await.is_ok(),
            "recovers on the next request, like a restart"
        );
        assert_eq!(e.health(), Health::Ready);
    }

    #[tokio::test]
    async fn ct_en_050_hang_answers_only_after_stop_and_latency_is_applied() {
        let e = FakeEngine::new(FakeBehavior::Hang);
        let e2 = e.clone();
        let t = tokio::spawn(async move { e2.search(req(1)).await });
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(!t.is_finished(), "a hung search does not answer by itself");
        e.stop();
        assert!(t.await.unwrap().unwrap().stopped_early);

        let slow = FakeEngine::first_legal();
        slow.set_latency(Duration::from_millis(40)); // below the request's movetime of 50 ms
        let started = Instant::now();
        slow.search(req(1)).await.unwrap();
        assert!(started.elapsed() >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn ef_12_health_returns_to_ready_when_a_hung_search_is_dropped() {
        let e = FakeEngine::new(FakeBehavior::Hang);
        let abandoned = tokio::time::timeout(Duration::from_millis(30), e.search(req(1))).await;
        assert!(abandoned.is_err(), "the search future was dropped");
        assert_eq!(
            e.health(),
            Health::Ready,
            "a real instance is Ready after its drain"
        );
    }

    #[tokio::test]
    async fn ef_12_stop_ends_a_latency_wait_early() {
        let e = FakeEngine::first_legal();
        e.set_latency(Duration::from_secs(30));
        let mut long = req(1);
        long.limits = SearchLimits::new(Duration::from_secs(60), None, None).unwrap();
        let e2 = e.clone();
        let t = tokio::spawn(async move { e2.search(long).await });
        tokio::time::sleep(Duration::from_millis(40)).await;
        e.stop();
        let r = tokio::time::timeout(Duration::from_secs(5), t)
            .await
            .expect("stop must cut the latency")
            .unwrap()
            .unwrap();
        assert!(r.stopped_early);
    }

    #[tokio::test]
    async fn ef_12_multipv_above_the_real_cap_is_refused() {
        let e = FakeEngine::first_legal();
        let cap = EngineConfig::default().multipv_cap;
        assert!(e.search(req(cap)).await.is_ok());
        assert!(matches!(
            e.search(req(cap + 1)).await,
            Err(EngineError::InvalidRequest { .. })
        ));
        assert_eq!(e.search_count(), 1);
    }

    #[tokio::test]
    async fn ef_12_movetime_caps_the_latency_and_elapsed_is_deterministic() {
        let e = FakeEngine::first_legal();
        e.set_latency(Duration::from_secs(3600));
        let started = std::time::Instant::now();
        let a = e.search(req(1)).await.unwrap(); // movetime 50 ms
        let b = e.search(req(1)).await.unwrap();
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(a.elapsed, Duration::from_millis(50));
        assert_eq!(a, b, "two equal requests give equal results");
    }

    #[tokio::test]
    async fn ct_en_050_calls_are_recorded_so_a_test_can_assert_no_call_was_made() {
        let e = FakeEngine::first_legal();
        assert_eq!(e.search_count(), 0);
        assert!(e.calls().is_empty());
        let mut r = req(2);
        r.seed = 99;
        r.new_game = true;
        e.search(r).await.unwrap();
        e.stop();
        let calls = e.calls();
        assert_eq!(calls.len(), 2);
        match &calls[0] {
            FakeCall::Search {
                multipv,
                new_game,
                seed,
                ..
            } => {
                assert_eq!((*multipv, *new_game, *seed), (2, true, 99));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(calls[1], FakeCall::Stop);
        assert_eq!(e.search_count(), 1);
    }

    #[tokio::test]
    async fn ct_en_050_validation_runs_before_the_call_is_recorded() {
        let e = FakeEngine::first_legal();
        let mut r = req(1);
        r.multipv = 0;
        assert!(e.search(r).await.is_err());
        assert_eq!(e.search_count(), 0);
    }
}
