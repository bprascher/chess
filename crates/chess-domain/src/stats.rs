//! DD-DOM-018 (SDD 3.11.11): statistics as a pure fold over rows. This crate owns the counting
//! predicate and the empty-state rule (D-44, CR-4.15); the store returns rows filtered only by
//! structural keys and `aggregate` applies the predicate itself.

use std::collections::BTreeMap;

use chess_rules::Side;

use crate::error::HistoryError;
use crate::types::{Category, GameId, GameStatus, Mode, PlayerId, PlayerKind, Track, WallMs};

/// The result of a game for the viewed Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultForViewed {
    Win,
    Loss,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpponentRef {
    pub player: PlayerId,
    pub kind: PlayerKind,
    /// A computer's current label (its Strict rating's band, Q79); `None` for a human.
    pub category_now: Option<Category>,
}

/// One game, as the store's row query returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsRow {
    pub status: GameStatus,
    pub self_play: bool,
    pub custom_start: bool,
    pub mode: Mode,
    /// The colour the viewed Player had.
    pub color: Side,
    /// `None` for a game without a result (aborted, voided, unfinished).
    pub result_for_viewed: Option<ResultForViewed>,
    pub started_at: WallMs,
    pub plies: u32,
    pub active_ms: u64,
    pub opponent: OpponentRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tally {
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

impl Tally {
    /// Wins, losses and draws together. Saturates at `u32::MAX`, like the fold in `add`.
    pub fn games(&self) -> u32 {
        self.wins
            .saturating_add(self.losses)
            .saturating_add(self.draws)
    }

    fn add(&mut self, result: ResultForViewed) {
        let slot = match result {
            ResultForViewed::Win => &mut self.wins,
            ResultForViewed::Loss => &mut self.losses,
            ResultForViewed::Draw => &mut self.draws,
        };
        *slot = slot.saturating_add(1);
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerStats {
    pub overall: Tally,
    pub white: Tally,
    pub black: Tally,
    pub training: Tally,
    pub strict: Tally,
    /// Ordered by player id.
    pub by_opponent: Vec<(PlayerId, Tally)>,
    /// Computers only, in class order (Beginner first).
    pub by_category: Vec<(Category, Tally)>,
    /// Mean full moves, ceil(plies / 2) per game. `None` with no games, never 0 or NaN.
    pub avg_moves: Option<f64>,
    /// Mean `active_ms`. `None` with no games.
    pub avg_time_ms: Option<f64>,
}

impl PlayerStats {
    pub fn games(&self) -> u32 {
        self.overall.games()
    }
}

/// Completed, not self-play, not a custom start, with a result (D-44, D-64). The reset window
/// is applied by `aggregate`, which knows it.
pub fn counts_in_aggregates(row: &StatsRow) -> bool {
    row.status == GameStatus::Completed
        && !row.self_play
        && !row.custom_start
        && row.result_for_viewed.is_some()
}

/// `window = Some(reset_at)` only for the Player's own view (D-26, Q76): rows that started
/// before it are dropped. An opponent's view passes `None`.
pub fn aggregate(rows: &[StatsRow], window: Option<WallMs>) -> PlayerStats {
    let mut stats = PlayerStats::default();
    let mut by_opponent: BTreeMap<PlayerId, Tally> = BTreeMap::new();
    let mut by_category: Vec<(Category, Tally)> = Category::ALL
        .iter()
        .map(|c| (*c, Tally::default()))
        .collect();
    let mut moves: u128 = 0;
    let mut time: u128 = 0;
    let mut n: u128 = 0;
    for row in rows {
        let Some(result) = row.result_for_viewed else {
            continue;
        };
        if !counts_in_aggregates(row) || window.is_some_and(|w| row.started_at < w) {
            continue;
        }
        stats.overall.add(result);
        match row.color {
            Side::White => stats.white.add(result),
            Side::Black => stats.black.add(result),
        }
        match row.mode {
            Mode::Training => stats.training.add(result),
            Mode::Strict => stats.strict.add(result),
        }
        by_opponent
            .entry(row.opponent.player)
            .or_default()
            .add(result);
        if let Some(category) = row.opponent.category_now
            && let Some((_, tally)) = by_category.iter_mut().find(|(c, _)| *c == category)
        {
            tally.add(result);
        }
        moves += u128::from(u64::from(row.plies).div_ceil(2));
        time += u128::from(row.active_ms);
        n += 1;
    }
    stats.by_opponent = by_opponent.into_iter().collect();
    stats.by_category = by_category
        .into_iter()
        .filter(|(_, t)| t.games() > 0)
        .collect();
    if n > 0 {
        stats.avg_moves = Some(u128_to_f64(moves) / u128_to_f64(n));
        stats.avg_time_ms = Some(u128_to_f64(time) / u128_to_f64(n));
    }
    stats
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a mean of counts and milliseconds; 53 bits of mantissa are far more than the values need"
)]
fn u128_to_f64(v: u128) -> f64 {
    v as f64
}

// ---------------------------------------------------------------- rating history

/// One `RATING_HISTORY` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatingPoint {
    pub id: u64,
    pub at: WallMs,
    pub track: Track,
    /// `None` for an initial or reset point.
    pub game_id: Option<GameId>,
    pub rating_before: i32,
    pub rating_after: i32,
    pub k: i32,
}

fn ordered(points: &[RatingPoint], track: Track) -> Vec<RatingPoint> {
    let mut v: Vec<RatingPoint> = points
        .iter()
        .filter(|p| p.track == track)
        .copied()
        .collect();
    v.sort_by_key(|p| (p.at, p.id));
    v
}

/// The points of one track ordered by `(at, id)`; with `window = Some(reset_at)` from the reset
/// point on (`at >= reset_at`).
pub fn rating_series(
    points: &[RatingPoint],
    track: Track,
    window: Option<WallMs>,
) -> Vec<RatingPoint> {
    let mut v = ordered(points, track);
    if let Some(w) = window {
        v.retain(|p| p.at >= w);
    }
    v
}

/// Per track: the first row is an initial point (no game, before equals after), each row's
/// `rating_before` equals the previous row's `rating_after`, and a row without a game never
/// changes the rating (initial and reset points). The sum identity of ELO-1
/// (`final = initial + sum(after - before)`) follows from the chain, so it is not a separate
/// error (finding F-B7).
pub fn check_history(points: &[RatingPoint], track: Track) -> Result<(), HistoryError> {
    let rows = ordered(points, track);
    let mut previous: Option<&RatingPoint> = None;
    for row in &rows {
        match previous {
            None => {
                if row.game_id.is_some() || row.rating_before != row.rating_after {
                    return Err(HistoryError::FirstRowNotInitial { id: row.id });
                }
            }
            Some(prev) => {
                if row.rating_before != prev.rating_after {
                    return Err(HistoryError::Gap { id: row.id });
                }
                if row.game_id.is_none() && row.rating_before != row.rating_after {
                    return Err(HistoryError::NullGameChangesRating { id: row.id });
                }
            }
        }
        previous = Some(row);
    }
    Ok(())
}
