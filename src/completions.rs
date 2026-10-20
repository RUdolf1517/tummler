use clap::{Command, ValueEnum};
use clap_complete::{generate, Shell};
use std::io;

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum, Debug)]
#[value(rename_all = "lower")]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

impl ShellKind {
    pub fn to_shell(self) -> Shell {
        match self {
            ShellKind::Bash => Shell::Bash,
            ShellKind::Zsh => Shell::Zsh,
            ShellKind::Fish => Shell::Fish,
            ShellKind::Powershell => Shell::PowerShell,
            ShellKind::Elvish => Shell::Elvish,
        }
    }
}

pub fn generate_completions(shell: ShellKind, cmd: &mut Command) {
    generate(shell.to_shell(), cmd, "tummler", &mut io::stdout());
}
