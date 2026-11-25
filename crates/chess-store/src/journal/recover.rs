//! DD-ST-019 `StartupRecovery` and `MoveJournal::open_for_resume` (SDD 3.7.3, 3.9.1, 3.9.2,
//! 3.11.3; D-65, D-66, D-72, D-83, D-87, D-110, D-152, D-153). Part D4.
//!
//! **One routine, two callers.** `recover_game` takes one game's journal lock, repairs the file
//! windows of the rebase and of `close_final` (B-R03, B-R05), reads the journal, compares it
//! with the store, and brings the two together. A resume (`open_for_resume`) keeps the lock and
//! returns a writer; the startup scan (`startup`) releases it. The scan appends nothing (D-58):
//! the only bytes it ever writes are the truncation of a torn tail and a rebase, which is a
//! rewrite of the file from records the journal and the store already hold.
//!
//! **Cases** (per game, with the lock held):
//!
//! | file | row | what happens |
//! |---|---|---|
//! | none, no `.rebase` | `in_progress`, no rows, `journal_seq` 0 | resume creates a header-only journal (the app appends `Start`, 3.9.2); the scan does nothing |
//! | none, no `.rebase` | `in_progress`, rows or `journal_seq` > 0 | **rebase from the store** with an empty intact run (D-66, RR-A-1 (a)) |
//! | none | final | nothing to do |
//! | intact | `in_progress` | torn tail truncated; reconcile plan applied if not empty; then the **whole journal in one `apply_records` call** (D3-10); a terminal last record finalizes (D-72 order at startup) |
//! | damaged, or `journal_seq` beyond the intact run (D-66, `UndecodableTail` too) | `in_progress` | rebase: build, database step, swap (3.11.3) |
//! | damaged | final | moved to `damaged/`, **not** rebased, rows untouched (RR-A-1 (b)) |
//! | intact | final | nothing is applied; a journal that ends in a terminal record is moved to `done/` by the scan |
//! | no row | | a resume is `NotFound`; the scan leaves the file alone, except a file that is only a header, which is removed (STATE-ST-01) |
//!
//! Every database call goes through the store thread **after the queued batches of the game**
//! (D3-09). A batch the store refuses parks the game (D3 ruling 2026-11-24): the scan reports
//! `Parked` and goes on; it never retries in a loop. The documented repair is
//! `Store::unpark_game`, then a resume or the next start, which passes the whole journal again.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chess_domain::{GameId, GameStatus, PlayerKind};
use uuid::Uuid;

use crate::api::{ApplyOutcome, ParkCause};
use crate::error::{Entity, JournalError, StoreError, WriteCause};
use crate::fault;
use crate::journal::files::{
    Settled, finish_close_final, journal_path, move_aside_damaged, rebase_path, same_file,
    settle_rebase_files_at, sync_dir,
};
use crate::journal::format::{
    ClockText, HEADER_LEN, JournalRecord, Kind, Payload, SeatKind, StartPayload, UuidText,
    decode_header, encode_header, matchup_code,
};
use crate::journal::reader::{DamageKind, DamagedInfo, scan_bytes};
use crate::journal::rebase::{RebaseInput, build_rebased, swap, write_rebase_file};
use crate::journal::reconcile::{
    ClockPair, Folded, Plan, ReconcileInput, RunSource, StoredMove, apply_plan, fold, reconcile,
    reconciled_clocks,
};
use crate::journal::writer::{JournalWriter, MoveJournal, lock_with_grace};
use crate::repo::resume::ResumeRow;
use crate::store::Store;

// ------------------------------------------------------------------------------------ types

/// Why `open_for_resume` failed: the journal's side (`JournalError`, SDD 3.7.3) or the
/// database's (`StoreError`). The SDD signature `Result<_, JournalError>` has no room for a
/// database error, so the two are carried side by side (reported as an SDD conflict).
#[derive(Debug, thiserror::Error)]
pub enum ResumeError {
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// What recovery did to one game (`Recovered.report`, SDD 3.7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    /// A bad tail with nothing intact after it was dropped (never acknowledged).
    pub torn_tail_dropped: bool,
    /// Damage: a damaged middle, an undecodable CRC-valid tail (CH-5 B), an unreadable header,
    /// or the store ahead of the journal (D-66). The caller reports `JournalUnreadable`.
    pub damaged: Option<DamagedInfo>,
    /// The moves in force after recovery.
    pub recovered_moves: u32,
    pub source: RunSource,
    /// The game continues on a rebased journal (3.11.3).
    pub rebased: bool,
    /// Where the damaged file went: `journal/damaged/<uuid>.<k>.jnl`, or the file that was moved
    /// there for a final game (RR-A-1 (b)).
    pub moved_aside: Option<PathBuf>,
    /// A header-only journal was created for a row that had none (3.9.2): the app appends
    /// `Start` as record 1 with `wall_ms = GAME.started_at`.
    pub created_empty: bool,
}

/// What a resume returns (SDD 3.7.3): the journal's intact records, the moves and clocks in
/// force, and the report. `moves` are `MOVE.ply` one-based (the shape of the reconcile function,
/// not the SDD's unnamed `MoveRecord`); `clocks` is the pair or `None` for an untimed game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    pub records: Vec<JournalRecord>,
    pub moves: Vec<StoredMove>,
    pub clocks: ClockPair,
    pub report: RecoveryReport,
}

/// What the startup scan did with one journal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryOutcome {
    /// The journal ended in `GameEnd`, `Abort` or `Voided`; the store transaction committed and
    /// the journal moved to `done/` (D-72 order for the `GameEnd`s).
    Finalized,
    /// An interrupted game: the rows were brought to the journal's state (journal wins) and the
    /// journal is kept for the resume.
    Reconciled { recovered_moves: u32 },
    /// The journal was damaged, or behind the store (D-66): the game continues on a rebased
    /// journal; the old file is in `damaged/`. The app reports `JournalUnreadable`.
    Rebased {
        recovered_moves: u32,
        damaged: DamagedInfo,
    },
    /// A final game's journal was damaged: moved to `damaged/`, not rebased, rows unchanged
    /// (RR-A-1 (b)). The app reports `JournalUnreadable`.
    MovedAside { damaged: Option<DamagedInfo> },
    /// The game was already final; its journal went to `done/` (a crash after the commit).
    ClosedLeftover,
    /// B-R03: the journal existed under two names (`journal/` and `done/`); the old name was
    /// removed.
    FinishedClose,
    /// The row is final (`voided`, `completed`, ...) but the journal does not end in the
    /// terminal record: the app's repair appends it (3.11.3, the scan never appends).
    NeedsRecord,
    /// The journal's lock is held by a live process; nothing was touched.
    Held,
    /// The journal names a game that has no row; the file was left alone.
    NoGameRow,
    /// A header-only journal with no row was removed (STATE-ST-01).
    RemovedEmpty,
    /// The database was busy: the batch waits in the backlog and the journal stays.
    Queued,
    /// The store refused the journal's records and parked the game. Treated as damage by the
    /// caller; the repair is `unpark_game` and a resume.
    Parked { cause: Option<ParkCause> },
    /// Anything else, by category (never a path).
    Failed { error: String },
    /// An incomplete `.rebase` with no journal was moved aside (B-R05); the game is rebuilt from
    /// the store (the outcome of that rebuild follows as `Rebased`).
    IncompleteRebaseMovedAside,
    /// `done/<uuid>.jnl` holds a file that is not this journal (a different file, or anything
    /// that is not a regular file, such as a symbolic link), so `close_final` can never finish
    /// for this game and a person has to look (FIND-D4R-04, B-R03: "a different file is damage,
    /// not a retry"). The scan touched nothing: both names stay, the rows are as they were, and
    /// a resume is **not** blocked (it reads the journal under `journal/` as usual).
    DoneNameCollision,
}

/// One game's line in the startup report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredGame {
    pub game: GameId,
    pub outcome: RecoveryOutcome,
}

/// What the startup recovery reports to `Store::open` (`OpenReport.recovery`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartupReport {
    pub games: Vec<RecoveredGame>,
    /// The journal directory could not be listed, or the store has no identity: nothing was
    /// recovered.
    pub scan_failed: bool,
}

impl StartupReport {
    pub fn outcome_of(&self, game: GameId) -> Option<&RecoveryOutcome> {
        self.games
            .iter()
            .find(|g| g.game == game)
            .map(|g| &g.outcome)
    }
}

// ------------------------------------------------------------------------- the shared routine

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    Resume,
    Startup,
}

/// The journal file under our lock, or nothing.
pub(crate) enum Holder {
    None,
    File {
        file: File,
        path: PathBuf,
        acked_len: u64,
        next_seq: u64,
    },
    Writer(JournalWriter),
}

impl Holder {
    fn path(&self) -> Option<PathBuf> {
        match self {
            Holder::None => None,
            Holder::File { path, .. } => Some(path.clone()),
            Holder::Writer(writer) => Some(writer.path().to_path_buf()),
        }
    }

    fn into_writer(self) -> Option<JournalWriter> {
        match self {
            Holder::None => None,
            Holder::File {
                file,
                path,
                acked_len,
                next_seq,
            } => Some(JournalWriter::from_locked(file, path, acked_len, next_seq)),
            Holder::Writer(writer) => Some(writer),
        }
    }
}

/// What happened, as the startup scan needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fate {
    NoRow,
    RemovedEmpty,
    /// Final row, no journal file.
    NoJournalFinal,
    /// In-progress row with no journal, nothing stored (a resume creates the header).
    EmptyRow,
    Interrupted,
    /// The last record is terminal and the store holds it: the journal can be closed.
    Terminal(ApplyOutcome),
    FinalClosable,
    FinalNeedsRecord,
    Rebased,
    MovedAside,
    FinishedClose,
    IncompleteRebaseMovedAside,
    DoneCollision,
}

pub(crate) struct Done {
    holder: Holder,
    recovered: Recovered,
    fate: Fate,
}

fn io_err(e: &io::Error) -> JournalError {
    JournalError::write_failed(e)
}

/// Opens `path` read-write, takes its lock (with the grace of B-R04) and checks that the name
/// still is that file. `Ok(None)`: no such file.
fn lock_open(path: &Path) -> Result<Option<File>, JournalError> {
    let file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_err(&e)),
    };
    match lock_with_grace(&file) {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => return Err(JournalError::Locked { owner_kind: None }),
        Err(fs::TryLockError::Error(e)) => return Err(io_err(&e)),
    }
    // A holder that finished (`close_final`) released the lock after it moved the file: this
    // descriptor then names a file that is no longer at `path`.
    let same = {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(path), file.metadata()) {
            (Ok(named), Ok(held)) => named.dev() == held.dev() && named.ino() == held.ino(),
            _ => false,
        }
    };
    if same {
        Ok(Some(file))
    } else {
        Err(JournalError::Locked { owner_kind: None })
    }
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// What `journal/done/<uuid>.jnl` is, next to a journal that exists under `journal/` (B-R03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoneState {
    /// No journal under `journal/`, or no `done/` name.
    Absent,
    /// The same regular file under both names (`close_final` died between link and removal).
    SameFile,
    /// Another file, or a name that is not a regular file (a symbolic link, FIND-D4R-03).
    Different,
}

struct Prepared {
    /// The file whose lock we hold: `<uuid>.jnl`, else `<uuid>.jnl.rebase`, else nothing.
    file: Option<File>,
    done: DoneState,
}

/// The file-only prologue (blocking): lock the journal (or the `.rebase`) and look at `done/`.
/// Nothing is moved or removed here: what to do with the files depends on the row, which the
/// caller reads next, under this lock (FIND-D4R-04, FIND-D4R-05).
fn prepare(jdir: &Path, id: Uuid) -> Result<Prepared, JournalError> {
    let jnl = journal_path(jdir, id);
    let reb = rebase_path(jdir, id);
    let target = if exists(&jnl) {
        Some(jnl.clone())
    } else if exists(&reb) {
        Some(reb)
    } else {
        None
    };
    if target.is_none() {
        // Test seam (FIND-D4R-01): the decision "no file" is made here; what follows is acted on
        // later, possibly after another process has built the journal.
        fault::check("journal.resume.no_file_seen", jdir).map_err(|e| io_err(&e))?;
    }
    let file = match &target {
        Some(path) => lock_open(path)?,
        None => None,
    };
    // B-R03: the journal under two names (`close_final` died between the link and the removal).
    let done_path = jdir.join("done").join(format!("{}.jnl", id.hyphenated()));
    let done = if file.is_some() && exists(&jnl) && exists(&done_path) {
        if same_file(&jnl, &done_path).map_err(|e| io_err(&e))? {
            DoneState::SameFile
        } else {
            DoneState::Different
        }
    } else {
        DoneState::Absent
    };
    Ok(Prepared { file, done })
}

/// What the file steps after the row is known leave: the lock, and the bytes of the journal.
struct Settle {
    settled: Settled,
    file: Option<File>,
    bytes: Option<Vec<u8>>,
}

/// Repairs the windows of the rebase (B-R05) and reads the journal. `journal_seq` is the row's
/// value for an in-progress game (the completeness witness of a `.rebase`), `None` otherwise.
///
/// **Only under a lock.** With no file in our hands there is nothing to repair: a `.rebase` or a
/// journal that shows up now belongs to a process that is building it, and moving, removing or
/// installing it would destroy that work (FIND-D4R-01; the row is read between the decision
/// "no file" and this call, which is long enough for another resumer to run its whole rebase).
fn settle_and_read(
    jdir: &Path,
    id: Uuid,
    store: Uuid,
    journal_seq: Option<u64>,
    mut file: Option<File>,
) -> Result<Settle, JournalError> {
    let settled = if file.is_some() {
        settle_rebase_files_at(jdir, id, store, journal_seq).map_err(|e| io_err(&e))?
    } else {
        Settled::Nothing
    };
    if matches!(settled, Settled::MovedAsideIncomplete { .. }) {
        file = None;
    }
    let bytes = match &file {
        Some(_) => match fs::read(journal_path(jdir, id)) {
            Ok(bytes) => Some(bytes),
            // We locked a `.rebase` that its creator then removed (it lost the race for the
            // lock of its own new file, or gave up): there is no file under our lock any more.
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                file = None;
                None
            }
            Err(e) => return Err(io_err(&e)),
        },
        None => None,
    };
    Ok(Settle {
        settled,
        file,
        bytes,
    })
}

/// The header of a journal that names this game and this store.
fn header_names(bytes: &[u8], id: Uuid, store: Uuid) -> bool {
    decode_header(bytes).is_ok_and(|header| header.game == id && header.store == store)
}

/// The intact part of a journal as the recovery sees it.
struct View {
    records: Vec<JournalRecord>,
    /// The end of the intact run, as a file offset (`HEADER_LEN` when there is no record).
    intact_end: usize,
    torn: bool,
    damaged: Option<DamagedInfo>,
}

impl View {
    fn empty() -> View {
        View {
            records: Vec::new(),
            intact_end: HEADER_LEN,
            torn: false,
            damaged: None,
        }
    }

    fn last_seq(&self) -> u64 {
        self.records.last().map_or(0, |r| r.seq)
    }
}

/// An unreadable header is damage with an empty intact run (RR-A-1 (a)); another store, or a
/// newer journal version, is refused.
fn view_of(bytes: &[u8], id: Uuid, store: Uuid) -> Result<View, JournalError> {
    match scan_bytes(bytes, id, store) {
        Ok(scan) => Ok(View {
            records: scan.records,
            intact_end: scan.intact_end,
            torn: scan.torn_tail_dropped,
            damaged: scan.damaged,
        }),
        Err(JournalError::Unreadable) => Ok(View {
            records: Vec::new(),
            intact_end: HEADER_LEN,
            torn: false,
            damaged: Some(DamagedInfo {
                kind: DamageKind::Middle {
                    at: 0,
                    resumes_at: 0,
                },
            }),
        }),
        Err(other) => Err(other),
    }
}

fn is_terminal(record: &JournalRecord) -> bool {
    matches!(record.kind(), Kind::GameEnd | Kind::Abort | Kind::Voided)
}

fn seat(kind: Option<PlayerKind>) -> Result<SeatKind, ResumeError> {
    match kind {
        Some(PlayerKind::Human) => Ok(SeatKind::Human),
        Some(PlayerKind::Computer) => Ok(SeatKind::Computer),
        None => Err(StoreError::Corrupt.into()),
    }
}

/// The `Start` record of a row (3.9.2, RR-A-1 (a)): the frozen `rated`, `self_play` and
/// `custom_start`, the players, matchup, mode, `start_fen` and clock, `wall_ms = started_at`.
fn start_from_row(row: &ResumeRow, id: Uuid) -> Result<(StartPayload, i64), ResumeError> {
    let record = &row.record;
    let (Some(white), Some(black), Some(mode), Some(started)) =
        (record.white, record.black, record.mode, record.started_at)
    else {
        return Err(StoreError::Corrupt.into());
    };
    let (white_kind, black_kind) = (seat(row.white_kind)?, seat(row.black_kind)?);
    let clock_spec = match record.clock {
        Some(spec) => Some(ClockText::parse(&spec.to_spec_text()).ok_or(StoreError::Corrupt)?),
        None => None,
    };
    Ok((
        StartPayload {
            game: UuidText(id),
            white: UuidText(white.as_uuid()),
            black: UuidText(black.as_uuid()),
            white_kind,
            black_kind,
            matchup: matchup_code(white_kind, black_kind).to_owned(),
            mode: mode.as_str().to_owned(),
            custom_start: record.custom_start,
            start_fen: record.start_fen.clone(),
            clock_spec,
            rated: record.rated,
            self_play: record.self_play,
        },
        started.0,
    ))
}

fn clocks_of(row: &ResumeRow) -> ClockPair {
    row.record.white_clock_ms.zip(row.record.black_clock_ms)
}

fn report_of(view: &View, moves: &[StoredMove], source: RunSource) -> RecoveryReport {
    RecoveryReport {
        torn_tail_dropped: view.torn,
        damaged: view.damaged,
        recovered_moves: u32::try_from(moves.len()).unwrap_or(u32::MAX),
        source,
        rebased: false,
        moved_aside: None,
        created_empty: false,
    }
}

fn join<T>(result: Result<T, tokio::task::JoinError>) -> Result<T, ResumeError> {
    result.map_err(|_| ResumeError::Store(StoreError::Internal))
}

/// How many times a resume that found another process building the journal starts again (the
/// second look finds the file and takes its lock, or is told `Locked`).
const RACE_ATTEMPTS: u32 = 3;

/// One game, lock held (SDD 3.11.3; the table in the module comment). `deadline` bounds the
/// total wait of the startup scan for the database (FIND-D4R-07); a resume passes `None`.
pub(crate) async fn recover_game(
    store: &Store,
    game: GameId,
    intent: Intent,
    deadline: Option<Instant>,
) -> Result<Done, ResumeError> {
    for _ in 0..RACE_ATTEMPTS {
        if let Some(done) = recover_game_once(store, game, intent, deadline).await? {
            return Ok(done);
        }
    }
    Err(JournalError::Locked { owner_kind: None }.into())
}

/// The connection's normal lock wait (`open.rs`, 5 s): one step never waits longer for the
/// database than this, whatever the budget has left.
const NORMAL_WAIT: Duration = Duration::from_secs(5);

/// How long the next database step of the startup scan may wait: `None` without a deadline
/// (the connection's normal wait), otherwise the time left, at most `NORMAL_WAIT`; `Busy` when
/// the budget is spent.
fn wait_left(deadline: Option<Instant>) -> Result<Option<Duration>, StoreError> {
    let Some(deadline) = deadline else {
        return Ok(None);
    };
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(StoreError::Busy);
    }
    Ok(Some(left.min(NORMAL_WAIT)))
}

/// `Ok(None)`: another process built the journal while this one was deciding that there was none
/// (FIND-D4R-01); nothing of this attempt is left on disk and the caller starts again.
#[allow(
    clippy::too_many_lines,
    reason = "one routine with one match per row of the case table in the module comment"
)]
async fn recover_game_once(
    store: &Store,
    game: GameId,
    intent: Intent,
    deadline: Option<Instant>,
) -> Result<Option<Done>, ResumeError> {
    let store_uuid = store.store_uuid()?;
    let data = store.data_dir().clone();
    let jdir = data.journal_dir();
    let id = game.as_uuid();

    let prep = {
        let jdir = jdir.clone();
        join(tokio::task::spawn_blocking(move || prepare(&jdir, id)).await)??
    };
    let no_row = || StoreError::NotFound {
        entity: Entity::Game,
        id,
    };
    if intent == Intent::Startup && prep.done == DoneState::Different {
        // FIND-D4R-04: nothing is touched and the game is not made unresumable; a resume goes on
        // from `journal/`. The lock is released with `prep`.
        return Ok(Some(Done {
            holder: Holder::None,
            recovered: empty_recovered(),
            fate: Fate::DoneCollision,
        }));
    }

    let Some(row) = store.resume_row(game).await? else {
        if intent == Intent::Resume {
            return Err(no_row().into());
        }
        // The scan: a file that is only a header, for a game with no row, is removed (only when
        // that header names this game and this store, FIND-D4R-12).
        let settle = {
            let (jdir, file) = (jdir.clone(), prep.file);
            join(
                tokio::task::spawn_blocking(move || {
                    settle_and_read(&jdir, id, store_uuid, None, file)
                })
                .await,
            )??
        };
        let mut fate = Fate::NoRow;
        if let (Some(_), Some(bytes)) = (&settle.file, &settle.bytes)
            && bytes.len() == HEADER_LEN
            && header_names(bytes, id, store_uuid)
        {
            let jdir = jdir.clone();
            join(
                tokio::task::spawn_blocking(move || {
                    fs::remove_file(journal_path(&jdir, id))
                        .and_then(|()| sync_dir(&jdir))
                        .map_err(|e| io_err(&e))
                })
                .await,
            )??;
            fate = Fate::RemovedEmpty;
        }
        return Ok(Some(Done {
            holder: Holder::None,
            recovered: empty_recovered(),
            fate,
        }));
    };

    let final_row = row.record.status != GameStatus::InProgress;
    let db_seq = row.record.journal_seq;
    let stored_clocks = clocks_of(&row);
    let nothing_stored = db_seq == 0 && row.moves.is_empty();

    // B-R03, finished: the same regular file under both names and the game is over, so the
    // close did happen. For a game still in progress the second name proves nothing: both names
    // stay and the recovery goes on (FIND-D4R-04); the close that follows the final commit
    // finishes the move.
    if prep.done == DoneState::SameFile && final_row && prep.file.is_some() {
        let file = prep.file;
        let jdir = jdir.clone();
        join(
            tokio::task::spawn_blocking(move || {
                let removed = fs::remove_file(journal_path(&jdir, id))
                    .and_then(|()| sync_dir(&jdir))
                    .map_err(|e| io_err(&e));
                drop(file);
                removed
            })
            .await,
        )??;
        return Ok(Some(Done {
            holder: Holder::None,
            recovered: empty_recovered(),
            fate: Fate::FinishedClose,
        }));
    }

    // The `.rebase` windows (B-R05) are settled now that the row is known: for a game in
    // progress, a `.rebase` that ends before the store's `journal_seq` was cut (FIND-D4R-05).
    let Settle {
        settled,
        file,
        bytes,
    } = {
        let (jdir, file) = (jdir.clone(), prep.file);
        let floor = (!final_row).then_some(db_seq);
        join(
            tokio::task::spawn_blocking(move || {
                settle_and_read(&jdir, id, store_uuid, floor, file)
            })
            .await,
        )??
    };
    let incomplete_aside = matches!(settled, Settled::MovedAsideIncomplete { .. });

    // ---- no journal file ----
    let Some(file) = file else {
        let view = View::empty();
        if final_row {
            return Ok(Some(Done {
                holder: Holder::None,
                recovered: recovered_from_row(&row, &view, RunSource::Store),
                fate: Fate::NoJournalFinal,
            }));
        }
        if nothing_stored {
            // 3.9.2: a row with no journal is a game with zero moves from `start_fen`.
            if intent == Intent::Startup {
                return Ok(Some(Done {
                    holder: Holder::None,
                    recovered: recovered_from_row(&row, &view, RunSource::Both),
                    fate: Fate::EmptyRow,
                }));
            }
            let journal = MoveJournal::new(data, store_uuid);
            let created = join(tokio::task::spawn_blocking(move || journal.create(&game)).await)?;
            let writer = match created {
                Ok(writer) => writer,
                // The name appeared after our decision: someone else created it (FIND-D4R-01).
                Err(JournalError::WriteFailed {
                    cause: WriteCause::Io(io::ErrorKind::AlreadyExists),
                }) => return Ok(None),
                Err(other) => return Err(other.into()),
            };
            let mut recovered = recovered_from_row(&row, &view, RunSource::Both);
            recovered.report.created_empty = true;
            return Ok(Some(Done {
                holder: Holder::Writer(writer),
                recovered,
                fate: Fate::EmptyRow,
            }));
        }
        // Rows or a `journal_seq` and no journal: the store is ahead of an empty journal (D-66).
        let mut view = View::empty();
        view.damaged = Some(DamagedInfo {
            kind: DamageKind::StoreAhead,
        });
        let Some(mut done) =
            rebase(store, game, &row, &view, &[], None, Holder::None, deadline).await?
        else {
            return Ok(None);
        };
        if incomplete_aside {
            done.fate = Fate::IncompleteRebaseMovedAside;
        }
        return Ok(Some(done));
    };

    // ---- a journal file, lock held ----
    let bytes = bytes.unwrap_or_default();
    let path = journal_path(&jdir, id);
    let short = bytes.len() < HEADER_LEN;
    let mut view = if short {
        View {
            damaged: Some(DamagedInfo {
                kind: DamageKind::Middle {
                    at: 0,
                    resumes_at: 0,
                },
            }),
            ..View::empty()
        }
    } else {
        view_of(&bytes, id, store_uuid)?
    };

    // A file cut before its header was complete, for a row that holds nothing: a create that
    // was killed. The header is written again, in place, under the lock we hold.
    if short && !final_row && nothing_stored {
        let writer_path = path.clone();
        let file = join(
            tokio::task::spawn_blocking(move || -> Result<File, JournalError> {
                let mut file = file;
                file.set_len(0).map_err(|e| io_err(&e))?;
                file.seek(SeekFrom::Start(0)).map_err(|e| io_err(&e))?;
                file.write_all(&encode_header(id, store_uuid))
                    .and_then(|()| file.sync_all())
                    .map_err(|e| io_err(&e))?;
                writer_path
                    .parent()
                    .map_or(Ok(()), sync_dir)
                    .map_err(|e| io_err(&e))?;
                Ok(file)
            })
            .await,
        )??;
        let view = View::empty();
        let mut recovered = recovered_from_row(&row, &view, RunSource::Both);
        recovered.report.created_empty = true;
        return Ok(Some(Done {
            holder: Holder::File {
                file,
                path,
                acked_len: HEADER_LEN as u64,
                next_seq: 1,
            },
            recovered,
            fate: Fate::EmptyRow,
        }));
    }

    // D-66: a store that holds a record the journal does not prove is damage, with or without a
    // torn tail. The scan's own damage takes precedence (3.11.2 dated note).
    if view.damaged.is_none() && db_seq > view.last_seq() {
        view.damaged = Some(DamagedInfo {
            kind: DamageKind::StoreAhead,
        });
    }
    let damaged = view.damaged.is_some();

    if final_row {
        if damaged {
            // RR-A-1 (b): no rebase; the file goes to `damaged/`, rows unchanged.
            let jdir = jdir.clone();
            let moved = join(
                tokio::task::spawn_blocking(move || {
                    let moved = move_aside_damaged(&jdir, id);
                    drop(file);
                    moved.map_err(|e| io_err(&e))
                })
                .await,
            )??;
            let mut recovered = recovered_from_row(&row, &view, RunSource::Store);
            recovered.report.moved_aside = Some(moved.damaged);
            return Ok(Some(Done {
                holder: Holder::None,
                recovered,
                fate: Fate::MovedAside,
            }));
        }
        let last_terminal = view.records.last().is_some_and(is_terminal);
        let mut recovered = recovered_from_row(&row, &view, RunSource::Journal);
        recovered.records.clone_from(&view.records);
        let acked = u64::try_from(view.intact_end).unwrap_or(0);
        return Ok(Some(Done {
            holder: Holder::File {
                file,
                path,
                acked_len: acked,
                next_seq: view.last_seq() + 1,
            },
            recovered,
            fate: if last_terminal {
                Fate::FinalClosable
            } else {
                Fate::FinalNeedsRecord
            },
        }));
    }

    let folded = fold(&view.records);
    let plan = reconcile(&ReconcileInput {
        journal: &folded,
        stored: &row.moves,
        scan_damaged: view
            .damaged
            .is_some_and(|d| d.kind != DamageKind::StoreAhead),
        db_seq,
        last_intact_seq: view.last_seq(),
        stored_clocks,
    });

    // Neither the journal nor the store holds a readable game: no intact run, so not even a
    // `Start`, and no `MOVE` row and no `journal_seq` to build one from. The caller offers
    // `VoidUnreadableGame`; the file and the rows are untouched.
    if plan.damaged
        && view.records.is_empty()
        && row.moves.is_empty()
        && db_seq == 0
        && view
            .damaged
            .is_some_and(|d| d.kind != DamageKind::StoreAhead)
    {
        return Err(JournalError::Unreadable.into());
    }

    if plan.damaged {
        let intact = bytes
            .get(HEADER_LEN..view.intact_end.max(HEADER_LEN))
            .unwrap_or_default()
            .to_vec();
        let Some(mut done) = rebase(
            store,
            game,
            &row,
            &view,
            &intact,
            Some((&folded, plan)),
            Holder::File {
                file,
                path,
                acked_len: 0,
                next_seq: 0,
            },
            deadline,
        )
        .await?
        else {
            return Ok(None);
        };
        if incomplete_aside {
            done.fate = Fate::IncompleteRebaseMovedAside;
        }
        return Ok(Some(done));
    }

    // ---- intact journal, in-progress row ----
    // A torn tail is cut before anything is appended after it (3.11.2).
    let acked = u64::try_from(view.intact_end).unwrap_or(0);
    let file = if view.torn {
        join(
            tokio::task::spawn_blocking(move || -> Result<File, JournalError> {
                file.set_len(acked)
                    .and_then(|()| file.sync_all())
                    .map_err(|e| io_err(&e))?;
                Ok(file)
            })
            .await,
        )??
    } else {
        file
    };
    let (rows, clocks) = apply_plan(&row.moves, stored_clocks, &plan);
    // The whole-journal call below upserts every journal move (journal wins) and sets the clocks,
    // so the plan is only needed for what that call cannot do: delete stored rows beyond the
    // journal's end. In the common case the resume is one transaction.
    if plan.rows_to_delete_from.is_some() {
        store
            .apply_reconcile(game, plan.clone(), None, wait_left(deadline)?)
            .await?;
    }
    // D3-10, D-87: the whole journal in ONE call. A batch that is not applied yet waits in the
    // backlog (`Queued`); a batch that is refused parks the game and comes back as the error.
    let applied = if view.records.is_empty() {
        ApplyOutcome::Applied
    } else {
        store.apply_records(game, view.records.clone()).await?
    };
    let terminal = view.records.last().is_some_and(is_terminal);
    let mut report = report_of(&view, &rows, plan.source);
    report.damaged = None;
    Ok(Some(Done {
        holder: Holder::File {
            file,
            path,
            acked_len: acked,
            next_seq: view.last_seq() + 1,
        },
        recovered: Recovered {
            records: view.records,
            moves: rows,
            clocks,
            report,
        },
        fate: if terminal {
            Fate::Terminal(applied)
        } else {
            Fate::Interrupted
        },
    }))
}

fn empty_recovered() -> Recovered {
    Recovered {
        records: Vec::new(),
        moves: Vec::new(),
        clocks: None,
        report: RecoveryReport {
            torn_tail_dropped: false,
            damaged: None,
            recovered_moves: 0,
            source: RunSource::Both,
            rebased: false,
            moved_aside: None,
            created_empty: false,
        },
    }
}

fn recovered_from_row(row: &ResumeRow, view: &View, source: RunSource) -> Recovered {
    let mut recovered = Recovered {
        records: Vec::new(),
        moves: row.moves.clone(),
        clocks: clocks_of(row),
        report: report_of(view, &row.moves, source),
    };
    recovered.report.recovered_moves = u32::try_from(row.moves.len()).unwrap_or(u32::MAX);
    recovered
}

/// The rebase (SDD 3.11.3, D-83, D-110): step 1 build and write the `.rebase`, step 2 one store
/// transaction (plan and `journal_seq`), the failpoint `journal.rebase.db_committed`, step 3 the
/// two moves. `old` is the damaged file's handle (its lock is dropped after the swap).
#[allow(
    clippy::too_many_arguments,
    reason = "the inputs of one rebase; a struct would only rename them"
)]
async fn rebase(
    store: &Store,
    game: GameId,
    row: &ResumeRow,
    view: &View,
    intact_bytes: &[u8],
    planned: Option<(&Folded, Plan)>,
    old: Holder,
    deadline: Option<Instant>,
) -> Result<Option<Done>, ResumeError> {
    let id = game.as_uuid();
    let store_uuid = store.store_uuid()?;
    let jdir = store.data_dir().journal_dir();
    let stored_clocks = clocks_of(row);
    let empty = Folded::default();
    let (folded, plan) = match planned {
        Some((folded, plan)) => (folded, plan),
        None => {
            // No file: an empty intact run against the store (D-66).
            let plan = reconcile(&ReconcileInput {
                journal: &empty,
                stored: &row.moves,
                scan_damaged: false,
                db_seq: row.record.journal_seq,
                last_intact_seq: 0,
                stored_clocks,
            });
            (&empty, plan)
        }
    };
    let start = if view.records.is_empty() {
        Some(start_from_row(row, id)?)
    } else {
        None
    };
    let journal_last_ply = folded.moves.keys().next_back().copied().unwrap_or(0);
    let rebased = build_rebased(&RebaseInput {
        game: id,
        store: store_uuid,
        intact_bytes,
        intact: &view.records,
        source: plan.source,
        journal_last_ply,
        stored: &row.moves,
        clocks: reconciled_clocks(&plan, stored_clocks),
        start,
    })
    .map_err(|_| JournalError::Unreadable)?;

    // Step 1: `<uuid>.jnl.rebase`, locked, synced.
    let rebase_file = {
        let (jdir, bytes) = (jdir.clone(), rebased.clone());
        join(tokio::task::spawn_blocking(move || write_rebase_file(&jdir, id, &bytes)).await)??
    };
    // FIND-D4R-01: with no journal file in our hands, the `.rebase` we just made (it is ours
    // alone, `create_new`) is the only claim on this game. If a `<uuid>.jnl` exists now, another
    // process built one between our decision and this point and may hold it: nothing of ours
    // may move it. The attempt is undone and starts again, which finds the file and its lock.
    if matches!(old, Holder::None) && exists(&journal_path(&jdir, id)) {
        drop(rebase_file);
        let _ = fs::remove_file(rebase_path(&jdir, id));
        return Ok(None);
    }
    // Step 2: one store transaction; `journal_seq` becomes the new file's last `seq`.
    let step_two = match wait_left(deadline) {
        Ok(wait) => {
            store
                .apply_reconcile(game, plan.clone(), Some(rebased.last_seq), wait)
                .await
        }
        Err(error) => Err(error),
    };
    if let Err(error) = step_two {
        // The `.rebase` is derived data; the next recovery would remove it as stale anyway.
        let _ = fs::remove_file(rebase_path(&jdir, id));
        return Err(error.into());
    }
    let ctx = journal_path(&jdir, id);
    fault::check("journal.rebase.db_committed", &ctx).map_err(|e| io_err(&e))?;
    // Step 3: the old file aside, the new one into place.
    let may_move_aside = !matches!(old, Holder::None);
    let (writer, moved) = {
        let jdir = jdir.clone();
        join(
            tokio::task::spawn_blocking(move || -> Result<_, JournalError> {
                let swapped = swap(&jdir, id, rebase_file, may_move_aside)?;
                drop(old);
                Ok(swapped)
            })
            .await,
        )??
    };
    let scanned =
        scan_bytes(&rebased.bytes, id, store_uuid).map_err(|_| JournalError::Unreadable)?;
    let (rows, clocks) = apply_plan(&row.moves, stored_clocks, &plan);
    let mut report = report_of(view, &rows, plan.source);
    report.rebased = true;
    // The file the swap just made (not the highest `k` of the game, FIND-D4R-06).
    report.moved_aside = moved.map(|aside| aside.damaged);
    // A rebased file that ends in a terminal record is a finished game whose end the store may
    // not hold yet: the whole file goes to the applier in one call.
    let terminal = scanned.records.last().is_some_and(is_terminal);
    let fate = if terminal {
        Fate::Terminal(store.apply_records(game, scanned.records.clone()).await?)
    } else {
        Fate::Rebased
    };
    if report.damaged.is_none() {
        report.damaged = Some(DamagedInfo {
            kind: DamageKind::StoreAhead,
        });
    }
    Ok(Some(Done {
        holder: Holder::Writer(writer),
        recovered: Recovered {
            records: scanned.records,
            moves: rows,
            clocks,
            report,
        },
        fate,
    }))
}

// ------------------------------------------------------------------------ open_for_resume

impl MoveJournal {
    /// SDD 3.7.3: takes the game's lock, repairs the file windows (`.rebase`, `done/`), scans,
    /// reconciles with the store, and returns the writer and what was recovered.
    ///
    /// - A **row with no journal** yields a header-only journal and an empty `Recovered`
    ///   (`report.created_empty`): the app appends `Start` as record 1 with
    ///   `wall_ms = GAME.started_at` (3.9.2). A complete `<uuid>.jnl.rebase` is renamed into
    ///   place first (CT-ST-089); an incomplete one is moved aside (B-R05).
    /// - A **damaged** journal (a middle, an undecodable tail, an unreadable header, or the
    ///   store ahead of it, D-66) is rebased: the writer continues on the new file and
    ///   `report.damaged` is `Some` (the app reports `JournalUnreadable`).
    /// - A **final or voided** game is never rebased (RR-A-1 (b)): the writer is `None` when the
    ///   damaged file was moved aside or the game has no journal; for an intact journal of a
    ///   final game the writer is returned so that the caller can `close_final`.
    /// - The whole journal goes to the applier in one call (D3-10).
    ///
    /// `Locked` when another descriptor holds the lock; `NotFound` for a game with no row;
    /// `Unreadable` when neither the journal nor the store holds a readable game; and a
    /// `ResumeError::Store` for a database failure (`GameParked`, `Busy`, ...). The SDD's
    /// `Result<_, JournalError>` and its `(JournalWriter, Recovered)` are widened for these two
    /// cases (reported).
    pub async fn open_for_resume(
        &self,
        store: &Store,
        game: &GameId,
    ) -> Result<(Option<JournalWriter>, Recovered), ResumeError> {
        let done = recover_game(store, *game, Intent::Resume, None).await?;
        Ok((done.holder.into_writer(), done.recovered))
    }
}

// ------------------------------------------------------------------------ startup recovery

/// The games that have a journal file or a `.rebase` in `journal/`, in name order.
fn list_games(jdir: &Path) -> io::Result<Vec<Uuid>> {
    let mut games = Vec::new();
    for entry in fs::read_dir(jdir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let stem = name
            .strip_suffix(".jnl.rebase")
            .or_else(|| name.strip_suffix(".jnl"));
        let Some(stem) = stem else { continue };
        let Ok(id) = Uuid::try_parse(stem) else {
            continue;
        };
        if id.hyphenated().to_string() == stem && entry.file_type()?.is_file() {
            games.push(id);
        }
    }
    games.sort();
    games.dedup();
    Ok(games)
}

/// D-72: the order key of a pending `GameEnd`: `(ended_at, game_uuid)`. Games that finalize
/// nothing (no `GameEnd` last, or a `GameEnd` followed by a `Voided`, D-87) come first, by uuid.
fn order_key(jdir: &Path, id: Uuid, store: Uuid) -> (u8, i64, Uuid) {
    let bytes = fs::read(journal_path(jdir, id)).ok();
    let last = bytes
        .and_then(|b| scan_bytes(&b, id, store).ok())
        .and_then(|scan| scan.records.last().cloned());
    match last.map(|r| r.payload) {
        Some(Payload::GameEnd(end)) => (1, end.ended_at, id),
        _ => (0, 0, id),
    }
}

fn failed(error: &ResumeError) -> RecoveryOutcome {
    RecoveryOutcome::Failed {
        error: error.to_string(),
    }
}

/// The most time the whole startup scan spends waiting for the database when another process
/// holds its write lock (FIND-D4R-07, NFR-1). `ESTIMATE`: twice the connection's 5 s wait, so
/// that one busy game does not use it up and many busy games do not add up to minutes. A game
/// that needs the database after the budget is spent is reported `Failed` (busy) at once; its
/// journal stays, and the next start or its resume retries.
const RECOVERY_BUDGET: Duration = Duration::from_secs(10);

/// The startup scan (DD-ST-019, SEQ-ST-01): every journal in `journal/` whose lock is free is
/// brought to the store's state, in the order of D-72. One bad journal never stops the others.
pub(crate) async fn startup(store: &Store) -> StartupReport {
    let Ok(store_uuid) = store.store_uuid() else {
        return StartupReport {
            games: Vec::new(),
            scan_failed: true,
        };
    };
    let jdir = store.data_dir().journal_dir();
    let listed = {
        let (jdir, _) = (jdir.clone(), ());
        tokio::task::spawn_blocking(move || {
            let games = list_games(&jdir)?;
            let mut keyed: Vec<(u8, i64, Uuid)> = games
                .into_iter()
                .map(|id| order_key(&jdir, id, store_uuid))
                .collect();
            keyed.sort_unstable();
            Ok::<_, io::Error>(keyed)
        })
        .await
    };
    let Ok(Ok(order)) = listed else {
        return StartupReport {
            games: Vec::new(),
            scan_failed: true,
        };
    };
    let mut report = StartupReport::default();
    let deadline = Instant::now() + RECOVERY_BUDGET;
    for (_, _, id) in order {
        let game = GameId::from_uuid(id);
        let outcome = recover_one(store, game, deadline).await;
        report.games.push(RecoveredGame { game, outcome });
    }
    report
}

async fn recover_one(store: &Store, game: GameId, deadline: Instant) -> RecoveryOutcome {
    let done = match recover_game(store, game, Intent::Startup, Some(deadline)).await {
        Ok(done) => done,
        Err(ResumeError::Journal(JournalError::Locked { .. })) => return RecoveryOutcome::Held,
        Err(error) => return classify_error(store, game, &error).await,
    };
    let Done {
        holder,
        recovered,
        fate,
    } = done;
    let close = |holder: Holder| async move {
        let Some(path) = holder.path() else {
            return Ok(());
        };
        tokio::task::spawn_blocking(move || {
            // The writer's descriptor (and its lock) is released when `holder` drops, after
            // the move, as in `close_final`.
            let result = finish_close_final(&path);
            drop(holder);
            result.map(drop)
        })
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::Other))?
    };
    match fate {
        Fate::NoRow => RecoveryOutcome::NoGameRow,
        Fate::RemovedEmpty => RecoveryOutcome::RemovedEmpty,
        Fate::NoJournalFinal | Fate::EmptyRow => RecoveryOutcome::Reconciled {
            recovered_moves: recovered.report.recovered_moves,
        },
        Fate::FinishedClose => RecoveryOutcome::FinishedClose,
        Fate::Interrupted => RecoveryOutcome::Reconciled {
            recovered_moves: recovered.report.recovered_moves,
        },
        Fate::Rebased => RecoveryOutcome::Rebased {
            recovered_moves: recovered.report.recovered_moves,
            damaged: recovered.report.damaged.unwrap_or(DamagedInfo {
                kind: DamageKind::StoreAhead,
            }),
        },
        Fate::IncompleteRebaseMovedAside => RecoveryOutcome::IncompleteRebaseMovedAside,
        Fate::DoneCollision => RecoveryOutcome::DoneNameCollision,
        Fate::MovedAside => RecoveryOutcome::MovedAside {
            damaged: recovered.report.damaged,
        },
        Fate::FinalNeedsRecord => RecoveryOutcome::NeedsRecord,
        Fate::FinalClosable => match close(holder).await {
            Ok(()) => RecoveryOutcome::ClosedLeftover,
            Err(e) => RecoveryOutcome::Failed {
                error: JournalError::write_failed(&e).to_string(),
            },
        },
        Fate::Terminal(ApplyOutcome::Applied) => {
            let damaged = recovered.report.damaged;
            match close(holder).await {
                Ok(()) => {
                    let _ = damaged;
                    RecoveryOutcome::Finalized
                }
                Err(e) => RecoveryOutcome::Failed {
                    error: JournalError::write_failed(&e).to_string(),
                },
            }
        }
        // The database is busy: the batch waits in the backlog and the journal stays where it
        // is, so the next start (or the retry) finishes the job.
        Fate::Terminal(_) => RecoveryOutcome::Queued,
    }
}

/// A refused batch parks the game: say so, instead of a bare error.
async fn classify_error(store: &Store, game: GameId, error: &ResumeError) -> RecoveryOutcome {
    if matches!(error, ResumeError::Store(StoreError::GameParked { .. })) {
        return RecoveryOutcome::Parked { cause: None };
    }
    if let Ok(parked) = store.parked_games().await
        && let Some(entry) = parked.iter().find(|p| p.game == game)
    {
        return RecoveryOutcome::Parked {
            cause: Some(entry.cause),
        };
    }
    failed(error)
}

/// Used by tests of the table above.
#[cfg(test)]
mod tests {
    use super::*;

    /// `lock_open` waits (the grace of B-R04) for a holder that finishes by moving the file
    /// away; when it gets the lock the name belongs to another file, and it must say `Locked`
    /// rather than hand back a descriptor of the file that left (same device, other inode).
    #[test]
    fn lock_open_refuses_a_file_that_was_replaced_while_it_waited() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("g.jnl");
        fs::write(&path, b"old").unwrap();
        let holder = File::open(&path).unwrap();
        holder.try_lock().unwrap();
        let waiting = {
            let path = path.clone();
            std::thread::spawn(move || lock_open(&path))
        };
        std::thread::sleep(std::time::Duration::from_millis(15));
        fs::rename(&path, dir.path().join("g.moved")).unwrap();
        fs::write(&path, b"new").unwrap();
        drop(holder);
        let got = waiting.join().unwrap();
        assert!(
            matches!(got, Err(JournalError::Locked { .. })),
            "{:?}",
            got.map(|f| f.is_some())
        );
    }

    #[test]
    fn lock_open_hands_back_the_file_it_locked_and_none_for_no_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("g.jnl");
        assert!(lock_open(&path).unwrap().is_none());
        fs::write(&path, b"x").unwrap();
        assert!(lock_open(&path).unwrap().is_some());
    }

    fn store_in(dir: &Path) -> Store {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime
            .block_on(Store::open(crate::store::StoreOptions::new(
                crate::datadir::DataDir::new(dir),
            )))
            .unwrap()
            .store
    }

    /// The classification of a failed scan step (mutant D45): `GameParked` is `Parked` with no
    /// cause; any other error of an unparked game is `Failed` by category.
    #[test]
    fn a_failed_step_is_classified_by_its_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = store_in(dir.path());
        let game = GameId::from_uuid(Uuid::from_u128(5));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let parked = ResumeError::Store(StoreError::GameParked { game });
        assert_eq!(
            runtime.block_on(classify_error(&store, game, &parked)),
            RecoveryOutcome::Parked { cause: None }
        );
        let busy = ResumeError::Store(StoreError::Busy);
        assert_eq!(
            runtime.block_on(classify_error(&store, game, &busy)),
            RecoveryOutcome::Failed {
                error: "the store is busy".into()
            }
        );
    }

    /// The startup budget (FIND-D4R-07): no deadline is the normal wait, time left is passed on,
    /// a spent budget is `Busy` at once.
    #[test]
    fn the_budget_is_passed_on_and_a_spent_one_is_busy() {
        assert_eq!(wait_left(None).unwrap(), None);
        let long = wait_left(Some(Instant::now() + Duration::from_secs(60)))
            .unwrap()
            .unwrap();
        assert_eq!(long, NORMAL_WAIT, "never more than the normal wait");
        let left = wait_left(Some(Instant::now() + Duration::from_secs(3)))
            .unwrap()
            .unwrap();
        assert!(left > Duration::from_secs(2) && left <= Duration::from_secs(3));
        let spent = Instant::now();
        assert!(matches!(wait_left(Some(spent)), Err(StoreError::Busy)));
    }

    fn journal_dir_with_store() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let jdir = dir.path().join("journal");
        fs::create_dir_all(&jdir).unwrap();
        (dir, jdir)
    }

    /// With no lock in our hands there is nothing to repair: a `.rebase` that shows up belongs
    /// to the process that is building it and is left exactly as it is (FIND-D4R-01; mutant M03).
    #[test]
    fn settle_and_read_touches_nothing_without_a_lock() {
        let (_dir, jdir) = journal_dir_with_store();
        let (id, store) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let reb = rebase_path(&jdir, id);
        fs::write(&reb, b"in flight").unwrap();
        let settle = settle_and_read(&jdir, id, store, None, None).unwrap();
        assert!(settle.file.is_none() && settle.bytes.is_none());
        assert_eq!(settle.settled, Settled::Nothing);
        assert_eq!(fs::read(&reb).unwrap(), b"in flight");
        assert!(!jdir.join("damaged").exists());
    }

    /// A `.rebase` that we locked and that its creator then removed is no file at all, not a
    /// failed read (mutant M06).
    #[test]
    fn settle_and_read_treats_a_locked_file_that_vanished_as_no_file() {
        let (_dir, jdir) = journal_dir_with_store();
        let (id, store) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let reb = rebase_path(&jdir, id);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&reb)
            .unwrap();
        file.try_lock().unwrap();
        fs::remove_file(&reb).unwrap();
        let settle = settle_and_read(&jdir, id, store, None, Some(file)).unwrap();
        assert!(settle.file.is_none() && settle.bytes.is_none());
    }

    #[test]
    fn a_report_of_an_empty_view_has_no_damage() {
        let view = View::empty();
        let report = report_of(&view, &[], RunSource::Both);
        assert!(report.damaged.is_none());
        assert_eq!(report.recovered_moves, 0);
    }
}
