use crate::error::TummlerError;
use crate::ui::EXIT_SIGINT;
use std::cell::Cell;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static INSTALLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
    static LIMIT_SECS: Cell<u64> = const { Cell::new(0) };
}

fn pids() -> &'static Mutex<Vec<u32>> {
    static P: OnceLock<Mutex<Vec<u32>>> = OnceLock::new();
    P.get_or_init(|| Mutex::new(Vec::new()))
}

/// Ctrl-C kills tracked ffmpeg/pandoc children and exits 130.
pub fn install() {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = ctrlc::set_handler(|| {
        kill_all();
        std::process::exit(EXIT_SIGINT);
    });
}

fn kill_all() {
    if let Ok(guard) = pids().lock() {
        for pid in guard.iter().copied() {
            kill_pid(pid);
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    unsafe {
        let p = pid as i32;
        // Child is in its own process group (`process_group(0)`); kill the group
        // so soffice.bin / ffmpeg helpers die too.
        libc::kill(-p, libc::SIGKILL);
        libc::kill(p, libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn register(pid: u32) {
    if let Ok(mut g) = pids().lock() {
        g.push(pid);
    }
}

fn unregister(pid: u32) {
    if let Ok(mut g) = pids().lock() {
        g.retain(|p| *p != pid);
    }
}

/// Wall-clock budget for one input file. Cleared when the guard drops.
#[must_use = "deadline is cleared when the guard is dropped"]
pub struct FileDeadline {
    _priv: (),
}

impl Drop for FileDeadline {
    fn drop(&mut self) {
        DEADLINE.with(|c| c.set(None));
        LIMIT_SECS.with(|c| c.set(0));
    }
}

/// Start a per-file timer. `None` / `0` = no limit.
pub fn begin_file(timeout_secs: Option<u64>) -> FileDeadline {
    match timeout_secs {
        Some(s) if s > 0 => {
            DEADLINE.with(|c| c.set(Some(Instant::now() + Duration::from_secs(s))));
            LIMIT_SECS.with(|c| c.set(s));
        }
        _ => {
            DEADLINE.with(|c| c.set(None));
            LIMIT_SECS.with(|c| c.set(0));
        }
    }
    FileDeadline { _priv: () }
}

fn remaining() -> Option<Duration> {
    DEADLINE.with(|c| c.get().map(|dl| dl.saturating_duration_since(Instant::now())))
}

fn limit_secs() -> u64 {
    LIMIT_SECS.with(|c| c.get())
}

fn timed_out() -> TummlerError {
    TummlerError::TimedOut(limit_secs().max(1))
}

/// Like `Command::output()`, but the child is killed on Ctrl-C and when the
/// file deadline from [`begin_file`] expires.
pub fn run(cmd: &mut Command) -> Result<Output, TummlerError> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    if remaining() == Some(Duration::ZERO) {
        return Err(timed_out());
    }

    let child = cmd.spawn()?;
    let pid = child.id();
    register(pid);

    if let Some(wait_for) = remaining() {
        wait_timeout(child, wait_for, pid)
    } else {
        let result = child.wait_with_output();
        unregister(pid);
        result.map_err(TummlerError::from)
    }
}

fn wait_timeout(child: Child, wait_for: Duration, pid: u32) -> Result<Output, TummlerError> {
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let r = child.wait_with_output();
        let _ = tx.send(());
        r
    });
    match rx.recv_timeout(wait_for) {
        Ok(()) => {
            unregister(pid);
            handle
                .join()
                .unwrap_or_else(|_| Err(std::io::Error::other("child waiter panicked")))
                .map_err(TummlerError::from)
        }
        Err(_) => {
            kill_pid(pid);
            let _ = handle.join();
            unregister(pid);
            Err(timed_out())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_cmd() -> Command {
        #[cfg(windows)]
        {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", "exit", "0"]);
            cmd
        }
        #[cfg(not(windows))]
        {
            Command::new("true")
        }
    }

    #[test]
    fn timeout_kills_sleep() {
        let _d = begin_file(Some(1));
        #[cfg(windows)]
        let mut cmd = {
            let mut c = Command::new("ping");
            c.args(["-n", "30", "127.0.0.1"]);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = {
            let mut c = Command::new("sleep");
            c.arg("30");
            c
        };
        assert!(matches!(run(&mut cmd), Err(TummlerError::TimedOut(1))));
    }

    #[test]
    fn timeout_allows_fast_command() {
        let _d = begin_file(Some(5));
        let mut cmd = ok_cmd();
        let out = run(&mut cmd).expect("ok cmd");
        assert!(out.status.success());
    }

    #[test]
    fn no_deadline_runs() {
        let _d = begin_file(None);
        let mut cmd = ok_cmd();
        let out = run(&mut cmd).expect("ok cmd");
        assert!(out.status.success());
    }
}
