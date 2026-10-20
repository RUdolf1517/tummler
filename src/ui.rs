use clap::ValueEnum;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 0 = success, 1 = conversion error, 2 = invalid arguments (GNU-style).
pub const EXIT_OK: i32 = 0;
pub const EXIT_FAIL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✔"];

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    #[default]
    Auto,
    Always,
    Never,
}

static QUIET: AtomicBool = AtomicBool::new(false);

pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

pub fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

fn env_nonempty(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

/// Color: `--color` > `--no-color` > NO_COLOR > FORCE_COLOR > TERM=dumb > TTY.
pub fn init_color(when: ColorWhen, no_color_flag: bool) {
    let when = if no_color_flag { ColorWhen::Never } else { when };
    match when {
        ColorWhen::Always => colored::control::set_override(true),
        ColorWhen::Never => colored::control::set_override(false),
        ColorWhen::Auto => {
            if env_nonempty("NO_COLOR") || std::env::var("TERM").unwrap_or_default() == "dumb" {
                colored::control::set_override(false);
            } else if env_nonempty("FORCE_COLOR") {
                colored::control::set_override(true);
            }
        }
    }
}

pub fn stderr_is_tty() -> bool {
    io::stderr().is_terminal()
}

pub fn wants_animation() -> bool {
    !is_quiet() && stderr_is_tty()
}

pub fn error(msg: impl std::fmt::Display) {
    eprintln!("{} {msg}", "tummler:".red());
}

pub fn warn(msg: impl std::fmt::Display) {
    if is_quiet() {
        return;
    }
    eprintln!("{} {msg}", "tummler:".yellow());
}

pub fn info(msg: impl std::fmt::Display) {
    if is_quiet() {
        return;
    }
    eprintln!("tummler: {msg}");
}

pub fn usage_exit(msg: impl std::fmt::Display) -> ! {
    error(msg);
    std::process::exit(EXIT_USAGE);
}

/// Spinner for a single conversion.
pub fn spinner(msg: impl Into<String>) -> Option<ProgressBar> {
    if !wants_animation() {
        return None;
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(SPINNER_FRAMES)
            .template("{spinner:.cyan} {msg}")
            .expect("spinner template"),
    );
    pb.enable_steady_tick(Duration::from_millis(70));
    pb.set_message(msg.into());
    Some(pb)
}

/// Batch progress bar with a spinner.
pub fn batch_bar(len: u64) -> Option<ProgressBar> {
    if !wants_animation() {
        return None;
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .tick_strings(SPINNER_FRAMES)
            .template("{spinner:.cyan} {elapsed_precise} [{bar:28.cyan/blue}] {pos}/{len} {per_sec} {wide_msg}")
            .expect("bar template")
            .progress_chars("█▉▊▋▌▍▎▏ "),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    Some(pb)
}

pub fn finish_ok(pb: Option<&ProgressBar>, msg: impl Into<String>) {
    if let Some(pb) = pb {
        pb.finish_with_message(msg.into());
    }
}

pub fn finish_clear(pb: Option<&ProgressBar>) {
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
}
