//! DD-ST-001 `Store`: one concrete, cheaply clonable handle (D-39; no `dyn Store`, no fake
//! store). Writes run on the store thread, reads on the read pool through `spawn_blocking`
//! (architecture 4.8). Every identity at this boundary is a `PlayerId` or `GameId` (TR-3, D-40).
//!
//! Part A methods: players, settings, ratings (read), games (start, read, list, moves).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chess_domain::{GameId, PlayerId, PlayerKind, RatingPoint, Track};
use rusqlite::Connection;

use crate::api::{
    AbortRequest, EndOutcome, FinalizeOutcome, FinalizeRequest, GameFilter, GameRecord, LeaseBeat,
    LeaseInfo, LiveProcess, MoveRow, NewComputer, NewGame, NewPlayer, OpenReport, PlayerRecord,
    ProcessIdentity, RatingViolation, VoidSpec,
};
use crate::datadir::DataDir;
use crate::error::StoreError;
use crate::journal::MoveJournal;
use crate::migrate::Migrator;
use crate::open::{open_blocking, open_write_connection};
use crate::proc::pid_alive;
use crate::readpool::ReadPool;
use crate::repo::{apply, coord, finalize, game, invariant, lifecycle, player, rating};
use crate::thread::{Applier, MonotonicClock, StoreThread, instant_clock};

mod d3;
mod d5;

/// The injected wall clock, Unix epoch milliseconds (IFR-ST-05).
pub type WallClock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// Default size of the read pool (SDD 3.12: 2 `ESTIMATE`).
const DEFAULT_READ_CONNECTIONS: usize = 2;

pub struct StoreOptions {
    pub data_dir: DataDir,
    pub clock: WallClock,
    pub migrator: Migrator,
    pub read_connections: usize,
    /// Who this process is in the registry (kind and build version, SDD 3.11.6).
    pub process: ProcessIdentity,
    /// Times the apply backlog (part D3). Monotonic milliseconds; a test injects its own.
    monotonic: MonotonicClock,
}

impl StoreOptions {
    pub fn new(data_dir: DataDir) -> StoreOptions {
        StoreOptions {
            data_dir,
            clock: Arc::new(system_wall_ms),
            migrator: Migrator::embedded(),
            read_connections: DEFAULT_READ_CONNECTIONS,
            process: ProcessIdentity::current("cli", env!("CARGO_PKG_VERSION")),
            monotonic: instant_clock(),
        }
    }

    /// Test support (part D3): the clock that times the apply backlog (0.1 s to 5 s retries,
    /// the 30 s `StoreBacklog` age, the `sync_backlog` timeout). The default is the system's
    /// monotonic clock.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_monotonic_clock(mut self, clock: MonotonicClock) -> StoreOptions {
        self.monotonic = clock;
        self
    }

    /// The front-end kind (`cli`, `server`, `desktop`) and the build version of this process.
    #[must_use]
    pub fn with_process(mut self, kind: &str, build_version: &str) -> StoreOptions {
        self.process = ProcessIdentity::current(kind, build_version);
        self
    }

    #[must_use]
    pub fn with_clock(mut self, clock: WallClock) -> StoreOptions {
        self.clock = clock;
        self
    }

    /// Test support: a synthetic migration list (SDD 3.11.5).
    #[must_use]
    pub fn with_migrator(mut self, migrator: Migrator) -> StoreOptions {
        self.migrator = migrator;
        self
    }
}

fn system_wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

#[derive(Debug)]
pub struct OpenedStore {
    pub store: Store,
    pub report: OpenReport,
}

struct Inner {
    // Field order matters for Drop: the writer thread joins first.
    writer: StoreThread,
    readers: Arc<ReadPool>,
    clock: WallClock,
    data_dir: DataDir,
    process: ProcessIdentity,
    store_id: Option<uuid::Uuid>,
    /// Part D5: set once `enqueue_failed_once_per_start` has run in this start.
    failed_jobs_requeued: std::sync::atomic::AtomicBool,
}

/// The handle `chess-app` holds. `Clone + Send + Sync`.
#[derive(Clone)]
pub struct Store {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").finish_non_exhaustive()
    }
}

impl Store {
    /// Opens (or creates) the store per SDD 3.9.1, part A. Blocking work runs in
    /// `spawn_blocking`. Must be called inside a tokio runtime.
    pub async fn open(options: StoreOptions) -> Result<OpenedStore, StoreError> {
        let StoreOptions {
            data_dir,
            clock,
            migrator,
            read_connections,
            process,
            monotonic,
        } = options;
        let dir = data_dir.clone();
        let now = clock();
        let who = process.clone();
        let opened = tokio::task::spawn_blocking(move || {
            open_blocking(&dir, &migrator, read_connections, now, &who)
        })
        .await
        .map_err(|_| StoreError::Internal)??;
        let db = data_dir.db_path();
        let reopen_db = db.clone();
        let applier: Applier =
            Arc::new(move |conn, game, records| apply::apply_records(conn, game, records, &db));
        let writer = StoreThread::spawn(
            opened.write,
            Box::new(move || open_write_connection(&reopen_db)),
            applier,
            monotonic,
        )?;
        Ok(OpenedStore {
            store: Store {
                inner: Arc::new(Inner {
                    writer,
                    readers: Arc::new(ReadPool::new(opened.readers)),
                    clock,
                    store_id: data_dir.read_store_id()?.map(|id| id.store_uuid),
                    data_dir,
                    process,
                    failed_jobs_requeued: std::sync::atomic::AtomicBool::new(false),
                }),
            },
            report: opened.report,
        })
    }

    pub fn data_dir(&self) -> &DataDir {
        &self.inner.data_dir
    }

    async fn write<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, StoreError> + Send + 'static,
    {
        self.inner.writer.run(f).await
    }

    /// A write that first applies the queued batches of `games` (D3-09): the journal is the
    /// truth, and a live terminal command must not run ahead of earlier records of its game.
    async fn write_after_flush<T, F>(&self, games: Vec<GameId>, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&mut rusqlite::Connection) -> Result<T, StoreError> + Send + 'static,
    {
        self.inner.writer.run_flushed(games, f).await
    }

    async fn read<T, F>(&self, f: F) -> Result<T, StoreError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let pool = Arc::clone(&self.inner.readers);
        tokio::task::spawn_blocking(move || pool.with(f))
            .await
            .map_err(|_| StoreError::Internal)?
    }

    // ---- Players (W) ----

    /// `PLAYER` + `PREFERENCES` + one rating row per track. `NameTaken` on a repeated name.
    pub async fn register_player(&self, new: NewPlayer) -> Result<PlayerRecord, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        self.write(move |conn| player::register_player(conn, &new, clock()))
            .await
    }

    /// The roster batch: all N or none. `RosterIndexTaken` when `(roster_seed, roster_index)`
    /// exists.
    pub async fn insert_computers(
        &self,
        batch: Vec<NewComputer>,
    ) -> Result<Vec<PlayerRecord>, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        self.write(move |conn| player::insert_computers(conn, &batch, clock()))
            .await
    }

    pub async fn rename_player(
        &self,
        id: PlayerId,
        new_name: String,
    ) -> Result<PlayerRecord, StoreError> {
        self.write(move |conn| player::rename_player(conn, id, &new_name))
            .await
    }

    pub async fn archive_player(&self, id: PlayerId) -> Result<PlayerRecord, StoreError> {
        self.write(move |conn| player::archive_player(conn, id))
            .await
    }

    pub async fn update_preferences(&self, id: PlayerId, json: String) -> Result<(), StoreError> {
        self.write(move |conn| player::update_preferences(conn, id, &json))
            .await
    }

    pub async fn set_app_setting(&self, key: String, value: String) -> Result<(), StoreError> {
        self.write(move |conn| player::set_app_setting(conn, &key, &value))
            .await
    }

    // ---- Players (R) ----

    /// Every stored `roster_index`, ascending (D-46). The caller derives the next `k` per class
    /// with `RosterGen::first_k_after`.
    pub async fn roster_indexes(&self) -> Result<Vec<u32>, StoreError> {
        self.read(player::roster_indexes).await
    }

    pub async fn get_player(&self, id: PlayerId) -> Result<PlayerRecord, StoreError> {
        self.read(move |conn| player::get_player(conn, id)).await
    }

    pub async fn list_players(
        &self,
        kind: Option<PlayerKind>,
        include_archived: bool,
    ) -> Result<Vec<PlayerRecord>, StoreError> {
        self.read(move |conn| player::list_players(conn, kind, include_archived))
            .await
    }

    pub async fn find_player_by_name(
        &self,
        name: String,
    ) -> Result<Option<PlayerRecord>, StoreError> {
        self.read(move |conn| player::find_player_by_name(conn, &name))
            .await
    }

    pub async fn get_preferences(&self, id: PlayerId) -> Result<Option<String>, StoreError> {
        self.read(move |conn| player::get_preferences(conn, id))
            .await
    }

    pub async fn get_app_setting(&self, key: String) -> Result<Option<String>, StoreError> {
        self.read(move |conn| player::get_app_setting(conn, &key))
            .await
    }

    pub async fn rating_history(
        &self,
        id: PlayerId,
        track: Track,
    ) -> Result<Vec<RatingPoint>, StoreError> {
        self.read(move |conn| rating::rating_history(conn, id, track))
            .await
    }

    // ---- Games ----

    /// `GAME` + slot rows. Idempotent on `GameId`. `SlotTaken` when a slot is held.
    pub async fn start_game(&self, new: NewGame) -> Result<GameRecord, StoreError> {
        self.write(move |conn| game::start_game(conn, &new)).await
    }

    pub async fn set_analysis_used(&self, id: GameId) -> Result<(), StoreError> {
        self.write(move |conn| game::set_analysis_used(conn, id))
            .await
    }

    pub async fn get_game(&self, id: GameId) -> Result<GameRecord, StoreError> {
        self.read(move |conn| game::get_game(conn, id)).await
    }

    pub async fn list_games(&self, filter: GameFilter) -> Result<Vec<GameRecord>, StoreError> {
        self.read(move |conn| game::list_games(conn, &filter)).await
    }

    pub async fn list_unfinished(&self, id: PlayerId) -> Result<Vec<GameRecord>, StoreError> {
        self.read(move |conn| game::list_unfinished(conn, id)).await
    }

    pub async fn game_moves(&self, id: GameId) -> Result<Vec<MoveRow>, StoreError> {
        self.read(move |conn| game::game_moves(conn, id)).await
    }

    // ---- Game lifecycle writes and ratings (part D2, SDD 3.11.4) ----

    /// Completes a game: Elo settlement and rating rows for a rated game, the terminal `GAME`
    /// columns, slot and lease deletion and a queued analysis job, in one transaction.
    /// `AlreadyFinal` for a game that is already terminal.
    pub async fn finalize_game(
        &self,
        request: FinalizeRequest,
    ) -> Result<FinalizeOutcome, StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write_after_flush(vec![request.game], move |conn| {
            finalize::finalize_game(conn, &request, &ctx)
        })
        .await
    }

    /// Aborts a game (the finalize without rating work); sets the hash.
    pub async fn abort_game(&self, request: AbortRequest) -> Result<EndOutcome, StoreError> {
        let ctx = self.inner.data_dir.db_path();
        self.write_after_flush(vec![request.game], move |conn| {
            finalize::abort_game(conn, &request, &ctx)
        })
        .await
    }

    /// Voids a game whose journal cannot be read (DAT-7). `content_hash` is the hash of whatever
    /// moves are readable (zero moves allowed); `ended_at` is the store clock (the void time).
    pub async fn void_unreadable_game(
        &self,
        game: GameId,
        content_hash: String,
    ) -> Result<EndOutcome, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        let ctx = self.inner.data_dir.db_path();
        self.write_after_flush(vec![game], move |conn| {
            lifecycle::void_unreadable_game(conn, game, &content_hash, clock(), &ctx)
        })
        .await
    }

    /// Resets a human Player and voids the listed games in one transaction; the commit time is
    /// written to `reset_at` and to `GAME.ended_at` of each voided game. An unfinished game of
    /// the Player that is not listed rolls back with `UnlistedUnfinishedGames`.
    pub async fn reset_player(
        &self,
        player: PlayerId,
        voids: Vec<VoidSpec>,
        default_preferences_json: String,
    ) -> Result<(), StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        let ctx = self.inner.data_dir.db_path();
        let games = voids.iter().map(|spec| spec.game).collect();
        self.write_after_flush(games, move |conn| {
            lifecycle::reset_player(
                conn,
                player,
                &voids,
                &default_preferences_json,
                clock(),
                &ctx,
            )
        })
        .await
    }

    /// ELO-1 for the given Players, both tracks; an empty answer means the invariant holds.
    pub async fn check_rating_invariant(
        &self,
        players: Vec<PlayerId>,
    ) -> Result<Vec<RatingViolation>, StoreError> {
        self.read(move |conn| invariant::check_rating_invariant(conn, &players))
            .await
    }

    /// The journal creator for this store (IFP-ST-02). The `store.id` of an existing store that
    /// lost it is not recreated here (F-A-FIND-007 is open for Brian), so the answer is
    /// `Corrupt`: no journal may be written under an identity the store cannot name.
    pub fn journal(&self) -> Result<MoveJournal, StoreError> {
        self.inner
            .store_id
            .map(|id| MoveJournal::new(self.inner.data_dir.clone(), id))
            .ok_or(StoreError::Corrupt)
    }

    // ---- Coordination (SDD 3.11.6) ----

    /// This process's identity in the registry.
    pub fn process(&self) -> &ProcessIdentity {
        &self.inner.process
    }

    /// The owner text (`<kind>:<pid>`) to pass to the lease methods.
    pub fn owner(&self) -> String {
        self.inner.process.owner()
    }

    /// Registers this process (`BEGIN IMMEDIATE`: delete stale rows, check, upsert). Returns the
    /// other live processes; `OtherVersionRunning` when one has another build version. The open
    /// already did this once; call it again only to re-register.
    pub async fn register_process(&self) -> Result<Vec<LiveProcess>, StoreError> {
        let (me, clock) = (self.inner.process.clone(), Arc::clone(&self.inner.clock));
        self.write(move |conn| coord::register(conn, &me, clock(), &pid_alive))
            .await
    }

    /// The 10 s refresh (`PROCESS_HEARTBEAT_EVERY`); the caller drives the timer.
    pub async fn process_heartbeat(&self) -> Result<(), StoreError> {
        let (me, clock) = (self.inner.process.clone(), Arc::clone(&self.inner.clock));
        self.write(move |conn| coord::heartbeat(conn, &me, clock(), &pid_alive))
            .await
    }

    pub async fn deregister_process(&self) -> Result<(), StoreError> {
        let me = self.inner.process.clone();
        self.write(move |conn| coord::deregister(conn, &me)).await
    }

    /// Every live process, this one included.
    pub async fn list_live_processes(&self) -> Result<Vec<LiveProcess>, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        self.read(move |conn| coord::list_live(conn, clock(), &pid_alive))
            .await
    }

    /// Creates or takes over the display row of a game (the caller holds the journal lock).
    /// Returns the replaced row when it named another owner.
    pub async fn lease_upsert(
        &self,
        game: GameId,
        owner: String,
    ) -> Result<Option<LeaseInfo>, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        self.write(move |conn| coord::lease_upsert(conn, game, &owner, clock()))
            .await
    }

    /// Deletes the row only if `owner` holds it.
    pub async fn lease_delete(&self, game: GameId, owner: String) -> Result<bool, StoreError> {
        self.write(move |conn| coord::lease_delete(conn, game, &owner))
            .await
    }

    /// The 2 s refresh (`LEASE_BEAT_EVERY`).
    pub async fn lease_beat(&self, game: GameId, owner: String) -> Result<LeaseBeat, StoreError> {
        let clock = Arc::clone(&self.inner.clock);
        self.write(move |conn| coord::lease_beat(conn, game, &owner, clock()))
            .await
    }

    /// The display row of a game: who owns it, for `GameLeased{owner_kind}`.
    pub async fn lease_owner(&self, game: GameId) -> Result<Option<LeaseInfo>, StoreError> {
        self.read(move |conn| coord::lease_row(conn, game)).await
    }
}
