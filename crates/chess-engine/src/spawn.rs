//! DD-EN-005: process launch. The real spawner uses `tokio::process`; tests substitute a
//! scripted one through the `Spawner` trait.

use std::ffi::OsString;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::{Child, Command};

use crate::error::EngineError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitInfo {
    pub code: Option<i32>,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Control half of a spawned process.
pub trait ProcessControl: Send {
    /// `Some` once the process has exited.
    fn try_wait(&mut self) -> Option<ExitInfo>;
    /// Asks the OS to kill it (SIGKILL). Does not wait.
    fn start_kill(&mut self);
    /// Reaps the process.
    fn wait(&mut self) -> BoxFuture<'_, ExitInfo>;
    /// The OS process id, when there is one.
    fn id(&self) -> Option<u32>;
}

pub struct SpawnedProcess {
    pub stdin: Box<dyn AsyncWrite + Send + Unpin>,
    pub stdout: Box<dyn AsyncRead + Send + Unpin>,
    pub control: Box<dyn ProcessControl>,
}

pub trait Spawner: Send + Sync {
    fn spawn(&self, path: &Path) -> Result<SpawnedProcess, EngineError>;
}

/// The production spawner (SDD 3.10.5): no arguments, an empty environment, the binary's
/// directory as the working directory, stdin and stdout piped, stderr to null (a full unread
/// pipe could deadlock the engine, and engine text can hold positions), `kill_on_drop`, and its
/// own process group so a terminal Ctrl-C reaches `chess` and not Stockfish.
#[derive(Debug, Clone, Default)]
pub struct ProcessSpawner {
    args: Vec<OsString>,
}

impl ProcessSpawner {
    /// Production: no arguments.
    pub fn new() -> ProcessSpawner {
        ProcessSpawner::default()
    }

    /// Tests only: lets a test select a mode of the `fake_uci` executable. Production code
    /// never passes arguments (choice F-E3).
    pub fn with_args(args: Vec<OsString>) -> ProcessSpawner {
        ProcessSpawner { args }
    }
}

impl Spawner for ProcessSpawner {
    fn spawn(&self, path: &Path) -> Result<SpawnedProcess, EngineError> {
        let mut cmd = Command::new(path);
        cmd.args(&self.args)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .process_group(0);
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            cmd.current_dir(dir);
        }
        let mut child = cmd.spawn().map_err(|_| EngineError::SpawnFailed)?;
        let stdin = child.stdin.take().ok_or(EngineError::SpawnFailed)?;
        let stdout = child.stdout.take().ok_or(EngineError::SpawnFailed)?;
        Ok(SpawnedProcess {
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
            control: Box::new(ChildControl { child }),
        })
    }
}

struct ChildControl {
    child: Child,
}

impl ProcessControl for ChildControl {
    fn try_wait(&mut self) -> Option<ExitInfo> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(ExitInfo {
                code: status.code(),
            }),
            // Running, or the OS could not say: the next read or write will tell.
            Ok(None) | Err(_) => None,
        }
    }

    fn start_kill(&mut self) {
        // An error means the process is already gone; there is nothing more to do.
        let _ = self.child.start_kill();
    }

    fn wait(&mut self) -> BoxFuture<'_, ExitInfo> {
        Box::pin(async move {
            match self.child.wait().await {
                Ok(status) => ExitInfo {
                    code: status.code(),
                },
                Err(_) => ExitInfo { code: None },
            }
        })
    }

    fn id(&self) -> Option<u32> {
        self.child.id()
    }
}
