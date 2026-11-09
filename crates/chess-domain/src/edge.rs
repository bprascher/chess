//! DD-DOM-002: the two edges of the pure crate. The traits live here; the system clock and
//! the OS random source live in `chess-app` (TR-12). `SplitMix64` and `error_pick_seed` are
//! here too because the roster and the shared test-vector file (FF-8) own them.

use crate::types::{GameId, MonoMs, Stamp, WallMs};

/// Monotonic time for clocks, wall time only for timestamps (architecture 5.7).
pub trait TimeSource: Send + Sync {
    fn mono(&self) -> MonoMs;
    fn wall(&self) -> WallMs;

    /// One reading of both.
    fn stamp(&self) -> Stamp {
        Stamp {
            mono: self.mono(),
            wall: self.wall(),
        }
    }
}

/// A source of random `u64` values. The caller owns the generator.
pub trait Rng {
    fn next_u64(&mut self) -> u64;

    /// A value in `0..n` by multiply-shift on 128-bit arithmetic (no modulo bias to reason
    /// about). `n == 0` returns 0 rather than panicking.
    fn below(&mut self, n: u64) -> u64 {
        let wide = u128::from(self.next_u64()) * u128::from(n);
        u64::try_from(wide >> 64).unwrap_or(0)
    }
}

/// SplitMix64 (Vigna): a 64-bit state, one addition and two xor-multiply mixes per output.
/// The constants are the published ones. `chess-engine` keeps its own copy (no
/// `engine -> domain` edge, TR-12); both are checked against the same vector file
/// (`tests/data/splitmix64_vectors.txt`, FF-8). Any change after release changes every future
/// roster and error pick, so the outputs are pinned by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX1: u64 = 0xBF58_476D_1CE4_E5B9;
const MIX2: u64 = 0x94D0_49BB_1331_11EB;

impl SplitMix64 {
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }
}

impl Rng for SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(MIX1);
        z = (z ^ (z >> 27)).wrapping_mul(MIX2);
        z ^ (z >> 31)
    }
}

/// The seed of the engine's error pick for one move (architecture 4.6, TR-12): the one
/// derivation, called by `chess-app` and passed on as `SearchRequest.seed`. The two 64-bit
/// halves of the game id and the ply are folded through three SplitMix64 steps, so a change of
/// either half or of the ply changes the result.
pub fn error_pick_seed(game_id: GameId, ply: u32) -> u64 {
    let id = game_id.as_uuid().as_u128();
    let high = u64::try_from(id >> 64).unwrap_or(0);
    let low = u64::try_from(id & u128::from(u64::MAX)).unwrap_or(0);
    let a = SplitMix64::new(high).next_u64();
    let b = SplitMix64::new(a ^ low).next_u64();
    SplitMix64::new(b ^ u64::from(ply)).next_u64()
}

#[cfg(any(test, feature = "test-support"))]
mod fakes {
    use core::sync::atomic::{AtomicU64, Ordering};

    use super::{MonoMs, Rng, TimeSource, WallMs};

    /// Time under test control (test-strategy "Determinism").
    #[derive(Debug)]
    pub struct FakeTimeSource {
        mono: AtomicU64,
        wall: AtomicU64,
    }

    impl FakeTimeSource {
        pub fn new(mono: u64, wall: u64) -> FakeTimeSource {
            FakeTimeSource {
                mono: AtomicU64::new(mono),
                wall: AtomicU64::new(wall),
            }
        }

        /// Moves both readings forward.
        pub fn advance(&self, ms: u64) {
            self.mono.fetch_add(ms, Ordering::SeqCst);
            self.wall.fetch_add(ms, Ordering::SeqCst);
        }
    }

    impl TimeSource for FakeTimeSource {
        fn mono(&self) -> MonoMs {
            MonoMs(self.mono.load(Ordering::SeqCst))
        }

        fn wall(&self) -> WallMs {
            WallMs(i64::try_from(self.wall.load(Ordering::SeqCst)).unwrap_or(i64::MAX))
        }
    }

    /// Returns the scripted values in order, then zeros.
    #[derive(Debug, Default)]
    pub struct ScriptedRng {
        values: Vec<u64>,
        next: usize,
    }

    impl ScriptedRng {
        pub fn new(values: Vec<u64>) -> ScriptedRng {
            ScriptedRng { values, next: 0 }
        }
    }

    impl Rng for ScriptedRng {
        fn next_u64(&mut self) -> u64 {
            let v = self.values.get(self.next).copied().unwrap_or(0);
            self.next = self.next.saturating_add(1);
            v
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use fakes::{FakeTimeSource, ScriptedRng};

#[cfg(test)]
mod tests {
    use super::{FakeTimeSource, Rng, ScriptedRng, TimeSource};
    use crate::types::{MonoMs, WallMs};

    #[test]
    fn ut_dom_002_fake_time_source_is_deterministic() {
        let t = FakeTimeSource::new(5, 1_000);
        assert_eq!((t.mono(), t.wall()), (MonoMs(5), WallMs(1_000)));
        t.advance(7);
        let s = t.stamp();
        assert_eq!((s.mono, s.wall), (MonoMs(12), WallMs(1_007)));
    }

    #[test]
    fn ut_dom_002_below_stays_in_range_and_is_zero_for_zero() {
        let mut r = ScriptedRng::new(vec![0, u64::MAX, u64::MAX / 2, 1 << 63]);
        assert_eq!(r.below(1), 0);
        assert_eq!(r.below(2), 1, "the top of the range maps to the top bucket");
        assert_eq!(r.below(2), 0, "just under half maps to bucket 0");
        assert_eq!(r.below(2), 1, "exactly half maps to bucket 1");
        assert_eq!(r.below(0), 0, "scripted values are exhausted and n is 0");
        assert!(r.below(u64::MAX) < u64::MAX);
    }
}
