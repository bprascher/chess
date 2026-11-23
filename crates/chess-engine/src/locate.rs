//! DD-EN-006: finds the bundled engine. Never reads `PATH`, never downloads (D-18).

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::EngineError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinarySource {
    Bundled,
    /// `CHESS_STOCKFISH`: dev and tests only. The `id name` check is skipped.
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedBinary {
    pub path: PathBuf,
    pub source: BinarySource,
}

#[derive(Debug, Clone)]
pub struct BinaryLocator {
    env_override: Option<OsString>,
    current_exe: PathBuf,
}

impl BinaryLocator {
    /// For tests and for `from_environment`: the override value and the running executable.
    pub fn new(env_override: Option<OsString>, current_exe: PathBuf) -> BinaryLocator {
        BinaryLocator {
            env_override,
            current_exe,
        }
    }

    /// Reads `CHESS_STOCKFISH` and the current executable. These are the only two inputs.
    pub fn from_environment() -> Result<BinaryLocator, EngineError> {
        let exe = std::env::current_exe().map_err(|_| EngineError::BinaryMissing)?;
        Ok(BinaryLocator::new(std::env::var_os("CHESS_STOCKFISH"), exe))
    }

    pub fn locate(&self) -> Result<LocatedBinary, EngineError> {
        match self.env_override.as_ref().filter(|v| !v.is_empty()) {
            Some(value) => {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err(EngineError::InvalidOverride);
                }
                check_file(&path)?;
                Ok(LocatedBinary {
                    path,
                    source: BinarySource::Override,
                })
            }
            None => {
                // Follows the install-cli symlink.
                let exe = self
                    .current_exe
                    .canonicalize()
                    .map_err(|_| EngineError::BinaryMissing)?;
                let dir = exe.parent().ok_or(EngineError::BinaryMissing)?;
                let path = dir.join("stockfish");
                check_file(&path)?;
                Ok(LocatedBinary {
                    path,
                    source: BinarySource::Bundled,
                })
            }
        }
    }
}

/// Exists, regular file, owner-executable, not group- or world-writable.
fn check_file(path: &Path) -> Result<(), EngineError> {
    let meta = fs::metadata(path).map_err(|_| EngineError::BinaryMissing)?;
    if !meta.is_file() {
        return Err(EngineError::NotExecutable);
    }
    let mode = meta.permissions().mode();
    if mode & 0o100 == 0 || mode & 0o022 != 0 {
        return Err(EngineError::NotExecutable);
    }
    Ok(())
}
