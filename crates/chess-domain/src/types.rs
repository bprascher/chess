//! DD-DOM-001: shared value types with stable text forms (SDD 3.5).

use core::fmt;
use core::str::FromStr;

use chess_rules::Side;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{IdError, ParseTextError};

// ---------------------------------------------------------------- identity (TR-3, D-40)

macro_rules! uuid_id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(text: &str) -> Result<Self, IdError> {
                Uuid::try_parse(text).map(Self).map_err(|_| IdError)
            }
        }
    };
}

uuid_id!(
    /// A game's identity. Integer row keys of the store never appear here (D-40).
    GameId
);
uuid_id!(
    /// A Player's identity (human or computer).
    PlayerId
);

// ---------------------------------------------------------------- text enums

macro_rules! text_enum {
    (
        $(#[$m:meta])*
        $name:ident, $kind:literal {
            $($(#[$vm:meta])* $var:ident => $text:literal),+ $(,)?
        }
    ) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vm])* $var),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$var),+];

            /// The stable stored text form (SDD 3.5).
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$var => $text),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ParseTextError;

            fn from_str(text: &str) -> Result<Self, ParseTextError> {
                match text {
                    $($text => Ok($name::$var),)+
                    _ => Err(ParseTextError { kind: $kind }),
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = String::deserialize(d)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

text_enum!(
    /// Training (untimed, takeback, analysis; human against computer) or Strict (D-28).
    Mode, "mode" {
        Training => "training",
        Strict => "strict",
    }
);
text_enum!(
    /// Human or computer on each side: h-h, h-c, c-c.
    Matchup, "matchup" {
        HumanHuman => "hh",
        HumanComputer => "hc",
        ComputerComputer => "cc",
    }
);
text_enum!(
    PlayerKind, "player kind" {
        Human => "human",
        Computer => "computer",
    }
);
text_enum!(
    /// The rating scale of a game (D-28).
    Track, "track" {
        Strict => "strict",
        Training => "training",
    }
);
text_enum!(
    GameStatus, "game status" {
        InProgress => "in_progress",
        Completed => "completed",
        Aborted => "aborted",
        Voided => "voided",
        /// Store-only: never produced by the domain.
        Imported => "imported",
    }
);
text_enum!(
    GameResult, "result" {
        WhiteWins => "1-0",
        BlackWins => "0-1",
        Draw => "1/2-1/2",
    }
);
text_enum!(
    /// The computer class label and the human rating bands (ELO-1).
    Category, "category" {
        Beginner => "Beginner",
        Novice => "Novice",
        Intermediate => "Intermediate",
        Master => "Master",
        GrandMaster => "Grand Master",
    }
);
text_enum!(
    /// Why a suspend happened (SF-21). Replay does not use it to choose the resume target.
    SuspendCause, "suspend cause" {
        Detach => "detach",
        Shutdown => "shutdown",
        Recovered => "recovered",
    }
);
text_enum!(
    /// Why a draw offer was dropped (audit trail only).
    DropCause, "drop cause" {
        EngineFailure => "engine_failure",
        Suspend => "suspend",
    }
);
text_enum!(
    /// Why a game was voided; the journal strings of SF-10.
    VoidReason, "void reason" {
        Reset => "reset",
        Unreadable => "unreadable",
    }
);

impl VoidReason {
    pub fn journal_code(self) -> &'static str {
        self.as_str()
    }

    pub fn from_journal_code(code: &str) -> Result<VoidReason, ParseTextError> {
        code.parse()
    }
}

impl From<Mode> for Track {
    fn from(mode: Mode) -> Track {
        match mode {
            Mode::Training => Track::Training,
            Mode::Strict => Track::Strict,
        }
    }
}

impl GameResult {
    pub fn winner(self) -> Option<Side> {
        match self {
            GameResult::WhiteWins => Some(Side::White),
            GameResult::BlackWins => Some(Side::Black),
            GameResult::Draw => None,
        }
    }

    pub fn win_for(side: Side) -> GameResult {
        match side {
            Side::White => GameResult::WhiteWins,
            Side::Black => GameResult::BlackWins,
        }
    }
}

// ---------------------------------------------------------------- EndReason (SDD 3.5)

/// How a game ended. Display strings are `GAME.termination`; journal codes are SF-15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndReason {
    Checkmate,
    Stalemate,
    InsufficientMaterial,
    ThreefoldRepetition,
    FiftyMoveRule,
    TimeForfeit,
    TimeoutOpponentCannotMate,
    Resignation,
    DrawAgreed,
    Aborted,
    VoidedByReset,
    VoidedUnreadable,
}

impl EndReason {
    pub const ALL: &'static [EndReason] = &[
        EndReason::Checkmate,
        EndReason::Stalemate,
        EndReason::InsufficientMaterial,
        EndReason::ThreefoldRepetition,
        EndReason::FiftyMoveRule,
        EndReason::TimeForfeit,
        EndReason::TimeoutOpponentCannotMate,
        EndReason::Resignation,
        EndReason::DrawAgreed,
        EndReason::Aborted,
        EndReason::VoidedByReset,
        EndReason::VoidedUnreadable,
    ];

    /// The `GAME.termination` string.
    pub fn termination_text(self) -> &'static str {
        match self {
            EndReason::Checkmate => "checkmate",
            EndReason::Stalemate => "stalemate",
            EndReason::InsufficientMaterial => "insufficient material",
            EndReason::ThreefoldRepetition => "threefold repetition",
            EndReason::FiftyMoveRule => "fifty-move rule",
            EndReason::TimeForfeit => "time forfeit",
            EndReason::TimeoutOpponentCannotMate => "time forfeit, opponent cannot mate",
            EndReason::Resignation => "resignation",
            EndReason::DrawAgreed => "draw agreed",
            EndReason::Aborted => "aborted",
            EndReason::VoidedByReset => "voided by reset",
            EndReason::VoidedUnreadable => "voided: unreadable",
        }
    }

    pub fn from_termination_text(text: &str) -> Result<EndReason, ParseTextError> {
        EndReason::ALL
            .iter()
            .copied()
            .find(|r| r.termination_text() == text)
            .ok_or(ParseTextError {
                kind: "termination",
            })
    }

    /// The PGN `Termination` value.
    pub fn pgn_termination(self) -> &'static str {
        match self {
            EndReason::TimeForfeit => "time forfeit",
            EndReason::Aborted | EndReason::VoidedByReset | EndReason::VoidedUnreadable => {
                "abandoned"
            }
            _ => "normal",
        }
    }

    /// The `GameEnd.reason` journal code (SF-15). `None` for the three reasons that are not
    /// `GameEnd` reasons (`Abort` and `Voided` records carry none).
    pub fn journal_code(self) -> Option<&'static str> {
        match self {
            EndReason::Checkmate => Some("checkmate"),
            EndReason::Stalemate => Some("stalemate"),
            EndReason::InsufficientMaterial => Some("insufficient_material"),
            EndReason::ThreefoldRepetition => Some("threefold_repetition"),
            EndReason::FiftyMoveRule => Some("fifty_move_rule"),
            EndReason::TimeForfeit => Some("time_forfeit"),
            EndReason::TimeoutOpponentCannotMate => Some("timeout_opponent_cannot_mate"),
            EndReason::Resignation => Some("resignation"),
            EndReason::DrawAgreed => Some("draw_agreed"),
            EndReason::Aborted | EndReason::VoidedByReset | EndReason::VoidedUnreadable => None,
        }
    }

    /// Only the nine `GameEnd` codes are accepted; any other text is an error.
    pub fn from_journal_code(code: &str) -> Result<EndReason, ParseTextError> {
        EndReason::ALL
            .iter()
            .copied()
            .find(|r| r.journal_code() == Some(code))
            .ok_or(ParseTextError {
                kind: "journal reason",
            })
    }
}

/// A finished game's result and reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Outcome {
    pub result: GameResult,
    pub reason: EndReason,
}

// ---------------------------------------------------------------- seats, time, state

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    pub player: PlayerId,
    pub kind: PlayerKind,
}

/// Who plays a move: a person at the keyboard, or the engine path (D-30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Actor {
    Human,
    Engine,
}

/// Monotonic milliseconds (clocks only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MonoMs(pub u64);

/// Wall-clock milliseconds since the Unix epoch (timestamps only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WallMs(pub i64);

/// One reading of the injected `TimeSource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub mono: MonoMs,
    pub wall: WallMs,
}

/// Remaining time of both sides, whole milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockValues {
    pub white_ms: u64,
    pub black_ms: u64,
}

impl ClockValues {
    pub fn get(self, side: Side) -> u64 {
        match side {
            Side::White => self.white_ms,
            Side::Black => self.black_ms,
        }
    }
}

/// The closed set of states of architecture 4.3 as implemented (SDD 3.10). `Setup` is not a
/// state of a `Game`: `Game::new` returns a game in `AwaitingMove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    AwaitingMove,
    EngineThinking,
    EngineError,
    DrawOffered,
    Paused,
    Suspended,
    Completed,
    Finalized,
    Aborted,
    Voided,
}

impl GameState {
    pub const ALL: &'static [GameState] = &[
        GameState::AwaitingMove,
        GameState::EngineThinking,
        GameState::EngineError,
        GameState::DrawOffered,
        GameState::Paused,
        GameState::Suspended,
        GameState::Completed,
        GameState::Finalized,
        GameState::Aborted,
        GameState::Voided,
    ];

    /// The sub-states of `InProgress` in the 4.3 diagram. `Suspended` is outside it.
    pub fn is_in_progress(self) -> bool {
        matches!(
            self,
            GameState::AwaitingMove
                | GameState::EngineThinking
                | GameState::EngineError
                | GameState::DrawOffered
                | GameState::Paused
        )
    }

    /// No further play: `Completed`, `Finalized`, `Aborted`, `Voided`.
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            GameState::Completed | GameState::Finalized | GameState::Aborted | GameState::Voided
        )
    }
}
