//! DD-EN-014: every timeout and size in one place so tests can shrink them (architecture 4.8).
//! Numbers marked ESTIMATE are the placeholders of SDD 3.11 and architecture 4.7 (AMD-12).

use std::time::Duration;

use crate::error::EngineError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    /// `uciok` and `readyok` each (ENG-4: 5 s).
    pub handshake_timeout: Duration,
    /// Soft stop after the movetime plus this (SDD 3.10.2: 500 ms, ESTIMATE).
    pub soft_stop_grace: Duration,
    /// Hard kill after the movetime plus this (ENG-4: 2 s).
    pub hard_kill_grace: Duration,
    /// Bound on waiting for the abandoned `bestmove` while draining (2 s).
    pub drain_timeout: Duration,
    /// `quit`, then this, then kill (1 s, ESTIMATE, the SDD author's value).
    pub shutdown_grace: Duration,
    /// Longest accepted engine output line (64 KiB, ESTIMATE).
    pub line_cap: usize,
    /// Capacity of the per-search updates channel is the caller's; this is the throttle.
    pub update_interval: Duration,
    /// Highest `multipv` a request may ask for (ESTIMATE, choice F-E6).
    pub multipv_cap: u8,
    /// Computer instances (ESTIMATE, AMD-12 placeholders).
    pub computer_threads: u32,
    pub computer_hash_mb: u32,
    /// The shared analysis instance (ESTIMATE, AMD-12 placeholders).
    pub analysis_threads: u32,
    pub analysis_hash_mb: u32,
    /// Text a bundled binary's `id name` must contain (TBC-7 confirmed it by a live run).
    pub pinned_id_text: String,
}

impl Default for EngineConfig {
    fn default() -> EngineConfig {
        EngineConfig {
            handshake_timeout: Duration::from_secs(5),
            soft_stop_grace: Duration::from_millis(500),
            hard_kill_grace: Duration::from_secs(2),
            drain_timeout: Duration::from_secs(2),
            shutdown_grace: Duration::from_secs(1),
            line_cap: 64 * 1024,
            update_interval: Duration::from_millis(100),
            multipv_cap: 8,
            computer_threads: 1,
            computer_hash_mb: 16,
            analysis_threads: 2,
            analysis_hash_mb: 64,
            pinned_id_text: "Stockfish 19".to_string(),
        }
    }
}

impl EngineConfig {
    /// Longest accepted timeout or grace (ESTIMATE, EF-02): one hour. Larger values would
    /// overflow `Instant` arithmetic in the driver.
    pub const MAX_DURATION: Duration = Duration::from_secs(60 * 60);

    /// Range checks (UT-EN-050). The soft deadline must come before the hard one.
    pub fn validate(&self) -> Result<(), EngineError> {
        let bad = |field| Err(EngineError::InvalidConfig { field });
        for (field, d) in [
            ("handshake_timeout", self.handshake_timeout),
            ("soft_stop_grace", self.soft_stop_grace),
            ("hard_kill_grace", self.hard_kill_grace),
            ("drain_timeout", self.drain_timeout),
            ("shutdown_grace", self.shutdown_grace),
            ("update_interval", self.update_interval),
        ] {
            if d > Self::MAX_DURATION {
                return bad(field);
            }
        }
        if self.handshake_timeout.is_zero() {
            return bad("handshake_timeout");
        }
        if self.soft_stop_grace >= self.hard_kill_grace {
            return bad("soft_stop_grace");
        }
        if self.drain_timeout.is_zero() {
            return bad("drain_timeout");
        }
        if self.line_cap < 256 {
            return bad("line_cap");
        }
        if self.multipv_cap == 0 {
            return bad("multipv_cap");
        }
        if self.computer_threads == 0 || self.analysis_threads == 0 {
            return bad("threads");
        }
        if self.computer_hash_mb == 0 || self.analysis_hash_mb == 0 {
            return bad("hash_mb");
        }
        if self.pinned_id_text.is_empty() {
            return bad("pinned_id_text");
        }
        Ok(())
    }
}
