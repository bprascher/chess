//! DD-DOM-006 to DD-DOM-010: the `Game` aggregate. `decide` checks a command (policy, ply,
//! legality, clocks) and returns facts; `evolve` is the only code that changes the journaled
//! state and never reads a clock, so live play and journal replay run the same code
//! (DR-DOM-001, R-15).
//!
//! Exceptions, all stated in the SDD: the transient states (`EngineThinking`, `EngineError`,
//! the `stepping` flag; transitions T2, T4, T5, T12) have no fact, and neither has `Finalized`
//! (T19, a store event). `decide` sets those directly. Replay never sees them, which is why
//! `journal_eq` compares the persistent state.

use chess_rules::{Fen, Move, Position, PositionKey, Side};

use crate::clock::{Clock, ClockSync};
use crate::eco::{EcoMatch, EcoTable};
use crate::error::{Denied, DeniedReason, GameError, ReplayError, SetupError};
use crate::policy::{Action, ActionKind, GameContext, GameSetup, Policy};
use crate::types::{
    Actor, ClockValues, DropCause, EndReason, GameResult, GameState, Matchup, Outcome, PlayerId,
    PlayerKind, Stamp, SuspendCause, VoidReason, WallMs,
};

/// A guard that should be unreachable: the fifty-move and repetition rules end games earlier.
pub const MAX_PLIES: u32 = 10_000;

// ---------------------------------------------------------------- Fact

/// A domain event that is also one journal record kind (SDD 3.11.1, architecture 7.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    Start {
        setup: GameSetup,
        started_at: WallMs,
    },
    /// `ply` is the zero-based index of the move (plies already played). `played_at` is the
    /// record's `wall_ms`. A missing `think_ms` counts as 0 in `TimeTotals::seed_from_facts`.
    Move {
        ply: u32,
        mv: Move,
        think_ms: Option<u64>,
        clocks_after: Option<ClockValues>,
        played_at: WallMs,
    },
    Takeback {
        to_ply: u32,
    },
    Clock {
        clocks: Option<ClockValues>,
        running: Option<Side>,
    },
    /// `by` is the side that made the offer.
    DrawOffer {
        by: Side,
    },
    /// `by` is the side whose offer was declined.
    DrawDeclined {
        by: Side,
    },
    /// `by` is the side whose offer was dropped.
    DrawDropped {
        by: Side,
        cause: DropCause,
    },
    Suspend {
        cause: SuspendCause,
    },
    Pause,
    PlayResume,
    SuspendResume,
    Ended {
        outcome: Outcome,
    },
    Aborted,
    Voided {
        reason: VoidReason,
        reset_player: Option<PlayerId>,
    },
}

impl Fact {
    /// The journal record kind this fact is written as (architecture 7.6, SF-07).
    pub fn journal_kind(&self) -> &'static str {
        match self {
            Fact::Start { .. } => "Start",
            Fact::Move { .. } => "Move",
            Fact::Takeback { .. } => "Takeback",
            Fact::Clock { .. } => "Clock",
            Fact::DrawOffer { .. } => "DrawOffer",
            Fact::DrawDeclined { .. } => "DrawDeclined",
            Fact::DrawDropped { .. } => "DrawDropped",
            Fact::Suspend { .. } => "Suspend",
            Fact::Pause => "Pause",
            Fact::PlayResume => "Resume",
            Fact::SuspendResume => "SuspendResume",
            Fact::Ended { .. } => "GameEnd",
            Fact::Aborted => "Abort",
            Fact::Voided { .. } => "Voided",
        }
    }
}

// ---------------------------------------------------------------- commands, steps, records

/// The input of `Game::decide` (renamed from `Command`, TR-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    Move {
        mv: Move,
        expected_ply: u32,
        actor: Actor,
    },
    Takeback,
    OfferDraw {
        by: Side,
    },
    /// The offeree answers.
    RespondDraw {
        accept: bool,
    },
    DropDrawOffer,
    Resign {
        side: Side,
    },
    Flag,
    Pause,
    PlayResume,
    Step,
    Suspend {
        cause: SuspendCause,
    },
    SuspendResume,
    Abort,
    /// The SDD sketch has `Void{reason}`; the fact also holds the reset Player, which only the
    /// caller knows (GATE-103 finding F-D5).
    Void {
        reason: VoidReason,
        reset_player: Option<PlayerId>,
    },
    ResolveTerminal,
    EngineStarted,
    EngineFailed,
    EngineRetry,
    Checkpoint,
    MarkFinalized,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub game: Game,
    pub facts: Vec<Fact>,
    pub ended: Option<Outcome>,
}

/// One move of the game as the snapshot shows it (`MOVE` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlyRecord {
    pub ply: u32,
    pub mv: Move,
    pub san: String,
    pub fen_after: String,
    pub think_ms: u64,
    pub clocks_after: Option<ClockValues>,
    pub played_at: WallMs,
}

impl PlyRecord {
    pub fn uci(&self) -> String {
        self.mv.to_uci()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameView {
    pub state: GameState,
    pub ply: u32,
    pub turn: Side,
    pub plies: Vec<PlyRecord>,
    pub clocks: ClockSync,
    pub allowed_actions: Vec<ActionKind>,
    pub outcome: Option<Outcome>,
    pub eco: Option<EcoMatch>,
}

// ---------------------------------------------------------------- replay types (D-72, D-79)

/// The record's `san` and `fen_after`, supplied by the codec for a `Move` only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveCheck {
    pub san: String,
    pub fen_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayItem {
    pub fact: Fact,
    pub check: Option<MoveCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayWarning {
    SanMismatch {
        ply: u32,
        recorded: String,
        computed: String,
    },
    DrawDroppedBy {
        recorded: Side,
        pending: Side,
    },
}

#[derive(Debug, Clone)]
pub struct ReplayOutcome {
    pub game: Option<Game>,
    pub applied: usize,
    /// The facts end in a non-terminal state that is not `Suspended`, and the replayed position
    /// is not terminal (a crash or kill).
    pub interrupted: bool,
    /// The replayed position is terminal but no `Ended` fact followed (D-65).
    pub terminal: Option<Outcome>,
    pub warnings: Vec<ReplayWarning>,
    pub error: Option<ReplayError>,
}

// ---------------------------------------------------------------- Game

#[derive(Debug, Clone)]
pub struct Game {
    setup: GameSetup,
    state: GameState,
    resume_target: Option<GameState>,
    /// A c-c `Step` is running: the move ends in `Paused`.
    stepping: bool,
    start_position: Position,
    /// `positions[n - 1]` is the position after ply `n`.
    positions: Vec<Position>,
    keys: Vec<PositionKey>,
    plies: Vec<PlyRecord>,
    clock: Clock,
    pending_offer: Option<Side>,
    offer_blocked_for: Option<Side>,
    outcome: Option<Outcome>,
    eco: Option<EcoMatch>,
}

fn ply_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The termination order of 3.11.2: checkmate, stalemate, dead position, threefold, fifty-move.
fn termination_of(position: &Position, keys: &[PositionKey]) -> Option<Outcome> {
    let draw = |reason| {
        Some(Outcome {
            result: GameResult::Draw,
            reason,
        })
    };
    if position.is_checkmate() {
        return Some(Outcome {
            result: GameResult::win_for(position.turn().other()),
            reason: EndReason::Checkmate,
        });
    }
    if position.is_stalemate() {
        return draw(EndReason::Stalemate);
    }
    if position.is_dead_by_material() {
        return draw(EndReason::InsufficientMaterial);
    }
    if let Some(last) = keys.last()
        && keys.iter().filter(|k| *k == last).count() >= 3
    {
        return draw(EndReason::ThreefoldRepetition);
    }
    if position.halfmove_clock() >= 100 {
        return draw(EndReason::FiftyMoveRule);
    }
    None
}

fn flag_outcome(flagged: Side, position: &Position) -> Outcome {
    let opponent = flagged.other();
    if position.can_side_checkmate(opponent) {
        Outcome {
            result: GameResult::win_for(opponent),
            reason: EndReason::TimeForfeit,
        }
    } else {
        Outcome {
            result: GameResult::Draw,
            reason: EndReason::TimeoutOpponentCannotMate,
        }
    }
}

fn denied(reason: DeniedReason) -> GameError {
    GameError::Denied(Denied { reason })
}

fn same_position(a: &Position, b: &Position) -> bool {
    a.repetition_key() == b.repetition_key()
        && a.halfmove_clock() == b.halfmove_clock()
        && a.fullmove_number() == b.fullmove_number()
}

impl Game {
    // ------------------------------------------------------------ construction

    /// A new game in `AwaitingMove` with the `Start` fact; White's clock starts (T1, ASM-DOM-001).
    #[allow(
        clippy::new_ret_no_self,
        reason = "SDD IFP-DOM-2 names this constructor Game::new and it returns the Step that carries the Start fact"
    )]
    pub fn new(setup: GameSetup, stamp: Stamp) -> Result<Step, SetupError> {
        Policy::check_setup(&setup)?;
        let mut game = Game::from_start(setup.clone());
        let first = game.turn();
        game.clock.start(first, stamp.mono);
        let fact = Fact::Start {
            setup,
            started_at: stamp.wall,
        };
        Ok(Step {
            game,
            facts: vec![fact],
            ended: None,
        })
    }

    fn from_start(setup: GameSetup) -> Game {
        let position = setup.start.position();
        let key = position.repetition_key();
        let eco = EcoTable::observe(None, &key, 0);
        let clock = Clock::new(setup.clock);
        Game {
            setup,
            state: GameState::AwaitingMove,
            resume_target: None,
            stepping: false,
            start_position: position,
            positions: Vec::new(),
            keys: vec![key],
            plies: Vec::new(),
            clock,
            pending_offer: None,
            offer_blocked_for: None,
            outcome: None,
            eco,
        }
    }

    // ------------------------------------------------------------ accessors

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn setup(&self) -> &GameSetup {
        &self.setup
    }

    pub fn position(&self) -> &Position {
        self.positions.last().unwrap_or(&self.start_position)
    }

    /// The number of plies played (also the index of the next move).
    pub fn ply(&self) -> u32 {
        ply_u32(self.plies.len())
    }

    pub fn turn(&self) -> Side {
        self.position().turn()
    }

    pub fn plies(&self) -> &[PlyRecord] {
        &self.plies
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    pub fn eco(&self) -> Option<&EcoMatch> {
        self.eco.as_ref()
    }

    pub fn pending_offer(&self) -> Option<Side> {
        self.pending_offer
    }

    pub fn offer_blocked_for(&self) -> Option<Side> {
        self.offer_blocked_for
    }

    /// The repetition keys of every position since the start, the start included.
    pub fn repetition_keys(&self) -> &[PositionKey] {
        &self.keys
    }

    pub fn context(&self) -> GameContext {
        GameContext {
            mode: self.setup.mode,
            matchup: self.setup.matchup,
            state: self.state,
            turn: self.turn(),
            white_kind: self.setup.white.kind,
            black_kind: self.setup.black.kind,
            pending_offer: self.pending_offer,
            offer_blocked_for: self.offer_blocked_for,
        }
    }

    fn kind_of(&self, side: Side) -> PlayerKind {
        self.setup.seat(side).kind
    }

    /// The pure termination check on the current position (3.11.2). A game with no move played
    /// has no pending termination: a start position that is already mate or stalemate is
    /// refused at setup (ASM-A2 in the evidence file).
    pub fn pending_termination(&self) -> Option<Outcome> {
        if self.plies.is_empty() {
            return None;
        }
        termination_of(self.position(), &self.keys)
    }

    pub fn view(&self, stamp: Stamp) -> GameView {
        GameView {
            state: self.state,
            ply: self.ply(),
            turn: self.turn(),
            plies: self.plies.clone(),
            clocks: self.clock.sync(stamp.mono, stamp.wall),
            allowed_actions: Policy::allowed_actions(&self.context()),
            outcome: self.outcome,
            eco: self.eco.clone(),
        }
    }

    // ------------------------------------------------------------ equality and invariants

    /// The state a journal can express: the transient states and `Finalized` map to the
    /// journaled one (EngineThinking and EngineError to AwaitingMove, or to Paused during a
    /// step; Finalized to Completed).
    fn persistent_state(&self) -> GameState {
        match self.state {
            GameState::EngineThinking | GameState::EngineError => {
                if self.stepping {
                    GameState::Paused
                } else {
                    GameState::AwaitingMove
                }
            }
            GameState::Finalized => GameState::Completed,
            other => other,
        }
    }

    /// Whether two games are equal in everything a journal records (the "field by field"
    /// equality of SDD 3.11.1).
    ///
    /// Compared: setup, persistent state, resume target, every ply record, repetition keys,
    /// the position, pending and blocked draw offers, outcome, ECO match, and the clock
    /// (spec and remaining times) while the game is not finished.
    ///
    /// **Not compared** (not journaled, ruling B5, Brian Prascher 2026-11-09, desk review D1(b)):
    /// - the remaining times of a **finished** game, whether or not a journaled `Clock`
    ///   checkpoint preceded `Ended` (RC-02). The writer does not journal the final clocks, so
    ///   a replay shows the clocks of the last journaled `Move` or `Clock` fact, not the final
    ///   ones (flag at 11 s on a 10 s clock: live 0 ms, replay 10 000 ms). A `Clock` fact
    ///   before `Ended` is applied on replay and its content (remaining times, running side)
    ///   is then not compared; altering or dropping it does not change this result. The clock
    ///   *spec* is still compared, and so are all other fields listed above. The time totals
    ///   of a finished game come from `GameEnd`; no consumer may read the clock of a finished
    ///   game from a replay.
    /// - the transient states `EngineThinking` and `EngineError` and a step in progress
    ///   (`stepping`); they compare as their journaled state (`AwaitingMove` or `Paused`).
    /// - `Finalized`, a store event; it compares as `Completed`.
    /// - the running interval of a clock (which side runs since when).
    pub fn journal_eq(&self, other: &Game) -> bool {
        let same_clock = if self.persistent_state().is_finished() {
            self.clock.spec() == other.clock.spec()
        } else {
            self.clock.journaled_eq(&other.clock)
        };
        self.setup == other.setup
            && self.persistent_state() == other.persistent_state()
            && self.resume_target == other.resume_target
            && self.plies == other.plies
            && self.keys == other.keys
            && self.position().to_fen() == other.position().to_fen()
            && self.pending_offer == other.pending_offer
            && self.offer_blocked_for == other.offer_blocked_for
            && self.outcome == other.outcome
            && self.eco == other.eco
            && same_clock
    }

    /// The structural invariants every reachable `Game` holds.
    pub fn check_invariants(&self) -> Result<(), &'static str> {
        if self.positions.len() != self.plies.len() || self.keys.len() != self.plies.len() + 1 {
            return Err("history lengths disagree");
        }
        if self.plies.len() > MAX_PLIES as usize {
            return Err("too many plies");
        }
        for (i, ply) in self.plies.iter().enumerate() {
            if usize::try_from(ply.ply).ok() != Some(i) {
                return Err("ply numbers are not consecutive");
            }
        }
        let last_key = self.position().repetition_key();
        if self.keys.last() != Some(&last_key) {
            return Err("last key is not the position's key");
        }
        let completed = matches!(self.state, GameState::Completed | GameState::Finalized);
        if completed != self.outcome.is_some() && self.state != GameState::Voided {
            // A voided game may keep the outcome of the Ended fact that preceded the void.
            return Err("outcome and state disagree");
        }
        if (self.state == GameState::Suspended) != self.resume_target.is_some() {
            return Err("resume target and state disagree");
        }
        let offer_open = self.state == GameState::DrawOffered
            || (self.state == GameState::Suspended
                && self.resume_target == Some(GameState::DrawOffered));
        if offer_open != self.pending_offer.is_some() {
            return Err("pending offer and state disagree");
        }
        if self.stepping
            && !matches!(
                self.state,
                GameState::EngineThinking | GameState::EngineError
            )
        {
            return Err("stepping outside a search");
        }
        Ok(())
    }

    // ------------------------------------------------------------ evolve (the only mutator)

    /// Applies one fact to a copy. Never reads a clock.
    pub fn evolve(&self, fact: Fact) -> Result<Game, ReplayError> {
        let mut next = self.clone();
        next.apply_fact(&fact)?;
        Ok(next)
    }

    /// `Suspended` accepts only the facts of 3.11.1; anything else is `NotResumed`.
    fn gate(&self, allowed: &[GameState]) -> Result<(), ReplayError> {
        if allowed.contains(&self.state) {
            Ok(())
        } else if self.state == GameState::Suspended {
            Err(ReplayError::NotResumed)
        } else {
            Err(ReplayError::WrongState { state: self.state })
        }
    }

    fn apply_fact(&mut self, fact: &Fact) -> Result<(), ReplayError> {
        use GameState as S;
        const PLAY: [S; 3] = [S::AwaitingMove, S::EngineThinking, S::Paused];
        const BEFORE_END: [S; 5] = [
            S::AwaitingMove,
            S::EngineThinking,
            S::EngineError,
            S::DrawOffered,
            S::Paused,
        ];
        match fact {
            Fact::Start { .. } => Err(ReplayError::SecondStart),
            Fact::Move {
                ply,
                mv,
                think_ms,
                clocks_after,
                played_at,
            } => {
                self.gate(&PLAY)?;
                let expected = self.ply();
                if *ply != expected {
                    return Err(ReplayError::WrongPly {
                        expected,
                        found: *ply,
                    });
                }
                if expected >= MAX_PLIES {
                    return Err(ReplayError::TooLong);
                }
                let before = self.position().clone();
                let after = before.play(*mv)?;
                let san = before.san(*mv)?;
                let mover = before.turn();
                let key = after.repetition_key();
                self.eco = EcoTable::observe(self.eco.take(), &key, expected + 1);
                self.plies.push(PlyRecord {
                    ply: expected,
                    mv: *mv,
                    san: san.to_string(),
                    fen_after: after.to_fen().to_string(),
                    think_ms: think_ms.unwrap_or(0),
                    clocks_after: *clocks_after,
                    played_at: *played_at,
                });
                self.keys.push(key);
                self.positions.push(after);
                self.clock.apply_move(mover, *clocks_after);
                if self.offer_blocked_for == Some(mover) {
                    self.offer_blocked_for = None;
                }
                self.state = if self.stepping || self.state == GameState::Paused {
                    GameState::Paused
                } else {
                    GameState::AwaitingMove
                };
                self.stepping = false;
                Ok(())
            }
            Fact::Takeback { to_ply } => {
                self.gate(&[S::AwaitingMove, S::EngineThinking, S::EngineError])?;
                let to = usize::try_from(*to_ply).map_err(|_| ReplayError::BadTakeback)?;
                if to >= self.plies.len() {
                    return Err(ReplayError::BadTakeback);
                }
                self.plies.truncate(to);
                self.positions.truncate(to);
                self.keys.truncate(to + 1);
                self.eco = EcoTable::classify(&self.keys);
                self.state = S::AwaitingMove;
                self.stepping = false;
                self.offer_blocked_for = None;
                let turn = self.turn();
                self.clock.reset_turn(turn);
                Ok(())
            }
            Fact::Clock { clocks, .. } => {
                // A `Clock` after the end is a harmless record: it is ignored, not refused
                // (desk review D2(ii)); refusing it would make the journal unreadable.
                if self.state.is_finished() {
                    return Ok(());
                }
                if let Some(values) = clocks {
                    self.clock.restore(*values);
                }
                Ok(())
            }
            Fact::DrawOffer { by } => {
                self.gate(&[S::AwaitingMove])?;
                self.pending_offer = Some(*by);
                self.state = S::DrawOffered;
                Ok(())
            }
            Fact::DrawDeclined { by } => {
                self.gate(&[S::DrawOffered])?;
                self.offer_blocked_for = Some(self.pending_offer.unwrap_or(*by));
                self.pending_offer = None;
                self.state = S::AwaitingMove;
                Ok(())
            }
            Fact::DrawDropped { .. } => {
                self.gate(&[S::DrawOffered])?;
                self.pending_offer = None;
                self.state = S::AwaitingMove;
                Ok(())
            }
            Fact::Suspend { .. } => {
                self.gate(&BEFORE_END)?;
                let target = match self.state {
                    S::DrawOffered => S::DrawOffered,
                    S::Paused => S::Paused,
                    S::EngineThinking | S::EngineError if self.stepping => S::Paused,
                    _ => S::AwaitingMove,
                };
                self.resume_target = Some(target);
                self.stepping = false;
                self.state = S::Suspended;
                Ok(())
            }
            Fact::Pause => {
                // Paused is accepted too: a c-c `Step` has no fact, so a journal can show Pause
                // twice (Pause, Step, Pause) and replay sees the second one in Paused.
                self.gate(&[S::AwaitingMove, S::EngineThinking, S::Paused])?;
                self.stepping = false;
                self.state = S::Paused;
                Ok(())
            }
            Fact::PlayResume => {
                self.gate(&[S::Paused])?;
                self.state = S::AwaitingMove;
                Ok(())
            }
            Fact::SuspendResume => {
                if self.state != S::Suspended {
                    return Err(ReplayError::WrongState { state: self.state });
                }
                self.state = self.resume_target.take().unwrap_or(S::AwaitingMove);
                Ok(())
            }
            Fact::Ended { outcome } => {
                // The outcome is stored as recorded and is not re-derived from the position
                // (desk review D2(i)); the journal CRC and the store reconciliation are the
                // integrity check (architecture 7.6).
                if self.state != S::Suspended {
                    self.gate(&BEFORE_END)?;
                }
                self.finish(S::Completed);
                self.outcome = Some(*outcome);
                Ok(())
            }
            Fact::Aborted => {
                if self.state != S::Suspended {
                    self.gate(&BEFORE_END)?;
                }
                self.finish(S::Aborted);
                Ok(())
            }
            Fact::Voided { .. } => {
                if !matches!(self.state, S::Suspended | S::Completed) {
                    self.gate(&BEFORE_END)?;
                }
                self.finish(S::Voided);
                Ok(())
            }
        }
    }

    fn finish(&mut self, state: GameState) {
        self.state = state;
        self.resume_target = None;
        self.pending_offer = None;
        self.stepping = false;
    }

    // ------------------------------------------------------------ replay (DD-DOM-010)

    /// Rebuilds a game from its facts in journal order. Never panics; stops at the first fact
    /// that does not fit and reports how many were applied.
    pub fn replay(items: impl IntoIterator<Item = ReplayItem>) -> ReplayOutcome {
        let mut out = ReplayOutcome {
            game: None,
            applied: 0,
            interrupted: false,
            terminal: None,
            warnings: Vec::new(),
            error: None,
        };
        let mut iter = items.into_iter();
        let Some(first) = iter.next() else {
            out.error = Some(ReplayError::NotStarted);
            return out;
        };
        let Fact::Start { setup, .. } = first.fact else {
            out.error = Some(ReplayError::NotStarted);
            return out;
        };
        if let Err(e) = Policy::check_setup(&setup) {
            out.error = Some(ReplayError::BadSetup(e));
            return out;
        }
        let mut game = Game::from_start(setup);
        out.applied = 1;
        for item in iter {
            if let Err(e) = game.replay_item(&item, &mut out.warnings) {
                out.error = Some(e);
                break;
            }
            out.applied += 1;
        }
        if out.error.is_none() && !game.state.is_finished() {
            out.terminal = game.pending_termination();
            out.interrupted = out.terminal.is_none() && game.state != GameState::Suspended;
        }
        out.game = Some(game);
        out
    }

    fn replay_item(
        &mut self,
        item: &ReplayItem,
        warnings: &mut Vec<ReplayWarning>,
    ) -> Result<(), ReplayError> {
        match (&item.fact, &item.check) {
            (Fact::Move { ply, mv, .. }, Some(check)) if *ply == self.ply() => {
                if let Ok(after) = self.position().play(*mv) {
                    let recorded = Fen::parse(&check.fen_after)
                        .map(|f| Position::from_fen(&f))
                        .ok();
                    if !recorded.is_some_and(|r| same_position(&r, &after)) {
                        return Err(ReplayError::FenMismatch { ply: *ply });
                    }
                    if let Ok(computed) = self.position().san(*mv)
                        && computed.as_str() != check.san
                    {
                        warnings.push(ReplayWarning::SanMismatch {
                            ply: *ply,
                            recorded: check.san.clone(),
                            computed: computed.to_string(),
                        });
                    }
                }
            }
            (Fact::DrawDropped { by, .. }, _) => {
                if let Some(pending) = self.pending_offer
                    && pending != *by
                    && self.state == GameState::DrawOffered
                {
                    warnings.push(ReplayWarning::DrawDroppedBy {
                        recorded: *by,
                        pending,
                    });
                }
            }
            _ => {}
        }
        self.apply_fact(&item.fact)
    }

    // ------------------------------------------------------------ decide

    fn deny_finished(&self) -> Result<(), GameError> {
        match self.state {
            GameState::Voided => Err(denied(DeniedReason::Voided)),
            s if s.is_finished() => Err(denied(DeniedReason::GameOver)),
            _ => Ok(()),
        }
    }

    fn require(&self, ok: bool) -> Result<(), GameError> {
        if ok {
            Ok(())
        } else {
            Err(denied(DeniedReason::StateNotAllowed))
        }
    }

    /// Applies `facts` with `evolve` to a copy, then installs the decided runtime clock.
    fn commit(&self, facts: Vec<Fact>, clock: Clock) -> Result<Step, GameError> {
        let mut game = self.clone();
        for fact in &facts {
            game.apply_fact(fact)
                .map_err(|_| GameError::InvalidTransition)?;
        }
        game.clock = clock;
        let ended = facts.iter().find_map(|f| match f {
            Fact::Ended { outcome } => Some(*outcome),
            _ => None,
        });
        Ok(Step { game, facts, ended })
    }

    /// A step that changes only transient state (no fact).
    fn transient(&self, state: GameState, stepping: bool, clock: Clock) -> Step {
        let mut game = self.clone();
        game.state = state;
        game.stepping = stepping;
        game.clock = clock;
        Step {
            game,
            facts: Vec::new(),
            ended: None,
        }
    }

    fn checkpoint_fact(clock: &Clock, now: crate::types::MonoMs) -> Fact {
        Fact::Clock {
            clocks: clock.checkpoint(now),
            running: clock.running_side(),
        }
    }

    pub fn decide(&self, cmd: GameCommand, stamp: Stamp) -> Result<Step, GameError> {
        let now = stamp.mono;
        let turn = self.turn();
        let ctx = self.context();
        let check = |action| Policy::check(action, &ctx).map_err(GameError::from);
        if cmd != GameCommand::MarkFinalized {
            self.deny_finished()?;
        }
        let mut clock = self.clock;
        match cmd {
            GameCommand::Move {
                mv,
                expected_ply,
                actor,
            } => {
                check(Action::MakeMove { by: turn, actor })?;
                if expected_ply != self.ply() {
                    return Err(GameError::StalePosition {
                        live_ply: self.ply(),
                    });
                }
                if self.ply() >= MAX_PLIES {
                    return Err(GameError::TooLong);
                }
                if clock.remaining(turn, now) == Some(0) {
                    clock.stop(now);
                    let outcome = flag_outcome(turn, self.position());
                    return self.commit(vec![Fact::Ended { outcome }], clock);
                }
                let after = self.position().play(mv)?;
                let timing = clock.after_move(turn, now);
                if self.stepping {
                    clock.stop(now);
                }
                let mut facts = vec![Fact::Move {
                    ply: self.ply(),
                    mv,
                    think_ms: Some(timing.think_ms),
                    clocks_after: clock.checkpoint(now),
                    played_at: stamp.wall,
                }];
                let mut keys = self.keys.clone();
                keys.push(after.repetition_key());
                if let Some(outcome) = termination_of(&after, &keys) {
                    clock.stop(now);
                    facts.push(Fact::Ended { outcome });
                }
                self.commit(facts, clock)
            }
            GameCommand::Takeback => {
                check(Action::Takeback)?;
                let n = self.plies.len();
                let human_to_move = self.kind_of(turn) == PlayerKind::Human;
                let start_side = self.setup.start.side_to_move();
                let floor = usize::from(self.kind_of(start_side) != PlayerKind::Human);
                let target = n.checked_sub(if human_to_move { 2 } else { 1 });
                let Some(target) = target.filter(|t| *t >= floor) else {
                    return Err(GameError::NothingToTakeBack);
                };
                let mut step = self.commit(
                    vec![Fact::Takeback {
                        to_ply: ply_u32(target),
                    }],
                    clock,
                )?;
                let mut c = step.game.clock;
                c.start(step.game.turn(), now);
                step.game.clock = c;
                Ok(step)
            }
            GameCommand::OfferDraw { by } => {
                check(Action::OfferDraw { by })?;
                clock.stop(now);
                let facts = vec![Fact::DrawOffer { by }, Game::checkpoint_fact(&clock, now)];
                self.commit(facts, clock)
            }
            GameCommand::RespondDraw { accept } => {
                let Some(offerer) = self.pending_offer else {
                    return Err(denied(DeniedReason::StateNotAllowed));
                };
                let by = offerer.other();
                let actor = if self.kind_of(by) == PlayerKind::Human {
                    Actor::Human
                } else {
                    Actor::Engine
                };
                check(Action::RespondDraw { by, actor })?;
                if accept {
                    let outcome = Outcome {
                        result: GameResult::Draw,
                        reason: EndReason::DrawAgreed,
                    };
                    self.commit(vec![Fact::Ended { outcome }], clock)
                } else {
                    clock.start(turn, now);
                    self.commit(vec![Fact::DrawDeclined { by: offerer }], clock)
                }
            }
            GameCommand::DropDrawOffer => {
                self.require(self.state == GameState::DrawOffered)?;
                self.drop_offer(clock, now)
            }
            GameCommand::EngineFailed => match self.state {
                GameState::EngineThinking => {
                    Ok(self.transient(GameState::EngineError, self.stepping, clock))
                }
                GameState::DrawOffered
                    if self.pending_offer.map(|o| self.kind_of(o.other()))
                        == Some(PlayerKind::Computer) =>
                {
                    self.drop_offer(clock, now)
                }
                _ => Err(denied(DeniedReason::StateNotAllowed)),
            },
            GameCommand::EngineStarted => {
                self.require(
                    self.state == GameState::AwaitingMove
                        && self.kind_of(turn) == PlayerKind::Computer,
                )?;
                Ok(self.transient(GameState::EngineThinking, false, clock))
            }
            GameCommand::EngineRetry => {
                check(Action::RetryEngine)?;
                Ok(self.transient(GameState::EngineThinking, self.stepping, clock))
            }
            GameCommand::Pause => {
                check(Action::Pause)?;
                clock.stop(now);
                let facts = vec![Fact::Pause, Game::checkpoint_fact(&clock, now)];
                self.commit(facts, clock)
            }
            GameCommand::Step => {
                check(Action::Step)?;
                clock.start(turn, now);
                Ok(self.transient(GameState::EngineThinking, true, clock))
            }
            GameCommand::PlayResume => {
                check(Action::PlayResume)?;
                clock.start(turn, now);
                self.commit(vec![Fact::PlayResume], clock)
            }
            GameCommand::Suspend { cause } => {
                self.require(self.state.is_in_progress())?;
                clock.stop(now);
                let mut facts = Vec::new();
                if let Some(by) = self.pending_offer
                    && self.kind_of(by.other()) == PlayerKind::Computer
                {
                    facts.push(Fact::DrawDropped {
                        by,
                        cause: DropCause::Suspend,
                    });
                }
                facts.push(Fact::Suspend { cause });
                facts.push(Game::checkpoint_fact(&clock, now));
                self.commit(facts, clock)
            }
            GameCommand::SuspendResume => {
                self.require(self.state == GameState::Suspended)?;
                let mut step = self.commit(vec![Fact::SuspendResume], clock)?;
                if step.game.state == GameState::AwaitingMove {
                    let mut c = step.game.clock;
                    c.start(step.game.turn(), now);
                    step.game.clock = c;
                }
                Ok(step)
            }
            GameCommand::Resign { side } => {
                check(Action::Resign { side })?;
                clock.stop(now);
                let outcome = Outcome {
                    result: GameResult::win_for(side.other()),
                    reason: EndReason::Resignation,
                };
                self.commit(vec![Fact::Ended { outcome }], clock)
            }
            GameCommand::Flag => {
                let Some(side) = clock.running_side() else {
                    return Err(GameError::FlagNotFallen);
                };
                if clock.remaining(side, now) != Some(0) {
                    return Err(GameError::FlagNotFallen);
                }
                clock.stop(now);
                let outcome = flag_outcome(side, self.position());
                self.commit(vec![Fact::Ended { outcome }], clock)
            }
            GameCommand::Abort => {
                check(Action::Abort)?;
                clock.stop(now);
                self.commit(vec![Fact::Aborted], clock)
            }
            GameCommand::Void {
                reason,
                reset_player,
            } => {
                self.require(self.state.is_in_progress() || self.state == GameState::Suspended)?;
                if reason == VoidReason::Reset && self.setup.matchup == Matchup::ComputerComputer {
                    return Err(denied(DeniedReason::StateNotAllowed));
                }
                clock.stop(now);
                self.commit(
                    vec![Fact::Voided {
                        reason,
                        reset_player,
                    }],
                    clock,
                )
            }
            GameCommand::ResolveTerminal => {
                let outcome = self
                    .pending_termination()
                    .ok_or(denied(DeniedReason::StateNotAllowed))?;
                clock.stop(now);
                self.commit(vec![Fact::Ended { outcome }], clock)
            }
            GameCommand::Checkpoint => {
                // Charge the running interval and restart it, so the live remaining time
                // equals the checkpoint a replay will restore.
                if let Some(side) = clock.running_side() {
                    clock.stop(now);
                    clock.start(side, now);
                }
                let facts = vec![Game::checkpoint_fact(&clock, now)];
                self.commit(facts, clock)
            }
            GameCommand::MarkFinalized => {
                if self.state != GameState::Completed {
                    self.deny_finished()?;
                    return Err(denied(DeniedReason::StateNotAllowed));
                }
                // T19: Finalized is a store event with no journal record.
                let step = self.transient(GameState::Finalized, false, clock);
                Ok(step)
            }
        }
    }

    fn drop_offer(&self, mut clock: Clock, now: crate::types::MonoMs) -> Result<Step, GameError> {
        let Some(by) = self.pending_offer else {
            return Err(denied(DeniedReason::StateNotAllowed));
        };
        clock.start(self.turn(), now);
        self.commit(
            vec![Fact::DrawDropped {
                by,
                cause: DropCause::EngineFailure,
            }],
            clock,
        )
    }
}

#[cfg(test)]
mod tests {
    //! `journal_eq` and `check_invariants` are the oracles of the replay property tests, so
    //! they are tested directly: a game that differs in exactly one field must be unequal, and
    //! each corrupted field must be reported. Private fields are reachable here, which is the
    //! only way to build a `Game` the public API cannot reach.
    use super::*;
    use crate::policy::StartPosition;
    use crate::types::{GameId, Matchup, Mode, MonoMs, Seat};

    fn seat(n: u128, kind: PlayerKind) -> Seat {
        Seat {
            player: PlayerId::from_uuid(crate::Uuid::from_u128(n)),
            kind,
        }
    }

    fn setup() -> GameSetup {
        GameSetup {
            game_id: GameId::from_uuid(crate::Uuid::from_u128(9)),
            matchup: Matchup::HumanHuman,
            mode: Mode::Strict,
            white: seat(1, PlayerKind::Human),
            black: seat(2, PlayerKind::Human),
            start: StartPosition::Standard,
            clock: crate::ClockSpec::from_seconds(60, 0).ok(),
        }
    }

    fn stamp(ms: u64) -> Stamp {
        Stamp {
            mono: MonoMs(ms),
            wall: WallMs(0),
        }
    }

    fn mv(uci: &str) -> Move {
        Move::from_uci(uci).unwrap()
    }

    /// Two plies played, Black to move... White to move again, timed, nothing else open.
    fn base() -> Game {
        let mut g = Game::new(setup(), stamp(0)).unwrap().game;
        for (i, m) in ["e2e4", "e7e5"].into_iter().enumerate() {
            let c = GameCommand::Move {
                mv: mv(m),
                expected_ply: g.ply(),
                actor: Actor::Human,
            };
            g = g.decide(c, stamp(1_000 * (i as u64 + 1))).unwrap().game;
        }
        g
    }

    fn differs(label: &str, edit: impl FnOnce(&mut Game)) {
        let a = base();
        let mut b = base();
        edit(&mut b);
        assert!(
            a.journal_eq(&a) && a.journal_eq(&base()),
            "{label}: reflexive"
        );
        assert!(!a.journal_eq(&b), "{label}: must differ");
        assert!(!b.journal_eq(&a), "{label}: must differ (symmetric)");
    }

    #[test]
    fn ut_dom_042_journal_eq_is_false_when_exactly_one_journaled_field_differs() {
        differs("setup", |g| g.setup.mode = Mode::Training);
        differs("state", |g| g.state = GameState::Paused);
        differs("resume_target", |g| {
            g.resume_target = Some(GameState::Paused)
        });
        differs("plies", |g| g.plies[0].think_ms += 1);
        differs("keys", |g| {
            let k = g.keys[0].clone();
            g.keys.push(k);
        });
        differs("position", |g| g.positions.push(Position::standard()));
        differs("pending_offer", |g| g.pending_offer = Some(Side::Black));
        differs("offer_blocked_for", |g| {
            g.offer_blocked_for = Some(Side::White)
        });
        differs("outcome", |g| {
            g.outcome = Some(Outcome {
                result: GameResult::Draw,
                reason: EndReason::DrawAgreed,
            });
        });
        differs("eco", |g| {
            g.eco = Some(EcoMatch {
                code: "A00".into(),
                name: "x".into(),
                ply: 0,
            });
        });
        // The clock remaining differs only after time passes on a running clock.
        let a = base();
        let mut b = base();
        b.clock.stop(MonoMs(9_000));
        assert!(!a.journal_eq(&b), "clock remaining");
    }

    #[test]
    fn ut_dom_042_journal_eq_ignores_what_a_journal_cannot_hold() {
        let a = base();
        let mut b = base();
        b.state = GameState::EngineThinking;
        assert!(
            a.journal_eq(&b),
            "a search in progress is journaled as AwaitingMove"
        );
        b.state = GameState::EngineError;
        assert!(a.journal_eq(&b));
        b.stepping = true;
        let mut paused = base();
        paused.state = GameState::Paused;
        assert!(
            b.journal_eq(&paused),
            "a step in progress is journaled as Paused"
        );
        let mut done = base();
        done.state = GameState::Completed;
        done.outcome = Some(Outcome {
            result: GameResult::Draw,
            reason: EndReason::DrawAgreed,
        });
        let mut fin = done.clone();
        fin.state = GameState::Finalized;
        assert!(done.journal_eq(&fin), "Finalized is a store event");
        // the remaining times of a finished game are live-only (ruling B5)
        fin.clock.stop(MonoMs(50_000));
        assert!(done.journal_eq(&fin));
    }

    #[test]
    fn ut_dom_042_b5_finished_games_still_compare_the_clock_spec() {
        // Narrow skip: only the remaining times of a finished game are not compared. The spec
        // (timed or untimed, base, increment) is not a runtime value and stays compared.
        let mut done = base();
        done.state = GameState::Completed;
        done.outcome = Some(Outcome {
            result: GameResult::Draw,
            reason: EndReason::DrawAgreed,
        });
        let mut other = done.clone();
        other.clock = Clock::new(None);
        assert!(!done.journal_eq(&other), "spec differs");
        assert!(!other.journal_eq(&done), "spec differs (symmetric)");
        let mut shorter = done.clone();
        shorter.clock = Clock::new(crate::ClockSpec::from_seconds(61, 0).ok());
        assert!(!done.journal_eq(&shorter), "base differs");
    }

    fn broken(label: &str, want: &str, edit: impl FnOnce(&mut Game)) {
        let mut g = base();
        assert_eq!(g.check_invariants(), Ok(()), "{label}: base is sound");
        edit(&mut g);
        assert_eq!(g.check_invariants(), Err(want), "{label}");
    }

    #[test]
    fn ut_dom_035_check_invariants_reports_each_corruption() {
        broken("positions short", "history lengths disagree", |g| {
            g.positions.pop();
        });
        broken("keys long", "history lengths disagree", |g| {
            let k = g.keys[0].clone();
            g.keys.push(k);
        });
        broken("ply numbers", "ply numbers are not consecutive", |g| {
            g.plies[1].ply = 7;
        });
        broken("last key", "last key is not the position's key", |g| {
            let k = g.keys[0].clone();
            *g.keys.last_mut().unwrap() = k;
        });
        broken(
            "completed without outcome",
            "outcome and state disagree",
            |g| {
                g.state = GameState::Completed;
            },
        );
        broken(
            "outcome without completed",
            "outcome and state disagree",
            |g| {
                g.outcome = Some(Outcome {
                    result: GameResult::Draw,
                    reason: EndReason::DrawAgreed,
                });
            },
        );
        broken(
            "suspended without target",
            "resume target and state disagree",
            |g| {
                g.state = GameState::Suspended;
            },
        );
        broken(
            "target without suspended",
            "resume target and state disagree",
            |g| {
                g.resume_target = Some(GameState::Paused);
            },
        );
        broken(
            "offer without state",
            "pending offer and state disagree",
            |g| {
                g.pending_offer = Some(Side::White);
            },
        );
        broken(
            "state without offer",
            "pending offer and state disagree",
            |g| {
                g.state = GameState::DrawOffered;
            },
        );
        broken(
            "stepping outside a search",
            "stepping outside a search",
            |g| {
                g.stepping = true;
            },
        );
    }

    #[test]
    fn ut_dom_035_check_invariants_accepts_the_legal_special_cases() {
        let mut v = base();
        v.state = GameState::Voided;
        assert_eq!(v.check_invariants(), Ok(()), "voided with no outcome");
        v.outcome = Some(Outcome {
            result: GameResult::Draw,
            reason: EndReason::DrawAgreed,
        });
        assert_eq!(
            v.check_invariants(),
            Ok(()),
            "voided after an Ended fact keeps the outcome"
        );
        let mut s = base();
        s.state = GameState::Suspended;
        s.resume_target = Some(GameState::DrawOffered);
        s.pending_offer = Some(Side::White);
        assert_eq!(
            s.check_invariants(),
            Ok(()),
            "an offer to a human survives a suspend"
        );
        let mut t = base();
        t.state = GameState::EngineThinking;
        t.stepping = true;
        assert_eq!(t.check_invariants(), Ok(()));
    }

    #[test]
    fn ut_dom_035_check_invariants_bounds_the_ply_count() {
        let mut g = base();
        let key = g.keys[0].clone();
        let pos = g.positions[0].clone();
        let rec = g.plies[0].clone();
        let fill = |g: &mut Game, n: usize| {
            g.plies.clear();
            g.positions.clear();
            g.keys.truncate(1);
            for i in 0..n {
                let mut r = rec.clone();
                r.ply = u32::try_from(i).unwrap();
                g.plies.push(r);
                g.positions.push(pos.clone());
                g.keys.push(key.clone());
            }
            // keep the last-key rule satisfied
            let last = g.position().repetition_key();
            *g.keys.last_mut().unwrap() = last;
        };
        fill(&mut g, MAX_PLIES as usize);
        assert_eq!(g.check_invariants(), Ok(()), "exactly the limit is allowed");
        fill(&mut g, MAX_PLIES as usize + 1);
        assert_eq!(g.check_invariants(), Err("too many plies"));
    }
}
