//! DTOs of the `StoreApi` method list (SDD 3.7.2). Every identity is a `PlayerId` or `GameId`
//! (TR-3, D-40); no integer row key appears in any type here. These are Rust DTOs of this
//! crate: none is serialized to a front-end and none derives `ts-rs`.

use chess_domain::{
    Category, ClockSpec, ComputerPlayerSpec, GameId, GameResult, GameStatus, Matchup, Mode,
    Personality, PlayerId, PlayerKind, RosterSeed, WallMs,
};

/// A human Player to register (`register_player`). `created_at` comes from the store's
/// injected clock (IFR-ST-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPlayer {
    pub id: PlayerId,
    pub name: String,
    pub start_category: Category,
    pub strict_elo: i32,
    pub training_elo: i32,
    /// The initial `PREFERENCES` blob (UI-1). The store does not know the preference schema; it
    /// requires only valid JSON (the table CHECK).
    pub preferences_json: String,
}

/// One computer Player of a roster batch (`insert_computers`). `personality.error_permille`
/// is an integer per mille (D-43).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewComputer {
    pub id: PlayerId,
    pub name: String,
    pub start_category: Category,
    pub roster_seed: RosterSeed,
    pub roster_index: u32,
    pub personality: Personality,
    pub strict_elo: i32,
    pub training_elo: i32,
}

impl From<&ComputerPlayerSpec> for NewComputer {
    fn from(spec: &ComputerPlayerSpec) -> NewComputer {
        NewComputer {
            id: PlayerId::from_uuid(spec.uuid),
            name: spec.name.clone(),
            start_category: spec.category,
            roster_seed: spec.roster_seed,
            roster_index: spec.roster_index,
            personality: spec.personality,
            strict_elo: spec.strict_elo_start,
            training_elo: spec.training_elo_start,
        }
    }
}

/// The stored engine parameters of a computer Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputerProfile {
    pub roster_seed: RosterSeed,
    pub roster_index: u32,
    pub personality: Personality,
}

/// A `PLAYER` row (with its `COMPUTER_PROFILE` for a computer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    pub id: PlayerId,
    pub kind: PlayerKind,
    pub name: String,
    pub start_category: Category,
    pub strict_elo: i32,
    pub training_elo: i32,
    pub strict_rated_games: u32,
    pub training_rated_games: u32,
    pub reset_at: Option<WallMs>,
    pub archived: bool,
    pub created_at: WallMs,
    pub computer: Option<ComputerProfile>,
}

/// A game to start (`start_game`). `self_play` is derived (`white == black`, SF-20) and
/// `custom_start` is derived (`start_fen.is_some()`, SF-02): neither is a field, so the caller
/// cannot send an inconsistent pair. `start_fen` is `None` exactly for the standard start; the
/// comparison of all six FEN fields is made in `chess-domain`, not here (D-64).
/// `started_at` is the session's start stamp, written unchanged (OQ-26 option A, D-84): the
/// store reads no clock for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGame {
    pub id: GameId,
    pub white: PlayerId,
    pub black: PlayerId,
    pub matchup: Matchup,
    pub mode: Mode,
    pub rated: bool,
    pub start_fen: Option<String>,
    /// `None` for an untimed game. A timed game starts with both checkpoints at the base time.
    pub clock: Option<ClockSpec>,
    pub started_at: WallMs,
}

/// A whole `GAME` row (SDD 3.7.2: `get_game` and `list_games` return the whole row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRecord {
    pub id: GameId,
    pub white: Option<PlayerId>,
    pub black: Option<PlayerId>,
    pub matchup: Option<Matchup>,
    pub mode: Option<Mode>,
    pub rated: bool,
    pub self_play: bool,
    pub custom_start: bool,
    pub start_fen: Option<String>,
    pub clock: Option<ClockSpec>,
    pub white_clock_ms: Option<u64>,
    pub black_clock_ms: Option<u64>,
    pub status: GameStatus,
    pub result: Option<GameResult>,
    pub termination: Option<String>,
    pub voided_by: Option<PlayerId>,
    pub eco: Option<String>,
    pub opening_name: Option<String>,
    pub started_at: Option<WallMs>,
    pub ended_at: Option<WallMs>,
    pub white_time_used_ms: Option<u64>,
    pub black_time_used_ms: Option<u64>,
    pub total_active_ms: Option<u64>,
    pub analysis_used: bool,
    pub white_elo_before: Option<i32>,
    pub white_elo_after: Option<i32>,
    pub black_elo_before: Option<i32>,
    pub black_elo_after: Option<i32>,
    pub content_hash: Option<String>,
    pub journal_seq: u64,
}

/// A `MOVE` row. `ply` is 1-based (the schema checks `ply >= 1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveRow {
    pub ply: u32,
    pub san: String,
    pub uci: String,
    pub fen_after: String,
    pub think_ms: Option<u64>,
    /// Both clocks or neither (SF-18).
    pub clocks_ms: Option<(u64, u64)>,
    pub played_at: Option<WallMs>,
}

/// Structural keys only for `list_games` (D-44): no predicate on self-play, custom start or
/// void. `from` is the reset window (`started_at >= from`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameFilter {
    pub player: Option<PlayerId>,
    pub status: Option<GameStatus>,
    pub from: Option<WallMs>,
    pub limit: Option<u32>,
}

/// What `Store::open` reports (SDD 3.9.1; part B adds the recovered games).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReport {
    pub first_launch: bool,
    /// The schema version found before an upgrade (`None` on first launch or no upgrade).
    pub upgraded_from: Option<u32>,
    pub schema_version: u32,
    pub loose_permissions: bool,
}

/// Who this process is in `PROCESS_HEARTBEAT` (SDD 3.11.6): the owner string is
/// `<kind>:<pid>`, and the build version is what PLAT-5 compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    /// The front-end kind: `cli`, `server` or `desktop` by convention. A `:` is not allowed.
    pub kind: String,
    pub pid: u32,
    pub build_version: String,
}

impl ProcessIdentity {
    /// This process: the given kind and build version, the real pid. A `:` in `kind` becomes `_`.
    pub fn current(kind: &str, build_version: &str) -> ProcessIdentity {
        ProcessIdentity {
            kind: kind.replace(':', "_"),
            pid: std::process::id(),
            build_version: build_version.to_owned(),
        }
    }

    /// The `owner` column: `<kind>:<pid>`. Also the owner of a `GAME_LEASE` row.
    pub fn owner(&self) -> String {
        format!("{}:{}", self.kind, self.pid)
    }
}

/// A live process of the registry (`list_live_processes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveProcess {
    pub owner: String,
    pub kind: String,
    /// `None` when the owner text carries no pid (a row this build did not write).
    pub pid: Option<u32>,
    pub build_version: String,
    pub heartbeat_at: i64,
}

/// A `GAME_LEASE` row: display only, the journal `flock` decides (SDD 3.11.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseInfo {
    pub owner: String,
    pub heartbeat_at: i64,
}

/// The answer of `lease_beat` (SDD 3.7.2).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeaseBeat {
    /// The row is this owner's; its heartbeat was refreshed.
    Held,
    /// The row names another owner (a takeover): this process no longer owns the display row.
    Lost { owner: String },
    /// No lease row for the game. The `GAME` row's state is returned so a session that was
    /// voided elsewhere can build its `Voided` record without a second call (D-67 (iii), D-69:
    /// copied from the row, never `now`). The status is `in_progress` when the row was only
    /// deleted.
    RowGone {
        status: chess_domain::GameStatus,
        voided_by: Option<PlayerId>,
        content_hash: Option<String>,
        ended_at: Option<WallMs>,
    },
}

// ---- Part D2: game lifecycle writes and ratings (SDD 3.7.2, 3.11.4) ----

/// Every input of the finalize transaction, so a restore can rebuild it from the journal
/// (SDD 3.7.2). The three time totals come from the `GameEnd` record and are never recomputed
/// (D-63). `content_hash` is computed by the caller and stored as an opaque string (D-42).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizeRequest {
    pub game: GameId,
    pub result: GameResult,
    pub termination: String,
    pub ended_at: WallMs,
    pub white_time_used_ms: u64,
    pub black_time_used_ms: u64,
    pub total_active_ms: u64,
    pub eco: Option<String>,
    pub opening_name: Option<String>,
    pub analysis_used: bool,
    pub content_hash: String,
    pub final_journal_seq: u64,
}

/// One side's rating change on one track, as written to `RATING_HISTORY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatingChange {
    pub player: PlayerId,
    pub track: chess_domain::Track,
    pub before: i32,
    pub after: i32,
    pub k: i32,
}

/// The answer of `finalize_game`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FinalizeOutcome {
    /// The game is now `completed`. `white` and `black` are `None` for an unrated game
    /// (self-play, custom start, `rated = 0`).
    Finalized {
        white: Option<RatingChange>,
        black: Option<RatingChange>,
    },
    /// The game was already `completed`, `aborted` or `voided`: nothing was changed (SDD 3.11.4,
    /// architecture 7.4).
    AlreadyFinal,
}

/// The answer of `abort_game` and `void_unreadable_game`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EndOutcome {
    Applied,
    /// The game was already terminal: nothing was changed.
    AlreadyFinal,
}

/// Abort of a computer-against-computer game (the `Abort` journal record: `ended_at` and
/// `content_hash`). `final_journal_seq` is the `seq` of that record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbortRequest {
    pub game: GameId,
    pub ended_at: WallMs,
    pub content_hash: String,
    pub final_journal_seq: u64,
}

/// One game that `reset_player` voids, with the hash the caller computed for it (advisory,
/// D-67 (i)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidSpec {
    pub game: GameId,
    pub content_hash: String,
}

/// What `check_rating_invariant` found wrong with one Player and track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatingViolation {
    pub player: PlayerId,
    pub track: chess_domain::Track,
    pub kind: RatingViolationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RatingViolationKind {
    /// The track has no rating rows, or the first row belongs to a game (no initial point).
    NoInitialPoint,
    /// Row `index` (0-based, append order) does not start where the row before it ended.
    ChainBroken { index: u32 },
    /// A row without a game (a point) changes the rating.
    PointChangesRating { index: u32 },
    /// The last point plus the later deltas is not the stored rating (ELO-1).
    CurrentMismatch { expected: i32, found: i32 },
}

// ---- Part D3: the apply path (SDD 3.7.2, 3.11.3, 3.12) ----

/// What `Store::apply_records` did with a batch. A batch is never lost: it is in the journal
/// already, and either it is in the database now or it waits in the backlog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplyOutcome {
    /// The batch was committed.
    Applied,
    /// The database was busy or unavailable (or an earlier batch of the game waits): the batch
    /// is queued behind the earlier ones and retried on the timer (0.1 s doubling to 5 s).
    Queued,
}

/// How much waits in the apply backlog (SDD 3.12); `store_backlog` is the `StoreBacklog` event
/// condition: the oldest waiting batch is 30 s old or older (architecture 4.8). The caller reads
/// this after a `Queued` answer or on its own timer and emits the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct BacklogStatus {
    /// Batches waiting, over all games.
    pub pending_batches: usize,
    /// Games with at least one batch waiting.
    pub pending_games: usize,
    /// Games whose head batch failed with an error that a retry cannot cure (an invalid record):
    /// nothing behind it is applied. A parked game keeps the backlog non-empty.
    pub parked_games: usize,
    /// The age of the oldest waiting batch.
    pub oldest_age: std::time::Duration,
    /// `true` when `oldest_age` is at least 30 s.
    pub store_backlog: bool,
}

/// What a parked game failed with: the category of the error that no retry cures (D3 ruling
/// 2026-11-24). Only the category travels, never a row value (NFR-9, NFR-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParkCause {
    /// A record broke a rule of the store (`InvariantViolated`): a hostile or buggy journal.
    InvariantViolated,
    /// A Player or a game the batch names does not exist (`NotFound`). Registering it and
    /// registering it and calling `unpark_game`, then passing the whole journal again, cures it.
    NotFound,
    /// The database holds something this program did not write (`Corrupt`).
    Corrupt,
    /// A panic in the attempt (`Internal`).
    Internal,
    /// The game's queue reached its bound (256 batches, or the backlog 4 096 in all; ESTIMATE):
    /// the database has been unavailable for minutes. Nothing queued was dropped.
    QueueFull,
    /// Any other error that is not `Busy` or `Unavailable`.
    Other,
}

/// One game whose database updates are parked (D3 ruling 2026-11-24, `Store::parked_games`).
/// Its first waiting batch failed with an error that a retry cannot cure; nothing behind it is
/// applied (order), nothing is dropped, and later batches of the game are refused with
/// `StoreError::GameParked` until `Store::unpark_game` or the next start (which reconciles from
/// the journal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParkedGame {
    pub game: GameId,
    /// What the batch failed with.
    pub cause: ParkCause,
    /// Batches of the game that wait behind the failed one, plus the failed one.
    pub pending_batches: usize,
    /// How long ago the game was parked.
    pub parked_for: std::time::Duration,
}
