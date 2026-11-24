//! Child-process harness for the multi-process tests (parts B and later): the test binary runs
//! itself again (`current_exe`, `--exact <entry>`) with a scenario in environment variables. A
//! child reports with lines that start with `CHILD ` on standard output, and a `Kill` failpoint
//! prints `FAILPOINT-KILL <name>` on standard error (see `chess_store::fault`). The parent can
//! SIGKILL the child (`Child::kill`), close its standard input (a polite stop), or wait.
#![allow(dead_code, reason = "each test file uses a different subset")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration-test helper: aborts a failing fixture"
)]

use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

pub const KILL_MARKER: &str = "FAILPOINT-KILL";

pub struct Spawned {
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    kills: Receiver<String>,
    stderr_log: std::sync::Arc<std::sync::Mutex<String>>,
}

/// Starts `entry` (the name of a `#[test]` that acts as the child) with the variables set.
pub fn spawn(entry: &str, envs: &[(&str, &str)]) -> Spawned {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", entry, "--nocapture", "--test-threads=1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap();
    let stdin = child.stdin.take();
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let (line_tx, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            // libtest prints "test <name> ... " before the test runs, on the same line.
            if let Some(at) = line.find("CHILD ") {
                let _ = line_tx.send(line[at + 6..].to_owned());
            }
        }
    });
    let (kill_tx, kills) = mpsc::channel();
    let stderr_log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let log = std::sync::Arc::clone(&stderr_log);
    std::thread::spawn(move || {
        for line in BufReader::new(err).lines().map_while(Result::ok) {
            if let Ok(mut log) = log.lock() {
                log.push_str(&line);
                log.push('\n');
            }
            if let Some(name) = line.strip_prefix(&format!("{KILL_MARKER} ")) {
                let _ = kill_tx.send(name.to_owned());
            }
        }
    });
    Spawned {
        child,
        stdin,
        lines,
        kills,
        stderr_log,
    }
}

impl Spawned {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// The next `CHILD ` line (without the prefix), or `None` at the end of output or timeout.
    pub fn line(&self, timeout: Duration) -> Option<String> {
        self.lines.recv_timeout(timeout).ok()
    }

    /// Waits for a line that starts with `prefix`.
    pub fn expect(&self, prefix: &str) -> String {
        loop {
            let line = self.line(Duration::from_secs(60)).unwrap_or_else(|| {
                panic!(
                    "child never said {prefix:?}; its stderr:\n{}",
                    self.stderr_log
                        .lock()
                        .map(|l| l.clone())
                        .unwrap_or_default()
                )
            });
            if line.starts_with(prefix) {
                return line;
            }
        }
    }

    /// The failpoint a `Kill` action stopped the child at, if it got there.
    pub fn killed_at(&self, timeout: Duration) -> Option<String> {
        self.kills.recv_timeout(timeout).ok()
    }

    pub fn send(&mut self, text: &str) {
        use std::io::Write;
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(text.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    /// SIGKILL and reap.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Closes standard input (the child's cue to stop) and waits for its exit.
    pub fn finish(&mut self) -> std::process::ExitStatus {
        self.stdin.take();
        self.child.wait().unwrap()
    }
}

impl Drop for Spawned {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Blocks until standard input is closed (the child's idle loop).
pub fn wait_for_stdin_eof() {
    let mut sink = String::new();
    let stdin = std::io::stdin();
    while stdin.lock().read_line(&mut sink).is_ok_and(|n| n > 0) {
        sink.clear();
    }
}

/// Reads one line from standard input.
pub fn read_stdin_line() -> String {
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    line
}
