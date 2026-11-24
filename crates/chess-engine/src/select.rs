//! DD-EN-009: search limits, the clock budget, SplitMix64 and the seeded error pick.
//! Pure apart from the one `search` call in `select_move`.

use std::time::Duration;

use crate::engine::Engine;
use crate::error::EngineError;
use crate::types::{EnginePersonality, SearchLimits, SearchRequest, SearchResult, UciMove};

/// Vigna's SplitMix64 with the published constants. A copy of the one in `chess-domain`
/// (no `engine -> domain` edge, TR-12, DR-EN-006); both are checked against the same vector
/// file (FF-8). Changing it changes every replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)` from the top 53 bits (choice F-E7: the SDD names `next_f64` but
    /// not its definition).
    pub fn next_f64(&mut self) -> f64 {
        // 53 bits fit a f64 mantissa exactly.
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

impl SearchLimits {
    /// `movetime = min(personality, max_think, clock_budget)`; never above any of them (ENG-3).
    /// `depth_limit == 0` means no depth cap (choice F-E8). A zero result is an error: the
    /// caller has no time left to think.
    pub fn for_move(
        personality: &EnginePersonality,
        max_think: Option<Duration>,
        clock_budget: Option<Duration>,
    ) -> Result<SearchLimits, EngineError> {
        let mut movetime = Duration::from_millis(u64::from(personality.movetime_ms));
        for cap in [max_think, clock_budget].into_iter().flatten() {
            movetime = movetime.min(cap);
        }
        let depth = (personality.depth_limit > 0).then_some(personality.depth_limit);
        SearchLimits::new(movetime, depth, None)
    }
}

/// `min(remaining / 2, max(50 ms, remaining / 30 + 3 * increment / 4))`, the cap applied last
/// (Q-SE-10). An upper limit, never a target. All constants are ESTIMATE (UR-14). `_ply` is
/// part of the SDD signature; the formula does not use it.
pub fn clock_budget(remaining: Duration, increment: Duration, _ply: u32) -> Duration {
    let share = (remaining / 30).saturating_add(increment.saturating_mul(3) / 4);
    let wanted = share.max(Duration::from_millis(50));
    wanted.min(remaining / 2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Best,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveChoice {
    pub mv: UciMove,
    pub kind: MoveKind,
    pub result: SearchResult,
}

/// F-3.4: the seeded random-error choice (SDD 3.10.8). Same seed, same roll, same pick. The
/// legality check of the move is the gateway's.
pub async fn select_move<E: Engine>(
    engine: &E,
    mut req: SearchRequest,
    personality: &EnginePersonality,
) -> Result<MoveChoice, EngineError> {
    let mut draws = SplitMix64::new(req.seed);
    let u = draws.next_f64();
    let error_roll = personality.error_rate > 0.0
        && personality.error_multipv >= 2
        && u < personality.error_rate;
    if error_roll {
        req.multipv = personality.error_multipv;
        let result = engine.search(req).await?;
        let count = u64::try_from(result.lines.len()).unwrap_or(0);
        if count >= 2 {
            let i = 1 + draws.next_u64() % (count - 1);
            let picked = usize::try_from(i)
                .ok()
                .and_then(|i| result.lines.get(i))
                .and_then(|line| line.pv.first())
                .cloned();
            if let Some(mv) = picked {
                return Ok(MoveChoice {
                    mv,
                    kind: MoveKind::Error,
                    result,
                });
            }
        }
        // One legal move, or stopped very early: the best move.
        let mv = result.best.clone();
        Ok(MoveChoice {
            mv,
            kind: MoveKind::Best,
            result,
        })
    } else {
        req.multipv = 1;
        let result = engine.search(req).await?;
        let mv = result.best.clone();
        Ok(MoveChoice {
            mv,
            kind: MoveKind::Best,
            result,
        })
    }
}
