//! DD-DOM-019: typed errors. Messages are English, carry no names, move lists or paths (NFR-9,
//! NFR-10). Mapping to the architecture 5.5 catalogue is `chess-app`'s job.

use chess_rules::IllegalMove;

use crate::types::GameState;

/// Text that is not one of the stored forms of an enum (`FromStr`, DD-DOM-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a recognised {kind}")]
pub struct ParseTextError {
    pub kind: &'static str,
}

/// Text that is not a UUID (`GameId`, `PlayerId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a valid identifier")]
pub struct IdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ClockSpecError {
    #[error("the base time must be 1 second to 24 hours")]
    BaseOutOfRange,
    #[error("the increment must be 0 seconds to 1 hour")]
    IncrementOutOfRange,
    #[error("the clock text must be <base seconds>+<increment seconds>, digits only")]
    BadText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EloError {
    #[error("the new rating does not fit the rating type")]
    RatingOverflow,
}

/// Why `Policy` refuses (snake-case strings, additive; SDD IFP-DOM-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeniedReason {
    StrictMode,
    TrainingUntimed,
    NotHumanTurn,
    StateNotAllowed,
    GameOver,
    Voided,
    DrawNotOwnTurn,
    DrawEngineBusy,
    DrawOfferPending,
    DrawRepeatBlocked,
    DrawNotAvailable,
    ComputerPlayer,
    ActivePlayer,
    ArchivedPlayer,
}

impl DeniedReason {
    pub const ALL: &'static [DeniedReason] = &[
        DeniedReason::StrictMode,
        DeniedReason::TrainingUntimed,
        DeniedReason::NotHumanTurn,
        DeniedReason::StateNotAllowed,
        DeniedReason::GameOver,
        DeniedReason::Voided,
        DeniedReason::DrawNotOwnTurn,
        DeniedReason::DrawEngineBusy,
        DeniedReason::DrawOfferPending,
        DeniedReason::DrawRepeatBlocked,
        DeniedReason::DrawNotAvailable,
        DeniedReason::ComputerPlayer,
        DeniedReason::ActivePlayer,
        DeniedReason::ArchivedPlayer,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            DeniedReason::StrictMode => "strict_mode",
            DeniedReason::TrainingUntimed => "training_untimed",
            DeniedReason::NotHumanTurn => "not_human_turn",
            DeniedReason::StateNotAllowed => "state_not_allowed",
            DeniedReason::GameOver => "game_over",
            DeniedReason::Voided => "voided",
            DeniedReason::DrawNotOwnTurn => "draw_not_own_turn",
            DeniedReason::DrawEngineBusy => "draw_engine_busy",
            DeniedReason::DrawOfferPending => "draw_offer_pending",
            DeniedReason::DrawRepeatBlocked => "draw_repeat_blocked",
            DeniedReason::DrawNotAvailable => "draw_not_available",
            DeniedReason::ComputerPlayer => "computer_player",
            DeniedReason::ActivePlayer => "active_player",
            DeniedReason::ArchivedPlayer => "archived_player",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not allowed: {}", .reason.as_str())]
pub struct Denied {
    pub reason: DeniedReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupField {
    Mode,
    Clock,
    Players,
    StartFen,
}

impl SetupField {
    pub fn as_str(self) -> &'static str {
        match self {
            SetupField::Mode => "mode",
            SetupField::Clock => "clock",
            SetupField::Players => "players",
            SetupField::StartFen => "start_fen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupReason {
    TrainingRequiresHumanVsComputer,
    TrainingUntimed,
    PlayersWrongKind,
    StartPositionTerminal,
}

impl SetupReason {
    pub fn as_str(self) -> &'static str {
        match self {
            SetupReason::TrainingRequiresHumanVsComputer => "training_requires_human_vs_computer",
            SetupReason::TrainingUntimed => "training_untimed",
            SetupReason::PlayersWrongKind => "players_wrong_kind",
            SetupReason::StartPositionTerminal => "start_position_terminal",
        }
    }
}

/// `check_setup` failure (`ValidationFailed{field, reason}` in the app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid setup: {} ({})", .field.as_str(), .reason.as_str())]
pub struct SetupError {
    pub field: SetupField,
    pub reason: SetupReason,
}

/// Why `Game::decide` refuses a command. Every refusal leaves the game unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GameError {
    #[error(transparent)]
    Denied(#[from] Denied),
    #[error("the position has moved on (live ply {live_ply})")]
    StalePosition { live_ply: u32 },
    #[error(transparent)]
    IllegalMove(#[from] IllegalMove),
    #[error("there is nothing to take back")]
    NothingToTakeBack,
    #[error("the running clock has not reached zero")]
    FlagNotFallen,
    #[error("the game has reached the ply limit")]
    TooLong,
    #[error("internal inconsistency in the game state")]
    InvalidTransition,
    #[error(transparent)]
    Setup(#[from] SetupError),
}

/// Why `Game::evolve` or `Game::replay` stops at a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReplayError {
    #[error("the first fact is not a start")]
    NotStarted,
    #[error("a second start fact")]
    SecondStart,
    #[error("the recorded setup is not valid")]
    BadSetup(#[from] SetupError),
    #[error("the fact does not fit the state {state:?}")]
    WrongState { state: GameState },
    #[error("a suspended game was not resumed before this fact")]
    NotResumed,
    #[error("the move has ply {found}, expected {expected}")]
    WrongPly { expected: u32, found: u32 },
    #[error("the move is not legal in the replayed position")]
    Illegal(#[from] IllegalMove),
    #[error("the position after ply {ply} differs from the recorded one")]
    FenMismatch { ply: u32 },
    #[error("the takeback target is beyond the moves played")]
    BadTakeback,
    #[error("the game has reached the ply limit")]
    TooLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the move delay must be 0 to 10000 ms in steps of 500")]
pub struct MoveDelayError;

// ---------------------------------------------------------------- part B (chess-a0r.2.7)

/// Roster generation failures (DD-DOM-013). The app maps `CountOutOfRange` to
/// `ValidationFailed{field: count}` and the rest to `Internal{id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RosterError {
    #[error("the number of computer players must be 1 to 25")]
    CountOutOfRange,
    #[error("no unused parameter set is left for this roster position")]
    ParameterSpaceExhausted,
    #[error("the class bounds are not valid")]
    BoundsInvalid,
}

/// `Personality` constructor rejections (DD-DOM-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PersonalityError {
    #[error("the error rate must be 0 to 1000 per mille")]
    ErrorPermilleOutOfRange,
    #[error("a player with an error rate needs a MultiPV of at least 2")]
    ErrorMultipvTooLow,
}

/// `VariationTree` failures (DD-DOM-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    #[error(transparent)]
    IllegalMove(#[from] IllegalMove),
    #[error("the variation tree is full")]
    Full,
    #[error("no such position in the variation tree")]
    NotFound,
    #[error("the starting position cannot be deleted")]
    DeleteRoot,
    #[error("a branch from the final position cannot be saved")]
    AtFinalPosition,
    #[error("there is no move to save")]
    EmptyLine,
    /// Line text that cannot be read, or a tree that cannot be written as line text: invalid
    /// text, or variations nested deeper than the `chess-rules` limit (64).
    #[error("the line text is not valid")]
    BadText,
}

/// `check_history` failures: an integration check, not shown to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HistoryError {
    #[error("the rating history has a gap at row {id}")]
    Gap { id: u64 },
    #[error("the first rating history row {id} is not an initial point")]
    FirstRowNotInitial { id: u64 },
    #[error("row {id} has no game but changes the rating")]
    NullGameChangesRating { id: u64 },
}

/// `BoardGrid::from_fen` failures. A front-end passes the snapshot's own text, so a failure is
/// a defect (`Internal{id}` in the app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BoardError {
    #[error("the position text is not a valid FEN")]
    BadFen,
    #[error("the last move text is not a valid move")]
    BadLastMove,
}

/// `resolve` was asked about a mismatch without a choice: the caller must ask the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the opening code in the file differs from the position's; a choice is needed")]
pub struct EcoUnanswered;
