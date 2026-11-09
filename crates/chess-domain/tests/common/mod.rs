//! Shared fixtures for the chess-domain integration tests (public API only).
//! chess-domain is pure, so no CHESS_DATA_DIR helper is included (ruling-test-helper-placement).
#![allow(
    dead_code,
    reason = "each test file uses a different subset of these helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]

use core::sync::atomic::{AtomicU64, Ordering};

use chess_domain::{
    ClockSpec, GameId, GameSetup, Matchup, Mode, MonoMs, PlayerId, PlayerKind, Seat, Stamp,
    StartPosition, TimeSource, Uuid, WallMs,
};

pub fn pid(n: u128) -> PlayerId {
    PlayerId::from_uuid(Uuid::from_u128(n))
}

pub fn gid(n: u128) -> GameId {
    GameId::from_uuid(Uuid::from_u128(n))
}

pub fn human(n: u128) -> Seat {
    Seat {
        player: pid(n),
        kind: PlayerKind::Human,
    }
}

pub fn computer(n: u128) -> Seat {
    Seat {
        player: pid(n),
        kind: PlayerKind::Computer,
    }
}

pub fn at(mono: u64) -> Stamp {
    Stamp {
        mono: MonoMs(mono),
        wall: WallMs(1_000_000 + i64::try_from(mono).unwrap()),
    }
}

pub fn spec(minutes: u32, seconds: u32) -> ClockSpec {
    ClockSpec::from_minutes_plus_seconds(minutes, seconds).unwrap()
}

/// Training, human (White, id 1) against the computer (Black, id 2), untimed, standard start.
pub fn training() -> GameSetup {
    GameSetup {
        game_id: gid(100),
        matchup: Matchup::HumanComputer,
        mode: Mode::Training,
        white: human(1),
        black: computer(2),
        start: StartPosition::Standard,
        clock: None,
    }
}

/// Strict human against human, 5+0.
pub fn strict_hh() -> GameSetup {
    GameSetup {
        game_id: gid(101),
        matchup: Matchup::HumanHuman,
        mode: Mode::Strict,
        white: human(1),
        black: human(3),
        start: StartPosition::Standard,
        clock: Some(spec(5, 0)),
    }
}

/// Strict human (White) against computer (Black), untimed.
pub fn strict_hc() -> GameSetup {
    GameSetup {
        game_id: gid(102),
        matchup: Matchup::HumanComputer,
        mode: Mode::Strict,
        white: human(1),
        black: computer(2),
        start: StartPosition::Standard,
        clock: None,
    }
}

/// Strict computer against computer, untimed.
pub fn strict_cc() -> GameSetup {
    GameSetup {
        game_id: gid(103),
        matchup: Matchup::ComputerComputer,
        mode: Mode::Strict,
        white: computer(4),
        black: computer(5),
        start: StartPosition::Standard,
        clock: None,
    }
}

/// A time source under test control. The crate's own `FakeTimeSource` is behind feature
/// `test-support`, which plain `cargo test --workspace` does not enable for integration tests
/// (GATE-103 finding F-D1); the same four lines are repeated here.
#[derive(Debug)]
pub struct FakeTime {
    mono: AtomicU64,
    wall: AtomicU64,
}

impl FakeTime {
    pub fn new(mono: u64, wall: u64) -> FakeTime {
        FakeTime {
            mono: AtomicU64::new(mono),
            wall: AtomicU64::new(wall),
        }
    }

    pub fn advance(&self, ms: u64) {
        self.mono.fetch_add(ms, Ordering::SeqCst);
        self.wall.fetch_add(ms, Ordering::SeqCst);
    }
}

impl TimeSource for FakeTime {
    fn mono(&self) -> MonoMs {
        MonoMs(self.mono.load(Ordering::SeqCst))
    }

    fn wall(&self) -> WallMs {
        WallMs(i64::try_from(self.wall.load(Ordering::SeqCst)).unwrap())
    }
}

// ---------------------------------------------------------------- scripted game runs

use chess_domain::{
    ActionKind, Actor, Fact, Game, GameCommand, GameError, ReplayItem, ReplayOutcome, Step,
};
use chess_rules::{Fen, Move, Side};

pub const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn mvu(uci: &str) -> Move {
    Move::from_uci(uci).unwrap()
}

pub fn custom(fen: &str) -> StartPosition {
    StartPosition::from_fen(Fen::parse(fen).unwrap())
}

pub fn with_start(mut setup: GameSetup, fen: &str) -> GameSetup {
    setup.start = custom(fen);
    setup
}

/// A game driven by commands with fake time; keeps every fact like a journal would.
pub struct Run {
    pub game: Game,
    pub facts: Vec<Fact>,
    pub now: u64,
}

impl Run {
    pub fn new(setup: GameSetup) -> Run {
        let step = Game::new(setup, at(0)).unwrap();
        Run {
            game: step.game,
            facts: step.facts,
            now: 0,
        }
    }

    pub fn tick(&mut self, ms: u64) -> &mut Run {
        self.now += ms;
        self
    }

    pub fn try_cmd(&mut self, c: GameCommand) -> Result<Step, GameError> {
        let step = self.game.decide(c, at(self.now))?;
        self.game = step.game.clone();
        self.facts.extend(step.facts.iter().cloned());
        Ok(step)
    }

    pub fn cmd(&mut self, c: GameCommand) -> Step {
        self.try_cmd(c.clone())
            .unwrap_or_else(|e| panic!("{c:?}: {e}"))
    }

    pub fn move_cmd(&self, uci: &str, actor: Actor) -> GameCommand {
        GameCommand::Move {
            mv: mvu(uci),
            expected_ply: self.game.ply(),
            actor,
        }
    }

    /// A human move.
    pub fn mv(&mut self, uci: &str) -> Step {
        let c = self.move_cmd(uci, Actor::Human);
        self.cmd(c)
    }

    /// `EngineStarted` then the engine's move.
    pub fn engine(&mut self, uci: &str) -> Step {
        self.cmd(GameCommand::EngineStarted);
        let c = self.move_cmd(uci, Actor::Engine);
        self.cmd(c)
    }

    pub fn replay(&self) -> ReplayOutcome {
        Game::replay(items(&self.facts))
    }
}

pub fn items(facts: &[Fact]) -> Vec<ReplayItem> {
    facts
        .iter()
        .cloned()
        .map(|fact| ReplayItem { fact, check: None })
        .collect()
}

pub fn kinds(list: &[ActionKind]) -> Vec<ActionKind> {
    list.to_vec()
}

pub fn side_name(s: Side) -> &'static str {
    match s {
        Side::White => "white",
        Side::Black => "black",
    }
}

/// Returns the scripted values, then zeros (the crate's own `ScriptedRng` is behind feature
/// `test-support`, see `FakeTime`).
pub struct ScriptedRng {
    values: Vec<u64>,
    next: usize,
}

impl ScriptedRng {
    pub fn new(values: Vec<u64>) -> ScriptedRng {
        ScriptedRng { values, next: 0 }
    }
}

impl chess_domain::Rng for ScriptedRng {
    fn next_u64(&mut self) -> u64 {
        let v = self.values.get(self.next).copied().unwrap_or(0);
        self.next += 1;
        v
    }
}
