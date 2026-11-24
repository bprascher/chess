//! Whether a process id exists (SDD 3.11.6: "`kill(pid, 0)` succeeds or fails with `EPERM`").
//!
//! The standard library has no way to signal an arbitrary pid, and D-107 admits neither `libc`
//! nor `rustix` nor `unsafe`. The check therefore runs `/bin/kill -0 <pid>` (the signal 0 of
//! the SDD, sent by the system's own `kill`). Exit status 0 means the process exists; "No such
//! process" on its standard error means it does not; "Operation not permitted" (`EPERM`) means
//! it exists and belongs to someone else. **Anything else (the tool cannot be started, an
//! unknown message) counts as "exists"**, so a failure of the check can only make the store
//! more cautious: the registry then falls back to the heartbeat age alone (SDD 3.11.6).

use std::process::{Command, Stdio};

/// True when a process with this id exists (or when that cannot be told).
pub(crate) fn pid_alive(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    let output = Command::new("/bin/kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    match output {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            let message = String::from_utf8_lossy(&out.stderr);
            !message.contains("No such process")
        }
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_process_and_a_running_child_exist_and_a_reaped_child_does_not() {
        assert!(pid_alive(std::process::id()));
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        assert!(pid_alive(pid), "a running child");
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(!pid_alive(pid), "a reaped child");
    }

    #[test]
    fn a_process_of_another_user_exists_by_eperm() {
        // pid 1 (launchd) belongs to root: `kill -0` answers EPERM, which means "exists".
        assert!(pid_alive(1));
    }
}
