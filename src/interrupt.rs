use crate::ui::EXIT_SIGINT;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static INSTALLED: AtomicBool = AtomicBool::new(false);

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
        libc::kill(pid as i32, libc::SIGKILL);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

/// Like `Command::output()`, but the child is killed on Ctrl-C.
pub fn run(cmd: &mut Command) -> std::io::Result<Output> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let pid = child.id();
    if let Ok(mut g) = pids().lock() {
        g.push(pid);
    }
    let result = child.wait_with_output();
    if let Ok(mut g) = pids().lock() {
        g.retain(|p| *p != pid);
    }
    result
}
