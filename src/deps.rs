use colored::Colorize;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;
#[cfg(windows)]
use std::process::Stdio;
use std::sync::OnceLock;

/// Check whether a binary is on PATH.
pub fn is_available(bin: &str) -> bool {
    which::which(bin).is_ok()
}

/// LibreOffice, if the user already installed it. Never auto-installed.
pub fn soffice_bin() -> Option<PathBuf> {
    static CACHED: OnceLock<Option<PathBuf>> = OnceLock::new();
    CACHED.get_or_init(find_soffice).clone()
}

fn find_soffice() -> Option<PathBuf> {
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
    #[cfg(windows)]
    {
        for p in [
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ] {
            let p = PathBuf::from(p);
            if p.is_file() {
                return Some(p);
            }
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
            "Install ffmpeg:\n  {}  winget install -e --id Gyan.FFmpeg\n  {}  brew install ffmpeg\n  {}  sudo apt install ffmpeg\n  {}  sudo dnf install ffmpeg\n  {}  https://ffmpeg.org/download.html",
            "Windows:".bold(),
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Fedora:".bold(),
            "Website:".bold()
        ),
        "pandoc" => format!(
            "Install pandoc:\n  {}  winget install -e --id JohnMacFarlane.Pandoc\n  {}  brew install pandoc\n  {}  sudo apt install pandoc\n  {}  sudo dnf install pandoc\n  {}  https://pandoc.org/installing.html",
            "Windows:".bold(),
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Fedora:".bold(),
            "Website:".bold()
        ),
        "pdftoppm" | "pdftotext" => format!(
            "Install poppler ({bin}):\n  {}  winget install -e --id oschwartz10612.Poppler\n  {}  brew install poppler\n  {}  sudo apt install poppler-utils\n  {}  sudo dnf install poppler-utils\n  {}  https://poppler.freedesktop.org/",
            "Windows:".bold(),
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Fedora:".bold(),
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

    confirm_and_install(bin)
}

fn confirm_and_install(bin: &str) -> bool {
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
    let status = {
        #[cfg(windows)]
        {
            Command::new("cmd").args(["/C", cmd]).status()
        }
        #[cfg(not(windows))]
        {
            Command::new("sh").arg("-c").arg(cmd).status()
        }
    };
    match status {
        Ok(s) if s.success() => {
            #[cfg(windows)]
            refresh_path();
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

fn pkg_name(bin: &str, brew: bool, pacman: bool) -> &str {
    match bin {
        "pdftoppm" | "pdftotext" if brew || pacman => "poppler",
        "pdftoppm" | "pdftotext" => "poppler-utils",
        other => other,
    }
}

fn winget_id(bin: &str) -> Option<&'static str> {
    Some(match bin {
        "ffmpeg" => "Gyan.FFmpeg",
        "pandoc" => "JohnMacFarlane.Pandoc",
        "pdftoppm" | "pdftotext" => "oschwartz10612.Poppler",
        _ => return None,
    })
}

#[cfg(windows)]
fn refresh_path() {
    let Ok(out) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')",
        ])
        .output()
    else {
        return;
    };
    if !out.status.success() {
        return;
    }
    let path = String::from_utf8_lossy(&out.stdout);
    let path = path.trim();
    if !path.is_empty() {
        // PATH from the registry after winget; no other threads should be converting yet.
        unsafe { std::env::set_var("PATH", path) };
    }
}

#[cfg(windows)]
fn winget_available() -> bool {
    which::which("winget").is_ok()
        || Command::new("winget")
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

fn detect_install_command(bin: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if which::which("brew").is_ok() {
            return Some(format!("brew install {}", pkg_name(bin, true, false)));
        }
        None
    }

    #[cfg(target_os = "linux")]
    {
        // Native repos first so linuxbrew does not shadow apt/dnf.
        if which::which("apt").is_ok() || which::which("apt-get").is_ok() {
            let apt = if which::which("apt").is_ok() {
                "apt"
            } else {
                "apt-get"
            };
            return Some(format!(
                "sudo {apt} update && sudo {apt} install -y {}",
                pkg_name(bin, false, false)
            ));
        }
        if which::which("dnf").is_ok() {
            return Some(format!(
                "sudo dnf install -y {}",
                pkg_name(bin, false, false)
            ));
        }
        if which::which("pacman").is_ok() {
            return Some(format!(
                "sudo pacman -S --noconfirm {}",
                pkg_name(bin, false, true)
            ));
        }
        if which::which("zypper").is_ok() {
            return Some(format!(
                "sudo zypper install -y {}",
                pkg_name(bin, false, false)
            ));
        }
        if which::which("brew").is_ok() {
            return Some(format!("brew install {}", pkg_name(bin, true, false)));
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        let id = winget_id(bin)?;
        if winget_available() {
            return Some(format!(
                "winget install -e --id {id} --accept-package-agreements --accept-source-agreements"
            ));
        }
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = bin;
        None
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

/// Tools tummler may offer to install. LibreOffice is never in this list.
const AUTO_BINS: &[&str] = &["ffmpeg", "pandoc", "pdftoppm", "pdftotext"];

/// Print status of all dependencies. In a TTY, offer to install missing auto-deps.
pub fn check_all(no_auto: bool, assume_yes: bool) {
    print_dep_table();
    offer_auto_deps(no_auto, assume_yes);
}

fn print_dep_table() {
    println!("{}", "tummler dependencies:".bold());
    println!();

    let deps = [
        ("ffmpeg", "Audio/video conversion"),
        ("pandoc", "Document conversion"),
        ("pdftoppm", "PDF → png/jpg (pages stacked, or --page N)"),
        ("pdftotext", "PDF → text / docx fallback"),
    ];

    for (bin, desc) in deps {
        let available = is_available(bin);
        let status = if available {
            "✓ installed".green()
        } else {
            "✗ not found".red()
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
        "  {:12}  {lo_status}  — slides; Word/ODT → pdf; PDF → docx if present",
        "soffice".bold()
    );

    println!();
    println!(
        "  {} {}",
        "Native (no extra install):".bold(),
        "images, archives, spreadsheets (xlsx↔csv)".dimmed()
    );
    println!();

    println!("  {} {}", "OS:".dimmed(), std::env::consts::OS);
    println!("  {} {}", "Arch:".dimmed(), std::env::consts::ARCH);
}

fn offer_auto_deps(no_auto: bool, assume_yes: bool) {
    if no_auto {
        return;
    }
    if AUTO_BINS.iter().all(|b| is_available(b)) {
        return;
    }
    if assume_yes {
        let mut any = false;
        for bin in AUTO_BINS {
            if is_available(bin) {
                continue;
            }
            any |= try_install(bin);
        }
        if any {
            println!();
            print_dep_table();
        }
        return;
    }
    if !std::io::stdin().is_terminal() {
        eprintln!(
            "{}",
            "Non-interactive mode — skipping auto-install.".dimmed()
        );
        return;
    }
    println!();
    let mut any = false;
    for bin in AUTO_BINS {
        if is_available(bin) {
            continue;
        }
        any |= confirm_and_install(bin);
    }
    if any {
        println!();
        print_dep_table();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_bins_never_include_soffice() {
        assert!(!AUTO_BINS.contains(&"soffice"));
        assert!(!AUTO_BINS.contains(&"libreoffice"));
        assert_eq!(AUTO_BINS[0], "ffmpeg");
    }

    #[test]
    fn poppler_pkg_name_apt_dnf_vs_brew() {
        assert_eq!(pkg_name("pdftoppm", false, false), "poppler-utils");
        assert_eq!(pkg_name("pdftotext", false, false), "poppler-utils");
        assert_eq!(pkg_name("pdftoppm", true, false), "poppler");
        assert_eq!(pkg_name("ffmpeg", false, false), "ffmpeg");
        assert_eq!(pkg_name("pandoc", false, false), "pandoc");
        assert_eq!(winget_id("ffmpeg"), Some("Gyan.FFmpeg"));
        assert_eq!(winget_id("pandoc"), Some("JohnMacFarlane.Pandoc"));
        assert_eq!(winget_id("pdftoppm"), Some("oschwartz10612.Poppler"));
        assert_eq!(winget_id("soffice"), None);
    }

    #[test]
    fn check_all_no_auto_does_not_install() {
        check_all(true, false);
    }
}
