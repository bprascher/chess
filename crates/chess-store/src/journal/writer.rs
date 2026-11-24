//! DD-ST-016 `JournalWriter` and `MoveJournal` (SDD 3.7.3, 3.11.6; D-82).
//!
//! **Flush before ack.** `append` returns only after `File::sync_all` (`fcntl(F_FULLFSYNC)` on
//! Apple targets, spike OQ-14) returned for the record. The work is one blocking task that runs
//! to completion even if the future is dropped.
//!
//! **Writer rule after a failed or ambiguous append (D-82).** The writer keeps `acked_len` (the
//! file length after the last record whose sync returned) and the next `seq`. A record is
//! written at `acked_len` after an explicit seek (not at the descriptor's position, which a
//! `set_len` does not move), synced, and only then are `acked_len` and `seq` advanced. A write
//! or sync error is followed by `set_len(acked_len)` and a sync in the same task: if that
//! succeeds the file is as before, the `seq` is not consumed and the writer stays usable
//! (`WriteFailed{cause}`); if it fails, or the caller dropped the future before the outcome was
//! known, the writer is **poisoned** and every later append returns
//! `WriteFailed{cause: Poisoned}` without touching the file.
//!
//! The lock is `File::try_lock` (`flock(LOCK_EX | LOCK_NB)`). Every descriptor comes from
//! `OpenOptions`/`File::open`, which set close-on-exec (checked by CT-ST-006 with a child).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use chess_domain::GameId;
use uuid::Uuid;

use crate::datadir::{DataDir, FILE_MODE};
use crate::error::{JournalError, WriteCause, write_cause};
use crate::fault;
use crate::journal::files::{journal_path, move_no_replace, sync_dir};
use crate::journal::format::{FormatError, JournalRecord, Payload, encode_header, encode_record};

/// `probe`: whether a game's journal lock is held right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    Free,
    Held,
}

/// Creates journals and probes their locks (IFP-ST-02). Cheap to clone.
#[derive(Debug, Clone)]
pub struct MoveJournal {
    data_dir: DataDir,
    store: Uuid,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl MoveJournal {
    /// `store` is the `store_uuid` of `store.id`, written into every header.
    pub fn new(data_dir: DataDir, store: Uuid) -> MoveJournal {
        MoveJournal { data_dir, store }
    }

    pub fn store_id(&self) -> Uuid {
        self.store
    }

    pub fn path(&self, game: &GameId) -> PathBuf {
        journal_path(&self.data_dir.journal_dir(), game.as_uuid())
    }

    /// Creates `journal/<uuid>.jnl` (mode `0600`, `create_new`: never replaces a file), takes
    /// the exclusive lock, writes the header and syncs the file and the directory. Another
    /// descriptor holding the lock is `Locked`. A header that could not be written removes the
    /// file again. Any other I/O error is `WriteFailed{cause}`.
    pub fn create(&self, game: &GameId) -> Result<JournalWriter, JournalError> {
        let path = self.path(game);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .open(&path)
            .map_err(|e| {
                // A file that exists and whose lock is held is a game open elsewhere.
                if e.kind() == io::ErrorKind::AlreadyExists && probe_path(&path) == LeaseState::Held
                {
                    JournalError::Locked { owner_kind: None }
                } else {
                    JournalError::write_failed(&e)
                }
            })?;
        let created = (|| -> Result<(), JournalError> {
            file.try_lock().map_err(|e| match e {
                fs::TryLockError::WouldBlock => JournalError::Locked { owner_kind: None },
                fs::TryLockError::Error(e) => JournalError::write_failed(&e),
            })?;
            let header = encode_header(game.as_uuid(), self.store);
            file.write_all(&header)
                .and_then(|()| fault::sync_file("sync.journal_create_file", &path, &file))
                .and_then(|()| {
                    path.parent().map_or(Ok(()), |dir| {
                        fault::sync_dir_named("sync.journal_create_dir", dir)
                    })
                })
                .map_err(|e| JournalError::write_failed(&e))
        })();
        if let Err(error) = created {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        let acked_len = u64::try_from(crate::journal::format::HEADER_LEN).unwrap_or(0);
        Ok(JournalWriter::from_locked(file, path, acked_len, 1))
    }

    /// Whether the lock is held, without keeping it (lock, then release). No file is `Free`.
    /// A file that cannot be opened or locked for another reason counts as `Held`: the lock is
    /// not proven free.
    pub fn probe(&self, game: &GameId) -> LeaseState {
        probe_path(&self.path(game))
    }
}

pub(crate) fn probe_path(path: &Path) -> LeaseState {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return LeaseState::Free,
        Err(_) => return LeaseState::Held,
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            LeaseState::Free
        }
        Err(_) => LeaseState::Held,
    }
}

struct WriteState {
    file: File,
    acked_len: u64,
    next_seq: u64,
}

struct Inner {
    path: PathBuf,
    poisoned: AtomicBool,
    state: Mutex<WriteState>,
}

/// The holder of a journal's lock. Appends are serialised by an internal mutex.
pub struct JournalWriter {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for JournalWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JournalWriter")
            .field("poisoned", &self.is_poisoned())
            .finish_non_exhaustive()
    }
}

/// Poisons the writer when the append future is dropped before it received its outcome.
struct PoisonOnDrop {
    inner: Arc<Inner>,
    armed: bool,
}

impl Drop for PoisonOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.inner.poisoned.store(true, Ordering::SeqCst);
        }
    }
}

impl JournalWriter {
    /// A writer over a file whose lock the caller already holds. `acked_len` is the length of
    /// the intact part (the writer writes there) and `next_seq` the `seq` of the next record.
    pub(crate) fn from_locked(
        file: File,
        path: PathBuf,
        acked_len: u64,
        next_seq: u64,
    ) -> JournalWriter {
        JournalWriter {
            inner: Arc::new(Inner {
                path,
                poisoned: AtomicBool::new(false),
                state: Mutex::new(WriteState {
                    file,
                    acked_len,
                    next_seq,
                }),
            }),
        }
    }

    /// True after a failed cleanup or a dropped append future (D-82).
    pub fn is_poisoned(&self) -> bool {
        self.inner.poisoned.load(Ordering::SeqCst)
    }

    /// The `seq` the next record will get.
    pub fn next_seq(&self) -> u64 {
        lock(&self.inner.state).next_seq
    }

    /// Length of the acknowledged part of the file.
    pub fn acked_len(&self) -> u64 {
        lock(&self.inner.state).acked_len
    }

    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// Appends one record and returns its `seq` only after the full sync returned. Runs in one
    /// blocking task that completes even if this future is dropped; dropping the future
    /// poisons the writer (the caller cannot know whether the record is durable).
    pub async fn append(&self, wall_ms: i64, payload: Payload) -> Result<u64, JournalError> {
        let inner = Arc::clone(&self.inner);
        let mut guard = PoisonOnDrop {
            inner: Arc::clone(&inner),
            armed: true,
        };
        let outcome =
            tokio::task::spawn_blocking(move || append_blocking(&inner, wall_ms, payload)).await;
        guard.armed = false;
        match outcome {
            Ok(result) => result,
            Err(_) => {
                self.inner.poisoned.store(true, Ordering::SeqCst);
                Err(JournalError::WriteFailed {
                    cause: WriteCause::Io(io::ErrorKind::Other),
                })
            }
        }
    }

    /// The same append without a runtime, for code that is already on a blocking thread.
    pub fn append_blocking(&self, wall_ms: i64, payload: Payload) -> Result<u64, JournalError> {
        append_blocking(&self.inner, wall_ms, payload)
    }

    /// Appends nothing further: moves the file to `journal/done/` (no replace), syncs both
    /// directories and releases the lock.
    pub fn close_final(self) -> Result<(), JournalError> {
        let path = self.inner.path.clone();
        let fail = |e: &io::Error| JournalError::write_failed(e);
        let dir = path.parent().ok_or(JournalError::Unreadable)?;
        let name = path.file_name().ok_or(JournalError::Unreadable)?;
        let done = dir.join("done");
        crate::datadir::create_dir_0700(&done).map_err(|e| fail(&e))?;
        move_no_replace(&path, &done.join(name)).map_err(|e| fail(&e))?;
        sync_dir(dir).map_err(|e| fail(&e))?;
        sync_dir(&done).map_err(|e| fail(&e))?;
        // The lock is released when the last handle on the file drops with `self`.
        Ok(())
    }
}

fn append_blocking(inner: &Inner, wall_ms: i64, payload: Payload) -> Result<u64, JournalError> {
    let mut state = lock(&inner.state);
    if inner.poisoned.load(Ordering::SeqCst) {
        return Err(JournalError::WriteFailed {
            cause: WriteCause::Poisoned,
        });
    }
    let seq = state.next_seq;
    let bytes = encode_record(&JournalRecord {
        seq,
        wall_ms,
        payload,
    })
    .map_err(|e| JournalError::WriteFailed {
        cause: match e {
            FormatError::TooLarge => WriteCause::TooLarge,
            FormatError::BadPayload => WriteCause::Io(io::ErrorKind::InvalidInput),
        },
    })?;
    match write_and_sync(&mut state, &inner.path, &bytes) {
        Ok(()) => {
            state.acked_len += u64::try_from(bytes.len()).unwrap_or(0);
            state.next_seq += 1;
            Ok(seq)
        }
        Err(error) => {
            if cleanup(&mut state, &inner.path).is_err() {
                inner.poisoned.store(true, Ordering::SeqCst);
            }
            Err(JournalError::WriteFailed {
                cause: write_cause(&error),
            })
        }
    }
}

/// Write at `acked_len` after an explicit seek, then the full sync. The failpoints sit where
/// the SDD section 4 names them.
fn write_and_sync(state: &mut WriteState, path: &Path, bytes: &[u8]) -> io::Result<()> {
    fault::check("journal.before_write", path)?;
    state.file.seek(SeekFrom::Start(state.acked_len))?;
    let (first, rest) = bytes.split_at(bytes.len() / 2);
    state.file.write_all(first)?;
    // `journal.partial_write`: the write stops part-way (an error here leaves half a record;
    // a kill here leaves it for the next owner's scan).
    fault::check("journal.partial_write", path)?;
    state.file.write_all(rest)?;
    fault::check("journal.after_write", path)?;
    fault::check("journal.before_fsync", path)?;
    fault::sync_file("journal.fsync_error", path, &state.file)?;
    fault::check("journal.after_fsync", path)
}

/// `set_len(acked_len)`, seek, sync: the file is as before the append (D-82 (2)).
fn cleanup(state: &mut WriteState, path: &Path) -> io::Result<()> {
    fault::check("journal.truncate_error", path)?;
    state.file.set_len(state.acked_len)?;
    state.file.seek(SeekFrom::Start(state.acked_len))?;
    state.file.sync_all()
}
