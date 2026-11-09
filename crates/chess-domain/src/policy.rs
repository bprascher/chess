//! DD-DOM-004, DD-DOM-005: `Policy` (the one decision table), setup validation, slot holders,
//! player actions, colour assignment, `StartPosition` (SDD 3.11.4 to 3.11.6, IFP-DOM-1).

use chess_rules::{Fen, Position, Side};
use serde::{Deserialize, Serialize};

use crate::clock::ClockSpec;
use crate::edge::Rng;
use crate::error::{Denied, DeniedReason, MoveDelayError, SetupError, SetupField, SetupReason};
use crate::types::{Actor, GameId, GameState, Matchup, Mode, PlayerId, PlayerKind, Seat};

// ---------------------------------------------------------------- StartPosition (D-64)

/// The standard start, or a custom FEN. `from_fen` is the only constructor from a FEN and it
/// normalises (SDD 3.11.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartPosition {
    Standard,
    Custom(Fen),
}

const STANDARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl StartPosition {
    /// `Standard` if all six fields, as parsed values, equal the standard start's (the `Fen`
    /// type compares its normalised text), else `Custom`. A different move counter or clock
    /// makes a start custom (Brian's choice A, 2026-11-04).
    pub fn from_fen(fen: Fen) -> StartPosition {
        if fen.as_str() == STANDARD_FEN {
            StartPosition::Standard
        } else {
            StartPosition::Custom(fen)
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, StartPosition::Custom(_))
    }

    /// `None` for the standard start: `start_fen` is null exactly then (SF-02, SF-13).
    pub fn fen(&self) -> Option<&Fen> {
        match self {
            StartPosition::Standard => None,
            StartPosition::Custom(fen) => Some(fen),
        }
    }

    pub fn position(&self) -> Position {
        match self {
            StartPosition::Standard => Position::standard(),
            StartPosition::Custom(fen) => Position::from_fen(fen),
        }
    }

    pub fn side_to_move(&self) -> Side {
        self.position().turn()
    }
}

impl Serialize for StartPosition {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.fen().map(Fen::as_str).serialize(s)
    }
}

impl<'de> Deserialize<'de> for StartPosition {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Option::<String>::deserialize(d)? {
            None => Ok(StartPosition::Standard),
            Some(text) => Fen::parse(&text)
                .map(StartPosition::from_fen)
                .map_err(|_| serde::de::Error::custom("not a valid FEN")),
        }
    }
}

// ---------------------------------------------------------------- GameSetup

/// What a new game is made of (GAME-1, PLY-1). Received in a protocol command, so unknown
/// fields are rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameSetup {
    pub game_id: GameId,
    pub matchup: Matchup,
    pub mode: Mode,
    pub white: Seat,
    pub black: Seat,
    pub start: StartPosition,
    pub clock: Option<ClockSpec>,
}

impl GameSetup {
    /// Self-play is derived, not chosen: the same Player on both sides.
    pub fn is_self_play(&self) -> bool {
        self.white.player == self.black.player
    }

    pub fn seat(&self, side: Side) -> Seat {
        match side {
            Side::White => self.white,
            Side::Black => self.black,
        }
    }
}

// ---------------------------------------------------------------- actions and context

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Takeback,
    LiveAnalysis,
    WhatIf,
    PostGameAnalysis,
    Rewind,
    SelectClock,
    OfferDraw { by: Side },
    RespondDraw { by: Side, actor: Actor },
    Pause,
    PlayResume,
    Step,
    SetMoveDelay,
    Resign { side: Side },
    Abort,
    CancelEngine,
    RetryEngine,
    MakeMove { by: Side, actor: Actor },
}

/// An action without its payload, for `allowed_actions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Takeback,
    LiveAnalysis,
    WhatIf,
    PostGameAnalysis,
    Rewind,
    SelectClock,
    OfferDraw,
    RespondDraw,
    Pause,
    PlayResume,
    Step,
    SetMoveDelay,
    Resign,
    Abort,
    CancelEngine,
    RetryEngine,
    MakeMove,
}

impl ActionKind {
    /// Declaration order: the fixed order of `allowed_actions` (stable snapshots).
    pub const ALL: &'static [ActionKind] = &[
        ActionKind::Takeback,
        ActionKind::LiveAnalysis,
        ActionKind::WhatIf,
        ActionKind::PostGameAnalysis,
        ActionKind::Rewind,
        ActionKind::SelectClock,
        ActionKind::OfferDraw,
        ActionKind::RespondDraw,
        ActionKind::Pause,
        ActionKind::PlayResume,
        ActionKind::Step,
        ActionKind::SetMoveDelay,
        ActionKind::Resign,
        ActionKind::Abort,
        ActionKind::CancelEngine,
        ActionKind::RetryEngine,
        ActionKind::MakeMove,
    ];
}

const SIDES: [Side; 2] = [Side::White, Side::Black];
const ACTORS: [Actor; 2] = [Actor::Human, Actor::Engine];

impl Action {
    pub fn kind(&self) -> ActionKind {
        match self {
            Action::Takeback => ActionKind::Takeback,
            Action::LiveAnalysis => ActionKind::LiveAnalysis,
            Action::WhatIf => ActionKind::WhatIf,
            Action::PostGameAnalysis => ActionKind::PostGameAnalysis,
            Action::Rewind => ActionKind::Rewind,
            Action::SelectClock => ActionKind::SelectClock,
            Action::OfferDraw { .. } => ActionKind::OfferDraw,
            Action::RespondDraw { .. } => ActionKind::RespondDraw,
            Action::Pause => ActionKind::Pause,
            Action::PlayResume => ActionKind::PlayResume,
            Action::Step => ActionKind::Step,
            Action::SetMoveDelay => ActionKind::SetMoveDelay,
            Action::Resign { .. } => ActionKind::Resign,
            Action::Abort => ActionKind::Abort,
            Action::CancelEngine => ActionKind::CancelEngine,
            Action::RetryEngine => ActionKind::RetryEngine,
            Action::MakeMove { .. } => ActionKind::MakeMove,
        }
    }
}

/// Every instance of a kind, over both sides and both actors (for `allowed_actions`).
fn instances(kind: ActionKind) -> Vec<Action> {
    let mut out = Vec::new();
    match kind {
        ActionKind::Takeback => out.push(Action::Takeback),
        ActionKind::LiveAnalysis => out.push(Action::LiveAnalysis),
        ActionKind::WhatIf => out.push(Action::WhatIf),
        ActionKind::PostGameAnalysis => out.push(Action::PostGameAnalysis),
        ActionKind::Rewind => out.push(Action::Rewind),
        ActionKind::SelectClock => out.push(Action::SelectClock),
        ActionKind::Pause => out.push(Action::Pause),
        ActionKind::PlayResume => out.push(Action::PlayResume),
        ActionKind::Step => out.push(Action::Step),
        ActionKind::SetMoveDelay => out.push(Action::SetMoveDelay),
        ActionKind::Abort => out.push(Action::Abort),
        ActionKind::CancelEngine => out.push(Action::CancelEngine),
        ActionKind::RetryEngine => out.push(Action::RetryEngine),
        ActionKind::OfferDraw => out.extend(SIDES.map(|by| Action::OfferDraw { by })),
        ActionKind::Resign => out.extend(SIDES.map(|side| Action::Resign { side })),
        ActionKind::RespondDraw => {
            for by in SIDES {
                out.extend(ACTORS.map(|actor| Action::RespondDraw { by, actor }));
            }
        }
        ActionKind::MakeMove => {
            for by in SIDES {
                out.extend(ACTORS.map(|actor| Action::MakeMove { by, actor }));
            }
        }
    }
    out
}

/// What `Policy` needs to know about a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameContext {
    pub mode: Mode,
    pub matchup: Matchup,
    pub state: GameState,
    /// The side to move.
    pub turn: Side,
    pub white_kind: PlayerKind,
    pub black_kind: PlayerKind,
    /// The side that has an open draw offer.
    pub pending_offer: Option<Side>,
    /// The side that may not offer again until it has moved (after a decline).
    pub offer_blocked_for: Option<Side>,
}

impl GameContext {
    pub fn kind_of(&self, side: Side) -> PlayerKind {
        match side {
            Side::White => self.white_kind,
            Side::Black => self.black_kind,
        }
    }

    fn human_to_move(&self) -> bool {
        self.kind_of(self.turn) == PlayerKind::Human
    }
}

fn deny(reason: DeniedReason) -> Result<(), Denied> {
    Err(Denied { reason })
}

fn allow_if(ok: bool, otherwise: DeniedReason) -> Result<(), Denied> {
    if ok { Ok(()) } else { deny(otherwise) }
}

/// The decision table of SDD 3.11.5. Total over its inputs: it never panics and an unknown
/// combination is a `Denied`.
pub struct Policy;

impl Policy {
    pub fn check(action: Action, ctx: &GameContext) -> Result<(), Denied> {
        let state = ctx.state;
        let analysis = matches!(action, Action::PostGameAnalysis | Action::WhatIf);
        if state == GameState::Voided && !analysis {
            return deny(DeniedReason::Voided);
        }
        if state.is_finished() && state != GameState::Voided && !analysis {
            return deny(DeniedReason::GameOver);
        }
        let strict = ctx.mode == Mode::Strict;
        let cc = ctx.matchup == Matchup::ComputerComputer;
        let hc = ctx.matchup == Matchup::HumanComputer;
        let not_allowed = DeniedReason::StateNotAllowed;
        match action {
            Action::Takeback => {
                if strict {
                    return deny(DeniedReason::StrictMode);
                }
                allow_if(
                    hc && matches!(
                        state,
                        GameState::AwaitingMove
                            | GameState::EngineThinking
                            | GameState::EngineError
                    ),
                    not_allowed,
                )
            }
            Action::LiveAnalysis => {
                if strict {
                    return deny(DeniedReason::StrictMode);
                }
                allow_if(
                    state == GameState::AwaitingMove && ctx.human_to_move(),
                    not_allowed,
                )
            }
            Action::WhatIf => {
                if state.is_finished() {
                    return Ok(());
                }
                if strict {
                    return deny(DeniedReason::StrictMode);
                }
                allow_if(state.is_in_progress() && ctx.human_to_move(), not_allowed)
            }
            Action::PostGameAnalysis => allow_if(state.is_finished(), not_allowed),
            Action::Rewind => allow_if(!cc && state.is_in_progress(), not_allowed),
            Action::SelectClock => allow_if(strict, DeniedReason::TrainingUntimed),
            Action::OfferDraw { by } => offer_draw(by, ctx),
            Action::RespondDraw { by, actor } => {
                let offeree = ctx.pending_offer.map(Side::other) == Some(by);
                let actor_ok = match actor {
                    Actor::Human => ctx.kind_of(by) == PlayerKind::Human,
                    Actor::Engine => ctx.kind_of(by) == PlayerKind::Computer,
                };
                allow_if(
                    state == GameState::DrawOffered && offeree && actor_ok,
                    not_allowed,
                )
            }
            Action::Pause => allow_if(
                cc && matches!(state, GameState::AwaitingMove | GameState::EngineThinking),
                not_allowed,
            ),
            Action::PlayResume | Action::Step => {
                allow_if(cc && state == GameState::Paused, not_allowed)
            }
            Action::SetMoveDelay => allow_if(cc, not_allowed),
            Action::Resign { side } => allow_if(
                !cc && ctx.kind_of(side) == PlayerKind::Human
                    && (state.is_in_progress() || state == GameState::Suspended),
                not_allowed,
            ),
            Action::Abort => allow_if(
                cc && (state.is_in_progress() || state == GameState::Suspended),
                not_allowed,
            ),
            Action::CancelEngine => allow_if(hc && state == GameState::EngineThinking, not_allowed),
            Action::RetryEngine => allow_if(state == GameState::EngineError, not_allowed),
            Action::MakeMove {
                by,
                actor: Actor::Human,
            } => {
                if ctx.kind_of(by) != PlayerKind::Human || by != ctx.turn {
                    return deny(DeniedReason::NotHumanTurn);
                }
                allow_if(state == GameState::AwaitingMove, not_allowed)
            }
            Action::MakeMove {
                by,
                actor: Actor::Engine,
            } => allow_if(
                ctx.kind_of(by) == PlayerKind::Computer
                    && by == ctx.turn
                    && state == GameState::EngineThinking,
                not_allowed,
            ),
        }
    }

    /// The kinds for which `check` succeeds for at least one side or actor, in declaration order.
    pub fn allowed_actions(ctx: &GameContext) -> Vec<ActionKind> {
        ActionKind::ALL
            .iter()
            .copied()
            .filter(|kind| {
                instances(*kind)
                    .into_iter()
                    .any(|action| Policy::check(action, ctx).is_ok())
            })
            .collect()
    }

    /// Setup validation, in the order mode, clock, players, start position (SDD 3.11.6). An
    /// invalid custom clock cannot reach this point: `ClockSpec` is validated when built.
    pub fn check_setup(setup: &GameSetup) -> Result<(), SetupError> {
        let fail = |field, reason| Err(SetupError { field, reason });
        if setup.mode == Mode::Training && setup.matchup != Matchup::HumanComputer {
            return fail(
                SetupField::Mode,
                SetupReason::TrainingRequiresHumanVsComputer,
            );
        }
        if setup.mode == Mode::Training && setup.clock.is_some() {
            return fail(SetupField::Clock, SetupReason::TrainingUntimed);
        }
        let kinds_ok = match setup.matchup {
            Matchup::HumanHuman => {
                setup.white.kind == PlayerKind::Human && setup.black.kind == PlayerKind::Human
            }
            Matchup::HumanComputer => setup.white.kind != setup.black.kind,
            Matchup::ComputerComputer => {
                setup.white.kind == PlayerKind::Computer && setup.black.kind == PlayerKind::Computer
            }
        };
        if !kinds_ok {
            return fail(SetupField::Players, SetupReason::PlayersWrongKind);
        }
        let position = setup.start.position();
        if position.is_checkmate() || position.is_stalemate() {
            return fail(SetupField::StartFen, SetupReason::StartPositionTerminal);
        }
        Ok(())
    }

    /// The Players that occupy an unfinished-game slot, in the game's mode (D-27). Whether a
    /// slot is already taken needs the store and is not decided here.
    pub fn slot_holders(setup: &GameSetup) -> Vec<(PlayerId, Mode)> {
        let holders: Vec<PlayerId> = match setup.matchup {
            Matchup::HumanHuman | Matchup::ComputerComputer => {
                if setup.is_self_play() {
                    vec![setup.white.player]
                } else {
                    vec![setup.white.player, setup.black.player]
                }
            }
            Matchup::HumanComputer => [setup.white, setup.black]
                .into_iter()
                .filter(|seat| seat.kind == PlayerKind::Human)
                .map(|seat| seat.player)
                .collect(),
        };
        holders.into_iter().map(|p| (p, setup.mode)).collect()
    }

    /// What may be done to a Player record (PLY-4). The computer check comes first.
    pub fn check_player_action(action: PlayerAction, ctx: &PlayerContext) -> Result<(), Denied> {
        match action {
            PlayerAction::Rename | PlayerAction::Archive | PlayerAction::Reset
                if ctx.kind == PlayerKind::Computer =>
            {
                deny(DeniedReason::ComputerPlayer)
            }
            PlayerAction::Archive if ctx.active => deny(DeniedReason::ActivePlayer),
            PlayerAction::UseAsSide if ctx.archived => deny(DeniedReason::ArchivedPlayer),
            _ => Ok(()),
        }
    }
}

fn offer_draw(by: Side, ctx: &GameContext) -> Result<(), Denied> {
    if ctx.matchup == Matchup::ComputerComputer || ctx.kind_of(by) == PlayerKind::Computer {
        return deny(DeniedReason::DrawNotAvailable);
    }
    match ctx.state {
        GameState::DrawOffered => return deny(DeniedReason::DrawOfferPending),
        GameState::EngineThinking | GameState::EngineError => {
            return deny(DeniedReason::DrawEngineBusy);
        }
        GameState::AwaitingMove => {}
        _ => return deny(DeniedReason::StateNotAllowed),
    }
    if by != ctx.turn {
        return deny(DeniedReason::DrawNotOwnTurn);
    }
    allow_if(
        ctx.offer_blocked_for != Some(by),
        DeniedReason::DrawRepeatBlocked,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    Rename,
    Archive,
    Reset,
    UseAsSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerContext {
    pub kind: PlayerKind,
    /// The Player this front-end session has selected.
    pub active: bool,
    pub archived: bool,
}

// ---------------------------------------------------------------- colours, delay

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    White,
    Black,
    Random,
}

/// Which side the first Player takes. `Random` draws one value from `rng`.
pub fn assign_colors(choice: ColorChoice, rng: &mut dyn Rng) -> Side {
    match choice {
        ColorChoice::White => Side::White,
        ColorChoice::Black => Side::Black,
        ColorChoice::Random => {
            if rng.below(2) == 0 {
                Side::White
            } else {
                Side::Black
            }
        }
    }
}

/// The pause between computer moves in a c-c game: 0 to 10 000 ms in 500 ms steps, default
/// 1000 ms (TBD-5, ESTIMATE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveDelay(u32);

impl MoveDelay {
    pub fn new(ms: u32) -> Result<MoveDelay, MoveDelayError> {
        if ms <= 10_000 && ms.is_multiple_of(500) {
            Ok(MoveDelay(ms))
        } else {
            Err(MoveDelayError)
        }
    }

    pub fn millis(self) -> u32 {
        self.0
    }
}

impl Default for MoveDelay {
    fn default() -> MoveDelay {
        MoveDelay(1000)
    }
}
