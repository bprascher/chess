//! DD-DOM-003: `Clock`, `ClockSpec`, `ClockSync` (SDD 3.11.3, IFP-DOM-3).
//!
//! One type serves timed and untimed games (DR-DOM-004): an untimed clock keeps only
//! `turn_elapsed`, so every move has a `think_ms` (DAT-3). Time arrives as arguments; nothing
//! here reads a clock. All arithmetic saturates at zero; a backwards reading is zero elapsed.

use core::fmt;

use chess_rules::Side;
use serde::{Deserialize, Serialize};

use crate::error::ClockSpecError;
use crate::types::{ClockValues, MonoMs, WallMs};

const MAX_BASE_S: u32 = 86_400; // 24 h (TBD-34, ESTIMATE)
const MAX_INCREMENT_S: u32 = 3_600; // 1 h (TBD-34, ESTIMATE)

/// Base time and Fischer increment in whole seconds (RULE-4, TBD-34).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockSpec {
    base_s: u32,
    increment_s: u32,
}

impl ClockSpec {
    /// Base 1 s to 24 h, increment 0 to 1 h (TBD-34 engineering estimates).
    pub fn from_seconds(base_s: u32, increment_s: u32) -> Result<ClockSpec, ClockSpecError> {
        if !(1..=MAX_BASE_S).contains(&base_s) {
            return Err(ClockSpecError::BaseOutOfRange);
        }
        if increment_s > MAX_INCREMENT_S {
            return Err(ClockSpecError::IncrementOutOfRange);
        }
        Ok(ClockSpec {
            base_s,
            increment_s,
        })
    }

    /// The preset and CLI shorthand: `5+0` is five minutes plus zero seconds (Q-SE-25).
    pub fn from_minutes_plus_seconds(
        minutes: u32,
        seconds: u32,
    ) -> Result<ClockSpec, ClockSpecError> {
        let base_s = minutes
            .checked_mul(60)
            .ok_or(ClockSpecError::BaseOutOfRange)?;
        ClockSpec::from_seconds(base_s, seconds)
    }

    pub fn base_ms(self) -> u64 {
        u64::from(self.base_s) * 1000
    }

    pub fn increment_ms(self) -> u64 {
        u64::from(self.increment_s) * 1000
    }

    /// `<base>+<increment>` in whole seconds, decimal (SF-11).
    pub fn to_spec_text(self) -> String {
        format!("{}+{}", self.base_s, self.increment_s)
    }

    /// Only the canonical form: digits only, no sign, no spaces, no leading zeros (`0` alone is
    /// allowed as a number), then the limits.
    pub fn from_spec_text(text: &str) -> Result<ClockSpec, ClockSpecError> {
        let (base, inc) = text.split_once('+').ok_or(ClockSpecError::BadText)?;
        ClockSpec::from_seconds(canonical_u32(base)?, canonical_u32(inc)?)
    }
}

fn canonical_u32(text: &str) -> Result<u32, ClockSpecError> {
    // An empty string passes the digit test and fails the parse below, as BadText.
    let digits = text.bytes().all(|b| b.is_ascii_digit());
    if !digits || (text.len() > 1 && text.starts_with('0')) {
        return Err(ClockSpecError::BadText);
    }
    text.parse().map_err(|_| ClockSpecError::BadText)
}

impl fmt::Display for ClockSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_spec_text())
    }
}

impl Serialize for ClockSpec {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_spec_text())
    }
}

impl<'de> Deserialize<'de> for ClockSpec {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        ClockSpec::from_spec_text(&text).map_err(serde::de::Error::custom)
    }
}

/// What the clocks show at one instant (architecture 4.3 `ClockSync`). Remaining times are
/// `None` for an untimed game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSync {
    pub white_ms: Option<u64>,
    pub black_ms: Option<u64>,
    pub running_side: Option<Side>,
    pub as_of: WallMs,
}

/// `think_ms` of the move just made and the mover's remaining time after the increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveTiming {
    pub think_ms: u64,
    pub remaining_after: Option<u64>,
}

fn idx(side: Side) -> usize {
    match side {
        Side::White => 0,
        Side::Black => 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    spec: Option<ClockSpec>,
    remaining: [u64; 2],
    turn_side: Side,
    /// Running time charged to `turn_side` in this turn so far, summed over running intervals.
    turn_elapsed: u64,
    /// Remaining time of `turn_side` at the start of the turn (timed games).
    turn_base: u64,
    running: Option<(Side, MonoMs)>,
}

impl Clock {
    pub fn new(spec: Option<ClockSpec>) -> Clock {
        let base = spec.map_or(0, ClockSpec::base_ms);
        Clock {
            spec,
            remaining: [base; 2],
            turn_side: Side::White,
            turn_elapsed: 0,
            turn_base: base,
            running: None,
        }
    }

    pub fn spec(&self) -> Option<ClockSpec> {
        self.spec
    }

    pub fn running_side(&self) -> Option<Side> {
        self.running.map(|(side, _)| side)
    }

    pub fn turn_elapsed_ms(&self) -> u64 {
        self.turn_elapsed
    }

    fn slot(&self, side: Side) -> u64 {
        self.remaining.get(idx(side)).copied().unwrap_or(0)
    }

    fn set_slot(&mut self, side: Side, value: u64) {
        if let Some(slot) = self.remaining.get_mut(idx(side)) {
            *slot = value;
        }
    }

    /// Begins running `side`'s clock at `now`.
    pub fn start(&mut self, side: Side, now: MonoMs) {
        if side != self.turn_side {
            self.begin_turn(side);
        }
        self.running = Some((side, now));
    }

    /// Stops the running clock, charging the elapsed time to the side (no effect if stopped).
    pub fn stop(&mut self, now: MonoMs) {
        if let Some((side, since)) = self.running.take() {
            let delta = now.0.saturating_sub(since.0);
            self.turn_elapsed = self.turn_elapsed.saturating_add(delta);
            if self.spec.is_some() {
                let left = self.slot(side).saturating_sub(delta);
                self.set_slot(side, left);
            }
        }
    }

    /// Remaining time of `side` at `now`, `None` for an untimed game.
    pub fn remaining(&self, side: Side, now: MonoMs) -> Option<u64> {
        self.spec?;
        let stored = self.slot(side);
        Some(match self.running {
            Some((running, since)) if running == side => {
                stored.saturating_sub(now.0.saturating_sub(since.0))
            }
            _ => stored,
        })
    }

    /// Completes `mover`'s move: stops, reports the think time, adds the Fischer increment
    /// after the move (ASM-DOM-003), then starts the opponent. The flag check is the caller's
    /// and comes first, on `remaining` before this call.
    pub fn after_move(&mut self, mover: Side, now: MonoMs) -> MoveTiming {
        self.stop(now);
        let think_ms = self.turn_elapsed;
        let remaining_after = self.spec.map(|spec| {
            let after = self.slot(mover).saturating_add(spec.increment_ms());
            self.set_slot(mover, after);
            after
        });
        self.begin_turn(mover.other());
        self.running = Some((mover.other(), now));
        MoveTiming {
            think_ms,
            remaining_after,
        }
    }

    /// The one timer the actor arms: the running side and the instant its time runs out.
    pub fn flag_deadline(&self, now: MonoMs) -> Option<(Side, MonoMs)> {
        let (side, _) = self.running?;
        let left = self.remaining(side, now)?;
        Some((side, MonoMs(now.0.saturating_add(left))))
    }

    pub fn sync(&self, now: MonoMs, wall: WallMs) -> ClockSync {
        ClockSync {
            white_ms: self.remaining(Side::White, now),
            black_ms: self.remaining(Side::Black, now),
            running_side: self.running_side(),
            as_of: wall,
        }
    }

    /// Both remaining times at `now`, `None` for an untimed game.
    pub fn checkpoint(&self, now: MonoMs) -> Option<ClockValues> {
        Some(ClockValues {
            white_ms: self.remaining(Side::White, now)?,
            black_ms: self.remaining(Side::Black, now)?,
        })
    }

    /// Sets the remaining times from a checkpoint and leaves the clock stopped. The elapsed
    /// time of the turn in progress is rebuilt as the start-of-turn value minus the checkpoint
    /// (SDD 3.11.3), so at most one checkpoint interval is lost.
    pub fn restore(&mut self, values: ClockValues) {
        if self.spec.is_none() {
            return;
        }
        self.remaining = [values.white_ms, values.black_ms];
        self.turn_elapsed = self.turn_base.saturating_sub(self.slot(self.turn_side));
        self.running = None;
    }

    fn begin_turn(&mut self, side: Side) {
        self.turn_side = side;
        self.turn_elapsed = 0;
        self.turn_base = self.slot(side);
    }

    /// The effect of a replayed `Move` fact: no reading of time, the fact carries the result.
    pub(crate) fn apply_move(&mut self, mover: Side, clocks_after: Option<ClockValues>) {
        if self.spec.is_some()
            && let Some(values) = clocks_after
        {
            self.remaining = [values.white_ms, values.black_ms];
        }
        self.running = None;
        self.begin_turn(mover.other());
    }

    /// A new turn for `side` with nothing elapsed (takeback).
    pub(crate) fn reset_turn(&mut self, side: Side) {
        self.running = None;
        self.begin_turn(side);
    }

    /// The part of the clock that a journal records and replay rebuilds: everything except
    /// the monotonic start of the running interval.
    pub(crate) fn journaled_eq(&self, other: &Clock) -> bool {
        self.spec == other.spec && self.remaining == other.remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(s: u32) -> Option<ClockSpec> {
        ClockSpec::from_seconds(s, 0).ok()
    }

    #[test]
    fn ut_dom_014_journaled_eq_compares_spec_and_remaining_but_not_the_running_interval() {
        let a = Clock::new(spec(60));
        assert_eq!(a.spec(), spec(60));
        assert!(a.journaled_eq(&Clock::new(spec(60))));
        assert!(!a.journaled_eq(&Clock::new(spec(120))), "different spec");
        assert!(!a.journaled_eq(&Clock::new(None)), "timed against untimed");
        let mut b = Clock::new(spec(60));
        b.start(Side::White, MonoMs(0));
        b.stop(MonoMs(5_000));
        assert!(!a.journaled_eq(&b), "same spec, different remaining");
        let mut c = Clock::new(spec(60));
        c.start(Side::Black, MonoMs(77));
        assert!(a.journaled_eq(&c), "the running interval is not journaled");
        assert!(!Clock::new(None).journaled_eq(&Clock::new(spec(60))));
    }
}
