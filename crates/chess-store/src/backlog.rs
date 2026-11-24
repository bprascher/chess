//! DD-ST-002, SDD 3.12 "Apply backlog": the per-game ordered queue of batches that the store
//! thread could not apply yet, and the retry timer. **Pure scheduling**: no SQL, no thread, no
//! clock of its own. Every method takes the current monotonic time in milliseconds, so a test
//! drives the whole schedule with a number (part D3 acceptance: "a timing test with an
//! injected clock").
//!
//! Rules:
//! - Batches of one game apply strictly in the order they were queued. A batch is only ever
//!   attempted when it is the head of its game's queue.
//! - The timer: the first retry comes 0.1 s after the failed attempt; every further failed
//!   pass doubles the wait, up to 5 s (`INITIAL_DELAY_MS`, `MAX_DELAY_MS`). A pass that applies
//!   something sets the wait back to 0.1 s.
//! - A pass stops at the first transient failure (`Busy`, `Unavailable`). The database is shared
//!   by every game, so one more attempt would only wait for the same lock; with a 100 ms
//!   `busy_timeout` per attempt that bounds one pass to about 100 ms and the store thread is
//!   never held for 5 s by an apply (SDD 3.12).
//! - A batch that fails with any other error **parks** its game, on the first attempt as on a
//!   retry (D3 ruling 2026-11-24): nothing behind it is applied (order, and no `journal_seq`
//!   claim over a hole), nothing is dropped, and the game keeps the backlog non-empty so the
//!   caller sees it (`parked_games`, `sync_backlog` -> `GameParked`). Later batches of a parked
//!   game are refused by the thread, not queued. `unpark` **drops the game's held queue** and
//!   clears the park (D-145 reading, Addendum N): the caller then passes the whole journal of
//!   the game in one `apply_records` call, which is idempotent; the next start reconciles from
//!   the journal (SDD 3.11.3).
//! - The backlog holds at most `MAX_BATCHES_PER_GAME` batches per game and `MAX_PENDING_BATCHES`
//!   in all (ESTIMATE, D-145). Reaching the bound parks the game with `ParkCause::QueueFull`
//!   and the thread answers `BacklogFull` for that call.
//! - The backlog is in memory. Closing the store with batches pending loses the queue and not
//!   the data: every batch was journaled and fsynced before it was queued (SEQ-ST-02).

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use uuid::Uuid;

use crate::api::{BacklogStatus, ParkCause};

/// The first wait after a failed attempt (SDD 3.12, architecture 4.8).
pub(crate) const INITIAL_DELAY_MS: u64 = 100;
/// The longest wait between two passes.
pub(crate) const MAX_DELAY_MS: u64 = 5_000;
/// A backlog whose oldest batch is this old is reported as `StoreBacklog` (architecture 4.8).
pub(crate) const STORE_BACKLOG_AFTER_MS: u64 = 30_000;

/// The most batches one game keeps in the backlog (D-145, Addendum N: 256; ESTIMATE, derived by
/// scaling, not measured: one `Clock` batch per second per live game is about 4 minutes of a
/// stalled database). Reaching it parks the game (`ParkCause::QueueFull`); nothing queued is
/// dropped until `unpark`.
pub const MAX_BATCHES_PER_GAME: usize = 256;

/// The most batches the whole backlog holds (D-145: 4 096; ESTIMATE, same derivation). A game
/// that has a queue and reaches it is parked as above; a game with no queue gets `BacklogFull`
/// and is not parked (there is nothing of it to hold).
pub const MAX_PENDING_BATCHES: usize = 4_096;

/// The result of one attempt, as the backlog needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attempt {
    /// Applied (or already applied: the batch is idempotent).
    Done,
    /// The database is busy, locked or unavailable: try again later.
    Transient,
    /// Any other error: park the game.
    Permanent(ParkCause),
}

/// What `flush_game` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flush {
    /// The game has no waiting batch any more.
    Drained,
    /// A transient failure stopped it; batches remain.
    Transient,
    /// A batch failed for good (now or earlier): the game is parked.
    Parked,
}

struct Parked {
    cause: ParkCause,
    since_ms: u64,
}

struct Entry<T> {
    item: T,
    enqueued_ms: u64,
}

pub(crate) struct Backlog<T> {
    queues: BTreeMap<Uuid, VecDeque<Entry<T>>>,
    parked: BTreeMap<Uuid, Parked>,
    delay_ms: u64,
    next_attempt_ms: u64,
}

impl<T> Backlog<T> {
    pub(crate) fn new() -> Backlog<T> {
        Backlog {
            queues: BTreeMap::new(),
            parked: BTreeMap::new(),
            delay_ms: INITIAL_DELAY_MS,
            next_attempt_ms: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.queues.is_empty()
    }

    /// The game has batches waiting: a new batch must queue behind them (order).
    pub(crate) fn has_game(&self, game: &Uuid) -> bool {
        self.queues.contains_key(game)
    }

    /// Some game is waiting for a retry (not parked): the database is believed busy, so a new
    /// batch of another game is queued instead of costing one more attempt of its own.
    pub(crate) fn has_retrying(&self) -> bool {
        self.queues
            .keys()
            .any(|game| !self.parked.contains_key(game))
    }

    pub(crate) fn is_parked(&self, game: &Uuid) -> bool {
        self.parked.contains_key(game)
    }

    /// Batches waiting over all games.
    pub(crate) fn len_of(&self, game: &Uuid) -> usize {
        self.queues.get(game).map_or(0, VecDeque::len)
    }

    pub(crate) fn len(&self) -> usize {
        self.queues.values().map(VecDeque::len).sum()
    }

    /// The first parked game, if any (the one `sync_backlog` names).
    pub(crate) fn first_parked(&self) -> Option<Uuid> {
        self.parked.keys().next().copied()
    }

    /// Every parked game with what it failed with, how many batches wait and since when.
    pub(crate) fn parked_games(&self, now_ms: u64) -> Vec<(Uuid, ParkCause, usize, u64)> {
        self.parked
            .iter()
            .map(|(game, parked)| {
                (
                    *game,
                    parked.cause,
                    self.queues.get(game).map_or(0, VecDeque::len),
                    now_ms.saturating_sub(parked.since_ms),
                )
            })
            .collect()
    }

    /// Parks a game that has a waiting batch. Nothing behind the head is applied until
    /// `unpark`.
    pub(crate) fn park(&mut self, game: Uuid, cause: ParkCause, now_ms: u64) {
        if self.queues.contains_key(&game) {
            self.parked.insert(
                game,
                Parked {
                    cause,
                    since_ms: now_ms,
                },
            );
        }
    }

    /// Releases a parked game and **drops its held queue** (D-145 reading): the caller passes the
    /// whole journal of the game again. Returns the number of batches dropped, `None` when the
    /// game was not parked.
    pub(crate) fn unpark(&mut self, game: &Uuid) -> Option<usize> {
        self.parked.remove(game)?;
        let dropped = self.queues.remove(game).map_or(0, |queue| queue.len());
        self.reset_when_empty();
        Some(dropped)
    }

    /// Tries the game's waiting batches in order until none is left or one fails (D3-09: the
    /// store flushes a game's queue before a live terminal command for it).
    pub(crate) fn flush_game(
        &mut self,
        game: &Uuid,
        now_ms: u64,
        attempt: &mut dyn FnMut(&Uuid, &T) -> Attempt,
    ) -> Flush {
        if self.parked.contains_key(game) {
            return Flush::Parked;
        }
        while let Some(entry) = self.queues.get(game).and_then(VecDeque::front) {
            match attempt(game, &entry.item) {
                Attempt::Done => {
                    self.delay_ms = INITIAL_DELAY_MS;
                    if let Some(queue) = self.queues.get_mut(game) {
                        queue.pop_front();
                        if queue.is_empty() {
                            self.queues.remove(game);
                        }
                    }
                }
                Attempt::Permanent(cause) => {
                    self.park(*game, cause, now_ms);
                    return Flush::Parked;
                }
                Attempt::Transient => return Flush::Transient,
            }
        }
        self.reset_when_empty();
        Flush::Drained
    }

    fn reset_when_empty(&mut self) {
        if self.queues.is_empty() {
            self.parked.clear();
            self.delay_ms = INITIAL_DELAY_MS;
        }
    }

    /// Queues a batch at the end of its game's queue. The first batch of a backlog in which no
    /// game waits for a retry (empty, or only parked games: FIND-D3R-04) starts the timer: the
    /// first retry is `INITIAL_DELAY_MS` from `now`.
    pub(crate) fn push(&mut self, game: Uuid, item: T, now_ms: u64) {
        if !self.has_retrying() {
            self.delay_ms = INITIAL_DELAY_MS;
            self.next_attempt_ms = now_ms.saturating_add(INITIAL_DELAY_MS);
        }
        self.queues.entry(game).or_default().push_back(Entry {
            item,
            enqueued_ms: now_ms,
        });
    }

    /// Milliseconds until the timer wants a pass; `None` when no game is waiting for one.
    pub(crate) fn wait_ms(&self, now_ms: u64) -> Option<u64> {
        self.has_retrying()
            .then(|| self.next_attempt_ms.saturating_sub(now_ms))
    }

    /// The current wait between passes (for tests and logs).
    #[cfg(test)]
    pub(crate) fn delay_ms(&self) -> u64 {
        self.delay_ms
    }

    /// One pass over the games. Without `force` nothing happens before the timer is due; `force`
    /// (a `sync_backlog` call) tries now and leaves the timer alone when it fails. `attempt`
    /// applies one batch; `now` reads the monotonic clock (again after each attempt, because an
    /// attempt can take its 100 ms).
    pub(crate) fn pass(
        &mut self,
        force: bool,
        now: &dyn Fn() -> u64,
        attempt: &mut dyn FnMut(&Uuid, &T) -> Attempt,
    ) {
        if self.queues.is_empty() || (!force && now() < self.next_attempt_ms) {
            return;
        }
        let games: Vec<Uuid> = self
            .queues
            .keys()
            .filter(|game| !self.parked.contains_key(*game))
            .copied()
            .collect();
        for game in games {
            while let Some(entry) = self.queues.get(&game).and_then(VecDeque::front) {
                match attempt(&game, &entry.item) {
                    Attempt::Done => {
                        self.delay_ms = INITIAL_DELAY_MS;
                        if let Some(queue) = self.queues.get_mut(&game) {
                            queue.pop_front();
                            if queue.is_empty() {
                                self.queues.remove(&game);
                                break;
                            }
                        }
                    }
                    Attempt::Permanent(cause) => {
                        self.park(game, cause, now());
                        break;
                    }
                    Attempt::Transient => {
                        if !force {
                            self.delay_ms = self.delay_ms.saturating_mul(2).min(MAX_DELAY_MS);
                            self.next_attempt_ms = now().saturating_add(self.delay_ms);
                        }
                        return;
                    }
                }
            }
        }
        // A parked game is never in `games`. An empty backlog resets.
        self.reset_when_empty();
    }

    /// The numbers behind `StoreBacklog`.
    pub(crate) fn status(&self, now_ms: u64) -> BacklogStatus {
        let oldest = self
            .queues
            .values()
            .filter_map(|queue| queue.front())
            .map(|entry| now_ms.saturating_sub(entry.enqueued_ms))
            .max()
            .unwrap_or(0);
        BacklogStatus {
            pending_batches: self.len(),
            pending_games: self.queues.len(),
            parked_games: self.parked.len(),
            oldest_age: Duration::from_millis(oldest),
            store_backlog: !self.queues.is_empty() && oldest >= STORE_BACKLOG_AFTER_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn game(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    /// A backlog of `u32` batches, a clock that the test sets, and an attempt that records the
    /// time of each call and answers from a script.
    struct Rig {
        backlog: Backlog<u32>,
        now: Cell<u64>,
        calls: Vec<(u64, u128, u32)>,
    }

    impl Rig {
        fn new() -> Rig {
            Rig {
                backlog: Backlog::new(),
                now: Cell::new(0),
                calls: Vec::new(),
            }
        }

        fn pass(&mut self, force: bool, mut answer: impl FnMut(u128, u32) -> Attempt) {
            let now = &self.now;
            let calls = &mut self.calls;
            let clock = || now.get();
            self.backlog.pass(force, &clock, &mut |g, item| {
                calls.push((now.get(), g.as_u128(), *item));
                answer(g.as_u128(), *item)
            });
        }
    }

    /// The schedule of SDD 3.12: waits of 0.1, 0.2, 0.4, 0.8, 1.6, 3.2 s, then 5 s each.
    #[test]
    fn the_retry_timer_doubles_from_100_ms_to_5_s_and_stays_there() {
        let mut rig = Rig::new();
        rig.now.set(1_000);
        rig.backlog.push(game(1), 7, 1_000);
        assert_eq!(rig.backlog.wait_ms(1_000), Some(100));
        let mut attempts = Vec::new();
        for _ in 0..9 {
            let due = rig.backlog.wait_ms(rig.now.get()).unwrap();
            rig.now.set(rig.now.get() + due);
            attempts.push(rig.now.get());
            rig.pass(false, |_, _| Attempt::Transient);
        }
        let gaps: Vec<u64> = attempts.windows(2).map(|w| w[1] - w[0]).collect();
        assert_eq!(attempts[0], 1_100);
        assert_eq!(gaps, vec![200, 400, 800, 1_600, 3_200, 5_000, 5_000, 5_000]);
        assert_eq!(rig.backlog.delay_ms(), MAX_DELAY_MS);
    }

    #[test]
    fn nothing_is_attempted_before_the_timer_is_due_and_force_ignores_it() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 1, 0);
        rig.now.set(99);
        rig.pass(false, |_, _| Attempt::Done);
        assert!(rig.calls.is_empty(), "99 ms < 100 ms");
        rig.now.set(100);
        rig.pass(false, |_, _| Attempt::Done);
        assert_eq!(rig.calls.len(), 1);
        assert!(rig.backlog.is_empty());
    }

    /// A forced pass that fails leaves the schedule alone (a sync call must not stretch it).
    #[test]
    fn a_failed_forced_pass_does_not_move_the_timer() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 1, 0);
        rig.now.set(10);
        rig.pass(true, |_, _| Attempt::Transient);
        assert_eq!(rig.calls.len(), 1);
        assert_eq!(rig.backlog.wait_ms(10), Some(90));
        assert_eq!(rig.backlog.delay_ms(), INITIAL_DELAY_MS);
    }

    /// Order inside a game; independence between games; one transient failure ends the pass.
    #[test]
    fn batches_of_a_game_apply_in_order_and_a_busy_pass_makes_one_attempt() {
        let mut rig = Rig::new();
        for (g, item) in [(1, 10), (2, 20), (1, 11), (3, 30), (1, 12)] {
            rig.backlog.push(game(g), item, 0);
        }
        rig.now.set(100);
        rig.pass(false, |_, _| Attempt::Transient);
        assert_eq!(
            rig.calls,
            vec![(100, 1, 10)],
            "ten games would still cost one 100 ms attempt"
        );
        rig.calls.clear();
        rig.now.set(10_000);
        rig.pass(false, |_, _| Attempt::Done);
        let order: Vec<(u128, u32)> = rig.calls.iter().map(|c| (c.1, c.2)).collect();
        assert_eq!(order, vec![(1, 10), (1, 11), (1, 12), (2, 20), (3, 30)]);
        assert!(rig.backlog.is_empty());
    }

    #[test]
    fn a_permanent_failure_parks_its_game_and_keeps_the_others_going() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 10, 0);
        rig.backlog.push(game(1), 11, 0);
        rig.backlog.push(game(2), 20, 0);
        rig.now.set(100);
        rig.pass(false, |g, _| {
            if g == 1 {
                Attempt::Permanent(ParkCause::Other)
            } else {
                Attempt::Done
            }
        });
        assert_eq!(rig.calls.iter().filter(|c| c.1 == 1).count(), 1);
        assert!(!rig.backlog.has_game(&game(2)));
        assert!(rig.backlog.has_game(&game(1)), "nothing dropped");
        assert!(!rig.backlog.has_retrying(), "only a parked game is left");
        assert_eq!(rig.backlog.wait_ms(100), None);
        let status = rig.backlog.status(100);
        assert_eq!(
            (
                status.pending_batches,
                status.pending_games,
                status.parked_games
            ),
            (2, 1, 1)
        );
        // A later pass does not touch the parked game.
        rig.calls.clear();
        rig.pass(true, |_, _| Attempt::Done);
        assert!(rig.calls.is_empty());
    }

    #[test]
    fn store_backlog_is_reported_from_30_s_of_age_of_the_oldest_batch() {
        let mut backlog: Backlog<u32> = Backlog::new();
        assert!(
            !backlog.status(1_000_000).store_backlog,
            "empty is not a backlog"
        );
        backlog.push(game(1), 1, 1_000);
        backlog.push(game(2), 2, 20_000);
        assert!(!backlog.status(30_999).store_backlog);
        let at = backlog.status(31_000);
        assert!(at.store_backlog);
        assert_eq!(at.oldest_age, Duration::from_millis(30_000));
        assert_eq!(at.pending_batches, 2);
    }

    #[test]
    fn a_pass_that_applies_something_resets_the_delay() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 1, 0);
        rig.backlog.push(game(2), 2, 0);
        // Four busy passes: the wait grows to 1.6 s.
        for _ in 0..4 {
            let due = rig.backlog.wait_ms(rig.now.get()).unwrap();
            rig.now.set(rig.now.get() + due);
            rig.pass(false, |_, _| Attempt::Transient);
        }
        assert_eq!(rig.backlog.delay_ms(), 1_600);
        // Game 1 applies, game 2 is busy: the wait restarts from 100 and doubles once.
        let due = rig.backlog.wait_ms(rig.now.get()).unwrap();
        rig.now.set(rig.now.get() + due);
        let now = rig.now.get();
        rig.pass(false, |g, _| {
            if g == 1 {
                Attempt::Done
            } else {
                Attempt::Transient
            }
        });
        assert_eq!(rig.backlog.delay_ms(), 200);
        assert_eq!(rig.backlog.wait_ms(now), Some(200));
    }

    #[test]
    fn an_emptied_backlog_starts_its_next_life_with_the_initial_delay() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 1, 0);
        rig.now.set(100);
        rig.pass(false, |_, _| Attempt::Transient);
        rig.now.set(300);
        rig.pass(false, |_, _| Attempt::Done);
        assert!(rig.backlog.is_empty());
        rig.now.set(50_000);
        rig.backlog.push(game(1), 2, 50_000);
        assert_eq!(rig.backlog.wait_ms(50_000), Some(100));
    }

    /// FIND-D3R-04: a game that is parked does not keep the timer of an earlier life. A new
    /// batch of another game, pushed while only a parked game is left, starts at 0.1 s with the
    /// delay back at 0.1 s (it used to find the timer due at once and the delay at 5 s).
    #[test]
    fn a_new_batch_behind_only_parked_games_starts_a_fresh_timer() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 1, 0);
        for _ in 0..9 {
            let due = rig.backlog.wait_ms(rig.now.get()).unwrap();
            rig.now.set(rig.now.get() + due);
            rig.pass(false, |_, _| Attempt::Transient);
        }
        assert_eq!(rig.backlog.delay_ms(), MAX_DELAY_MS);
        rig.now.set(70_000);
        rig.pass(true, |_, _| Attempt::Permanent(ParkCause::NotFound));
        assert!(rig.backlog.is_parked(&game(1)));
        rig.now.set(72_300);
        rig.backlog.push(game(2), 2, 72_300);
        assert_eq!(rig.backlog.wait_ms(72_300), Some(100));
        assert_eq!(rig.backlog.delay_ms(), INITIAL_DELAY_MS);
        // A push while a game is retrying leaves the running timer alone.
        rig.now.set(72_350);
        rig.backlog.push(game(3), 3, 72_350);
        assert_eq!(rig.backlog.wait_ms(72_350), Some(50));
    }

    /// What a parked game reports: the cause, the batches that wait, how long ago it parked.
    /// `unpark` clears the park and **drops the held queue** (D-145), reports how many batches
    /// went, and a second `unpark` says `None`. A stale timer does not outlive it.
    #[test]
    fn a_parked_game_reports_its_cause_and_unpark_drops_its_queue() {
        let mut rig = Rig::new();
        rig.backlog.push(game(1), 10, 0);
        rig.backlog.push(game(1), 11, 0);
        rig.backlog.push(game(2), 20, 0);
        rig.now.set(500);
        rig.pass(true, |g, _| {
            if g == 1 {
                Attempt::Permanent(ParkCause::NotFound)
            } else {
                Attempt::Transient
            }
        });
        assert_eq!(
            rig.backlog.parked_games(2_500),
            vec![(game(1), ParkCause::NotFound, 2, 2_000)]
        );
        assert_eq!(rig.backlog.first_parked(), Some(game(1)));
        assert_eq!(rig.backlog.unpark(&game(1)), Some(2));
        assert_eq!(rig.backlog.unpark(&game(1)), None);
        assert_eq!(
            rig.backlog.unpark(&game(2)),
            None,
            "not parked: nothing dropped"
        );
        assert!(!rig.backlog.is_parked(&game(1)));
        assert!(!rig.backlog.has_game(&game(1)), "the held queue is gone");
        assert_eq!(rig.backlog.len(), 1, "game 2 keeps its batch");
        assert!(rig.backlog.parked_games(2_500).is_empty());
        // The last parked game leaving an otherwise empty backlog resets it.
        rig.pass(true, |_, _| Attempt::Done);
        rig.backlog.push(game(3), 30, 9_000);
        rig.pass(true, |_, _| Attempt::Permanent(ParkCause::Other));
        assert_eq!(rig.backlog.unpark(&game(3)), Some(1));
        assert!(rig.backlog.is_empty());
        assert_eq!(rig.backlog.delay_ms(), INITIAL_DELAY_MS);
    }

    /// `flush_game` applies one game's batches in order and leaves the others alone; a transient
    /// failure stops it with the rest still queued; a permanent one parks; a parked game is
    /// reported as such without an attempt.
    #[test]
    fn flush_game_drains_one_game_in_order_and_reports_what_stopped_it() {
        let mut backlog: Backlog<u32> = Backlog::new();
        for (g, item) in [(1, 10), (2, 20), (1, 11), (1, 12)] {
            backlog.push(game(g), item, 0);
        }
        let mut seen = Vec::new();
        let outcome = backlog.flush_game(&game(1), 5, &mut |_, item| {
            seen.push(*item);
            if *item == 11 {
                Attempt::Transient
            } else {
                Attempt::Done
            }
        });
        assert_eq!((outcome, seen.clone()), (Flush::Transient, vec![10, 11]));
        assert_eq!(
            backlog.len(),
            3,
            "10 is gone, 11 and 12 and game 2's 20 wait"
        );
        seen.clear();
        let outcome = backlog.flush_game(&game(1), 6, &mut |_, item| {
            seen.push(*item);
            if *item == 12 {
                Attempt::Permanent(ParkCause::Internal)
            } else {
                Attempt::Done
            }
        });
        assert_eq!((outcome, seen.clone()), (Flush::Parked, vec![11, 12]));
        assert!(backlog.is_parked(&game(1)));
        assert_eq!(backlog.parked_games(16)[0].3, 10, "parked at 6");
        seen.clear();
        let outcome = backlog.flush_game(&game(1), 7, &mut |_, item| {
            seen.push(*item);
            Attempt::Done
        });
        assert_eq!((outcome, seen.is_empty()), (Flush::Parked, true));
        let outcome = backlog.flush_game(&game(2), 8, &mut |_, _| Attempt::Done);
        assert_eq!(outcome, Flush::Drained);
        let nothing = backlog.flush_game(&game(9), 8, &mut |_, _| Attempt::Done);
        assert_eq!(nothing, Flush::Drained, "a game with no batch is drained");
        assert_eq!(backlog.len(), 1);
    }
}
