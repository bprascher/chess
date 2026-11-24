//! D-83 / D-110: the rebase **builder** and the file steps around it (SDD 3.11.3). Part B
//! builds the new file and moves files; the database step (3.11.3 step 2, one store
//! transaction that applies the plan and sets `GAME.journal_seq`) is part D3/D4, and the
//! failpoint `journal.rebase.db_committed` between step 2 and step 3 is wired there.
//!
//! `build_rebased` is a pure function of the intact run `R` and the stored rows: it copies the
//! intact records **byte for byte** (same `seq`, `wall_ms`, payload; nothing renumbered or
//! re-timed), then synthesizes one `Move` per stored ply beyond the journal's end when the
//! store is longer and agrees (`source = Store`), then one `Clock` with the reconciled clocks
//! (`running_side` null, `wall_ms` of the previous record), except when the last record is
//! terminal (`GameEnd`, `Abort`, `Voided`) or is already the closing `Clock`: then no `Clock` follows
//! it (CH-11 B, spec 3.3 (e)). If `R` is empty it starts with a
//! `Start` synthesized by the caller from the `GAME` row (`wall_ms = GAME.started_at`).
//! Two runs give identical bytes.
//!
//! File steps: `write_rebase_file` (step 1: create `<uuid>.jnl.rebase`, lock, write, sync file
//! and directory), `swap_in_rebased` (step 3: 3a link the old file aside, 3b remove, 3c sync
//! both directories, 3d link the new file into place, 3e remove `.rebase`, 3f sync `journal/`).
//! The crash windows are in `files.rs`.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use uuid::Uuid;

use crate::datadir::FILE_MODE;
use crate::error::JournalError;
use crate::fault;
use crate::journal::files::{journal_path, move_aside_damaged, rebase_path, sync_dir};
use crate::journal::format::{
    ClockPayload, FormatError, HEADER_LEN, JournalRecord, Kind, MovePayload, Payload, StartPayload,
    encode_header, encode_record,
};
use crate::journal::reader::scan_bytes;
use crate::journal::reconcile::{ClockPair, RunSource, StoredMove};
use crate::journal::writer::JournalWriter;

/// Everything the builder needs.
pub struct RebaseInput<'a> {
    pub game: Uuid,
    pub store: Uuid,
    /// The intact records region of the old file: `bytes[HEADER_LEN..scan.intact_end]`.
    pub intact_bytes: &'a [u8],
    /// The same records, decoded (`scan.records`).
    pub intact: &'a [JournalRecord],
    /// `RunSource` of the reconcile plan: stored rows are synthesized only for `Store`.
    pub source: RunSource,
    /// The folded journal's last ply in force, as `MOVE.ply` (0 when none).
    pub journal_last_ply: u32,
    /// The stored `MOVE` rows, ascending.
    pub stored: &'a [StoredMove],
    /// The reconciled `GAME` clocks (`reconciled_clocks`).
    pub clocks: ClockPair,
    /// The `Start` to write as record 1 and its `wall_ms` (`GAME.started_at`). Required when
    /// `intact` is empty; ignored otherwise.
    pub start: Option<(StartPayload, i64)>,
}

/// The new file's bytes and its last `seq`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rebased {
    pub bytes: Vec<u8>,
    pub last_seq: u64,
}

/// Why the builder refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseError {
    /// The intact run is empty and no `Start` was given.
    MissingStart,
    /// A record could not be encoded.
    Format(FormatError),
    /// `intact_bytes` and `intact` do not describe the same intact run (FIND-RW1-10): the bytes
    /// do not scan clean to their own end, or they decode to other records than `intact`.
    InconsistentIntact,
}

fn is_terminal(kind: Kind) -> bool {
    matches!(kind, Kind::GameEnd | Kind::Abort | Kind::Voided)
}

/// FIND-RW1-10: the two views of the intact run must agree, or the new file would carry bytes
/// that scan differently from the `seq` the builder reports.
fn check_intact(input: &RebaseInput<'_>) -> Result<(), RebaseError> {
    let mut file = encode_header(input.game, input.store).to_vec();
    file.extend_from_slice(input.intact_bytes);
    let scan =
        scan_bytes(&file, input.game, input.store).map_err(|_| RebaseError::InconsistentIntact)?;
    // Damage and a torn tail both leave bytes after `intact_end`, so this one test covers them.
    let clean = scan.intact_end == file.len() && scan.records == input.intact;
    if clean {
        Ok(())
    } else {
        Err(RebaseError::InconsistentIntact)
    }
}

pub fn build_rebased(input: &RebaseInput<'_>) -> Result<Rebased, RebaseError> {
    check_intact(input)?;
    let mut bytes = encode_header(input.game, input.store).to_vec();
    let mut last_seq;
    let mut last_wall;
    let closing = ClockPayload {
        white_ms: input.clocks.map(|c| c.0),
        black_ms: input.clocks.map(|c| c.1),
        running_side: None,
    };
    // CH-11 B: the file ends with a terminal record (`GameEnd`, `Abort`, `Voided`) -> no closing
    // `Clock` after it. Idempotence (spec 3.3 (e), FIND-RW1-03): nor after a `Clock` that is
    // already the closing one (same clocks, no running side), so a second rebase of a rebased
    // file is byte-equal to the first.
    let mut ends_terminal = false;
    if let Some(last) = input.intact.last() {
        bytes.extend_from_slice(input.intact_bytes);
        last_seq = last.seq;
        last_wall = last.wall_ms;
        ends_terminal = is_terminal(last.kind())
            || matches!(&last.payload, Payload::Clock(clock) if *clock == closing);
    } else {
        let (start, wall_ms) = input.start.clone().ok_or(RebaseError::MissingStart)?;
        let record = JournalRecord {
            seq: 1,
            wall_ms,
            payload: Payload::Start(start),
        };
        bytes.extend(encode_record(&record).map_err(RebaseError::Format)?);
        last_seq = 1;
        last_wall = wall_ms;
    }
    if input.source == RunSource::Store {
        for row in input
            .stored
            .iter()
            .filter(|row| row.ply > input.journal_last_ply)
        {
            last_seq += 1;
            ends_terminal = false;
            last_wall = row.played_at.unwrap_or(last_wall);
            let record = JournalRecord {
                seq: last_seq,
                wall_ms: last_wall,
                payload: Payload::Move(MovePayload {
                    ply: row.ply.saturating_sub(1),
                    uci: row.uci.clone(),
                    think_ms: row.think_ms.unwrap_or(0),
                    white_ms: row.clocks.map(|c| c.0),
                    black_ms: row.clocks.map(|c| c.1),
                    san: row.san.clone(),
                    fen_after: row.fen_after.clone(),
                }),
            };
            bytes.extend(encode_record(&record).map_err(RebaseError::Format)?);
        }
    }
    if !ends_terminal {
        last_seq += 1;
        let clock = JournalRecord {
            seq: last_seq,
            wall_ms: last_wall,
            payload: Payload::Clock(closing),
        };
        bytes.extend(encode_record(&clock).map_err(RebaseError::Format)?);
    }
    debug_assert!(bytes.len() > HEADER_LEN);
    Ok(Rebased { bytes, last_seq })
}

/// A complete `<uuid>.jnl.rebase`, locked and synced, waiting for the swap.
pub struct RebaseFile {
    file: fs::File,
    len: u64,
    last_seq: u64,
}

/// Step 1: creates `journal/<uuid>.jnl.rebase` (mode `0600`, never replaces a file), locks it,
/// writes the bytes and syncs the file and `journal/`. A stale `.rebase` must have been removed
/// first (`settle_rebase_files`). Failpoint `journal.rebase.built` follows the syncs.
pub fn write_rebase_file(
    journal_dir: &Path,
    game: Uuid,
    rebased: &Rebased,
) -> Result<RebaseFile, JournalError> {
    let path = rebase_path(journal_dir, game);
    let io_err = |e: &io::Error| JournalError::write_failed(e);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(&path)
        .map_err(|e| io_err(&e))?;
    let written = (|| -> Result<(), JournalError> {
        file.try_lock().map_err(|e| match e {
            fs::TryLockError::WouldBlock => JournalError::Locked { owner_kind: None },
            fs::TryLockError::Error(e) => io_err(&e),
        })?;
        file.write_all(&rebased.bytes)
            .and_then(|()| fault::sync_file("sync.journal_rebase_file", &path, &file))
            .and_then(|()| sync_dir(journal_dir))
            .and_then(|()| fault::check("journal.rebase.built", &path))
            .map_err(|e| io_err(&e))
    })();
    if let Err(error) = written {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(RebaseFile {
        file,
        len: u64::try_from(rebased.bytes.len()).unwrap_or(0),
        last_seq: rebased.last_seq,
    })
}

/// Step 3 (D-110): moves the damaged `<uuid>.jnl` aside (if it exists), links the rebased file
/// into place without replacing anything, removes the `.rebase` name, syncs `journal/`. The lock
/// taken on the `.rebase` file belongs to the inode and carries over to the new name; the
/// returned writer continues with `seq = last_seq + 1`. Failpoints: `journal.rebase.old_linked`
/// (after 3a), `journal.rebase.old_aside` (after 3c), `journal.rebase.new_linked` (after 3d),
/// `journal.rebase.new_in_place` (after 3f).
pub fn swap_in_rebased(
    journal_dir: &Path,
    game: Uuid,
    rebase: RebaseFile,
) -> Result<JournalWriter, JournalError> {
    let io_err = |e: &io::Error| JournalError::write_failed(e);
    let jnl = journal_path(journal_dir, game);
    let new = rebase_path(journal_dir, game);
    if fs::symlink_metadata(&jnl).is_ok() {
        move_aside_damaged(journal_dir, game).map_err(|e| io_err(&e))?;
    }
    fault::check("journal.rebase.old_aside", &jnl).map_err(|e| io_err(&e))?;
    fs::hard_link(&new, &jnl).map_err(|e| io_err(&e))?;
    fault::check("journal.rebase.new_linked", &jnl).map_err(|e| io_err(&e))?;
    fs::remove_file(&new).map_err(|e| io_err(&e))?;
    sync_dir(journal_dir).map_err(|e| io_err(&e))?;
    fault::check("journal.rebase.new_in_place", &jnl).map_err(|e| io_err(&e))?;
    Ok(JournalWriter::from_locked(
        rebase.file,
        jnl,
        rebase.len,
        rebase.last_seq + 1,
    ))
}
