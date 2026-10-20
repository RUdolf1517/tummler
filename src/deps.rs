use colored::Colorize;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;

/// Check whether a binary is on PATH.
pub fn is_available(bin: &str) -> bool {
    which::which(bin).is_ok()
}

/// LibreOffice, if the user already installed it. Never auto-installed.
pub fn soffice_bin() -> Option<PathBuf> {
    for name in ["soffice", "libreoffice"] {
        if let Ok(p) = which::which(name) {
            return Some(p);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let bundled =
            PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice");
        if bundled.is_file() {
            return Some(bundled);
        }
    }
    None
}

/// Slide conversions need soffice on PATH. tummler will not install LibreOffice.
pub fn require_soffice() -> Result<PathBuf, String> {
    soffice_bin().ok_or_else(|| {
        "LibreOffice (soffice) not found in PATH. tummler does not install it; slide conversions (pptx/odp/ppt → pdf and between decks) need an existing install.".into()
    })
}

/// Human-readable install hints.
pub fn install_hint(bin: &str) -> String {
    match bin {
        "ffmpeg" => format!(
            "Install ffmpeg:\n  {}  brew install ffmpeg\n  {}  sudo apt install ffmpeg\n  {}  sudo dnf install ffmpeg\n  {}  https://ffmpeg.org/download.html",
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Fedora:".bold(),
            "Website:".bold()
        ),
        "ffprobe" => format!(
            "ffprobe usually ships with ffmpeg:\n  {} brew install ffmpeg\n  {} sudo apt install ffmpeg",
            "macOS:".bold(),
            "Debian/Ubuntu:".bold()
        ),
        "pandoc" => format!(
            "Install pandoc:\n  {}  brew install pandoc\n  {}  sudo apt install pandoc\n  {}  https://pandoc.org/installing.html",
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Website:".bold()
        ),
        _ => format!("Install {bin} with your system package manager."),
    }
}

/// Interactive auto-install, TTY only.
pub fn prompt_auto_install(bin: &str, no_auto: bool) -> bool {
    eprintln!("{} {} not found in PATH.", "⚠".yellow(), bin.bold());
    eprintln!("{}", install_hint(bin));

    if no_auto {
        return false;
    }

    if !std::io::stdin().is_terminal() {
        eprintln!(
            "{}",
            "Non-interactive mode — skipping auto-install.".dimmed()
        );
        return false;
    }

    let Some(cmd) = detect_install_command(bin) else {
        return false;
    };

    let answer = dialoguer::Confirm::new()
        .with_prompt(format!("Run `{cmd}` to install {bin}?"))
        .default(true)
        .interact()
        .unwrap_or(false);

    if !answer {
        return false;
    }

    run_install(bin, &cmd)
}

fn try_install(bin: &str) -> bool {
    let Some(cmd) = detect_install_command(bin) else {
        eprintln!("{} no package manager available to auto-install {bin}", "tummler:".yellow());
        return false;
    };
    run_install(bin, &cmd)
}

fn run_install(bin: &str, cmd: &str) -> bool {
    eprintln!("{} Running: {}", "→".cyan(), cmd.dimmed());
    match Command::new("sh").arg("-c").arg(cmd).status() {
        Ok(s) if s.success() => {
            if is_available(bin) {
                eprintln!("{} {} installed", "✓".green(), bin.green());
                true
            } else {
                eprintln!(
                    "{} install finished but {} is still missing. Check PATH.",
                    "⚠".yellow(),
                    bin
                );
                false
            }
        }
        Ok(s) => {
            eprintln!("{} command exited with code {:?}", "✗".red(), s.code());
            false
        }
        Err(e) => {
            eprintln!("{} failed to run installer: {}", "✗".red(), e);
            false
        }
    }
}

fn detect_install_command(bin: &str) -> Option<String> {
    let pkg = bin_for_pkg(bin);

    #[cfg(target_os = "macos")]
    {
        if which::which("brew").is_ok() {
            return Some(format!("brew install {pkg}"));
        }
        None
    }

    #[cfg(target_os = "linux")]
    {
        if which::which("brew").is_ok() {
            return Some(format!("brew install {pkg}"));
        }
        if which::which("apt").is_ok() || which::which("apt-get").is_ok() {
            return Some(format!("sudo apt update && sudo apt install -y {pkg}"));
        }
        if which::which("dnf").is_ok() {
            return Some(format!("sudo dnf install -y {pkg}"));
        }
        if which::which("pacman").is_ok() {
            return Some(format!("sudo pacman -S --noconfirm {pkg}"));
        }
        if which::which("zypper").is_ok() {
            return Some(format!("sudo zypper install -y {pkg}"));
        }
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = pkg;
        None
    }
}

fn bin_for_pkg(bin: &str) -> &str {
    match bin {
        "ffprobe" => "ffmpeg",
        other => other,
    }
}

/// Check and auto-install if needed.
pub fn ensure_available(bin: &str, no_auto: bool, assume_yes: bool) -> Result<(), String> {
    if is_available(bin) {
        return Ok(());
    }
    if no_auto {
        eprintln!("{} {} not found in PATH.", "⚠".yellow(), bin.bold());
        eprintln!("{}", install_hint(bin));
        return Err(format!("{bin} not found. {}", install_hint(bin)));
    }
    if assume_yes {
        if try_install(bin) && is_available(bin) {
            return Ok(());
        }
        return Err(format!("{bin} not found. {}", install_hint(bin)));
    }
    if prompt_auto_install(bin, false) && is_available(bin) {
        return Ok(());
    }
    Err(format!("{bin} not found. {}", install_hint(bin)))
}

/// Print status of all dependencies.
pub fn check_all() {
    println!("{}", "tummler dependencies:".bold());
    println!();

    let deps = [
        ("ffmpeg", "Audio/video conversion", true),
        ("ffprobe", "Media inspection (optional)", false),
        ("pandoc", "Document conversion", true),
    ];

    for (bin, desc, required) in deps {
        let available = is_available(bin);
        let status = if available {
            "✓ installed".green()
        } else if required {
            "✗ not found".red()
        } else {
            "○ not found (optional)".yellow()
        };
        println!("  {:12}  {}  — {}", bin.bold(), status, desc);
        if !available {
            println!(
                "    {}",
                install_hint(bin).lines().next().unwrap_or("").dimmed()
            );
        }
    }

    let lo = soffice_bin().is_some();
    let lo_status = if lo {
        "✓ installed".green()
    } else {
        "○ not found (optional, never auto-installed)".yellow()
    };
    println!(
        "  {:12}  {lo_status}  — slides (pptx/odp/ppt) if you already have LibreOffice",
        "soffice".bold()
    );

    println!();
    println!(
        "  {} {}",
        "Native (no extra install):".bold(),
        "images, archives, spreadsheets (xlsx→csv/txt)".dimmed()
    );
    println!();

    println!("  {} {}", "OS:".dimmed(), std::env::consts::OS);
    println!("  {} {}", "Arch:".dimmed(), std::env::consts::ARCH);
}
