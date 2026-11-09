//! DD-DOM-011, DD-DOM-012: Elo, rating eligibility and category bands (SDD 3.11.7, IFP-DOM-4).
//! Whole-integer ratings; `f64` only for the expected score; the change is rounded, not the
//! rating (ELO-1).

use crate::error::EloError;
use crate::types::{Category, GameResult, GameStatus, Mode, Track};

/// Rounds half away from zero (`f64::round`, ELO-1).
pub fn round_half_away(x: f64) -> f64 {
    x.round()
}

/// One side's rating and rated-game counter on the game's track, from before the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatedSide {
    pub rating: i32,
    pub rated_games_before: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideUpdate {
    pub before: i32,
    pub after: i32,
    pub delta: i32,
    pub k: i32,
    pub counter_after: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settlement {
    pub white: SideUpdate,
    pub black: SideUpdate,
}

/// The facts `rating_track` decides on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameFacts {
    pub status: GameStatus,
    pub mode: Mode,
    pub self_play: bool,
    pub custom_start: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnratedReason {
    SelfPlay,
    CustomStart,
    Imported,
    Aborted,
    Voided,
    NotCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingTrack {
    Rated(Track),
    Unrated(UnratedReason),
}

/// The Elo calculator (F-4.1, F-4.2).
pub struct EloCalc;

impl EloCalc {
    /// `1 / (1 + 10^((rb - ra) / 400))`, in (0, 1).
    pub fn expected_score(ra: i32, rb: i32) -> f64 {
        let diff = f64::from(rb) - f64::from(ra);
        1.0 / (1.0 + 10f64.powf(diff / 400.0))
    }

    /// 40 below 30 rated games on the track, else 32.
    pub fn k_factor(rated_games_on_track_before: u32) -> i32 {
        if rated_games_on_track_before < 30 {
            40
        } else {
            32
        }
    }

    /// Settles one game on one track. Each side uses its own K and counter; the ratings are
    /// the ones from before the game. No clamp.
    pub fn settle(
        white: RatedSide,
        black: RatedSide,
        result: GameResult,
    ) -> Result<Settlement, EloError> {
        let (sw, sb) = match result {
            GameResult::WhiteWins => (1.0, 0.0),
            GameResult::Draw => (0.5, 0.5),
            GameResult::BlackWins => (0.0, 1.0),
        };
        Ok(Settlement {
            white: update(white, black.rating, sw)?,
            black: update(black, white.rating, sb)?,
        })
    }

    /// Which track a game counts on, or why it counts on none (F-4.1, ELO-2, RULE-8).
    /// Precedence: self-play, custom start, then the status.
    pub fn rating_track(facts: &GameFacts) -> RatingTrack {
        if facts.self_play {
            return RatingTrack::Unrated(UnratedReason::SelfPlay);
        }
        if facts.custom_start {
            return RatingTrack::Unrated(UnratedReason::CustomStart);
        }
        match facts.status {
            GameStatus::Completed => RatingTrack::Rated(Track::from(facts.mode)),
            GameStatus::Imported => RatingTrack::Unrated(UnratedReason::Imported),
            GameStatus::Aborted => RatingTrack::Unrated(UnratedReason::Aborted),
            GameStatus::Voided => RatingTrack::Unrated(UnratedReason::Voided),
            GameStatus::InProgress => RatingTrack::Unrated(UnratedReason::NotCompleted),
        }
    }
}

fn update(side: RatedSide, opponent_rating: i32, score: f64) -> Result<SideUpdate, EloError> {
    let k = EloCalc::k_factor(side.rated_games_before);
    let expected = EloCalc::expected_score(side.rating, opponent_rating);
    let delta = to_i32(round_half_away(f64::from(k) * (score - expected)))?;
    let after = side
        .rating
        .checked_add(delta)
        .ok_or(EloError::RatingOverflow)?;
    Ok(SideUpdate {
        before: side.rating,
        after,
        delta,
        k,
        counter_after: side.rated_games_before.saturating_add(1),
    })
}

/// `|delta| <= K <= 40` because `S - E` lies in [-1, 1]; the range is checked first so the
/// conversion cannot truncate (a value outside it would be reported as an overflow).
fn to_i32(x: f64) -> Result<i32, EloError> {
    if !(-1.0e9..=1.0e9).contains(&x) {
        return Err(EloError::RatingOverflow);
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "SDD 3.11.7: the value is an already-rounded f64 inside +-1e9, checked above, so it fits i32"
    )]
    let narrowed = x as i32;
    Ok(narrowed)
}

impl Category {
    /// The band of a Strict rating (ELO-1). A computer's label uses its Strict rating only
    /// (F-4.4, Q79): the parameter is the Strict rating by name of this function.
    pub fn from_rating(strict_rating: i32) -> Category {
        match strict_rating {
            i32::MIN..=599 => Category::Beginner,
            600..=1199 => Category::Novice,
            1200..=2199 => Category::Intermediate,
            2200..=2499 => Category::Master,
            2500..=i32::MAX => Category::GrandMaster,
        }
    }

    /// The starting rating of the class on both tracks (ELO-1, PLY-2).
    pub fn baseline(self) -> i32 {
        match self {
            Category::Beginner => 400,
            Category::Novice => 600,
            Category::Intermediate => 1200,
            Category::Master => 2200,
            Category::GrandMaster => 2500,
        }
    }
}
