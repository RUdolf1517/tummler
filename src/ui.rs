use clap::ValueEnum;
use colored::Colorize;
use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};

/// 0 = успех, 1 = ошибка выполнения, 2 = неверные аргументы (как у GNU).
pub const EXIT_OK: i32 = 0;
pub const EXIT_FAIL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;

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

/// Цвет: `--color` > `--no-color` > NO_COLOR > FORCE_COLOR > TERM=dumb > TTY.
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
