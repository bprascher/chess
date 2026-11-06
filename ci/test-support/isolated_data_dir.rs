//! Isolated `CHESS_DATA_DIR` helper for tests (test-strategy "Determinism":
//! every automated run uses an isolated data store, never the user's).
//!
//! Std only, no third-party crate. One source file, included by a crate's tests with
//!
//! ```text
//! #[path = "<relative path to>/ci/test-support/isolated_data_dir.rs"]
//! mod isolated_data_dir;
//! ```
//!
//! Rules: the directory is created fresh under the OS temp directory with a unique
//! name (process id, counter, clock), is removed on drop, and removal refuses any
//! path that is not inside the canonical temp directory with the helper's prefix.
//! The helper never reads the user's real data directory and never changes the
//! process environment (`set_var` is `unsafe` in edition 2024 and races between
//! tests). Pass the variable to a child process or to a resolver argument instead.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The environment variable that selects the data directory (architecture section 7).
pub const ENV_VAR: &str = "CHESS_DATA_DIR";

const PREFIX: &str = "chess-test-";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A temporary data directory that belongs to one test.
#[derive(Debug)]
pub struct IsolatedDataDir {
    path: PathBuf,
    root: PathBuf,
}

impl IsolatedDataDir {
    /// Creates a new empty directory. `label` (for example the test name) is
    /// reduced to ASCII letters, digits, `-` and `_` so it cannot escape the temp root.
    pub fn new(label: &str) -> io::Result<Self> {
        let root = fs::canonicalize(std::env::temp_dir())?;
        let clean: String = label
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .take(40)
            .collect();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("{PREFIX}{clean}-{}-{n}-{nanos}", std::process::id());
        let path = root.join(name);
        // create_dir (not create_dir_all) fails if the name already exists.
        fs::create_dir(&path)?;
        Ok(Self { path, root })
    }

    /// The directory to use as the data directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The `(name, value)` pair for `Command::env` when starting a child process.
    pub fn env(&self) -> (&'static str, &Path) {
        (ENV_VAR, &self.path)
    }

    fn is_removable(&self) -> bool {
        self.path.parent() == Some(self.root.as_path())
            && self
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(PREFIX))
    }
}

impl Drop for IsolatedDataDir {
    fn drop(&mut self) {
        if self.is_removable() {
            // Best effort: a leftover directory in the temp root is harmless.
            drop(fs::remove_dir_all(&self.path));
        }
    }
}
