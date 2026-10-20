use colored::Colorize;
use std::io::IsTerminal;
use std::process::Command;

/// Проверка наличия бинарника в PATH
pub fn is_available(bin: &str) -> bool {
    which::which(bin).is_ok()
}

/// Человекочитаемые подсказки по установке
pub fn install_hint(bin: &str) -> String {
    match bin {
        "ffmpeg" => format!(
            "Установите ffmpeg:\n  {}  brew install ffmpeg\n  {}  sudo apt install ffmpeg\n  {}  sudo dnf install ffmpeg\n  {}  https://ffmpeg.org/download.html",
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Fedora:".bold(),
            "Сайт:".bold()
        ),
        "ffprobe" => format!(
            "ffprobe обычно идёт вместе с ffmpeg:\n  {} brew install ffmpeg\n  {} sudo apt install ffmpeg",
            "macOS:".bold(),
            "Debian/Ubuntu:".bold()
        ),
        "pandoc" => format!(
            "Установите pandoc:\n  {}  brew install pandoc\n  {}  sudo apt install pandoc\n  {}  https://pandoc.org/installing.html",
            "macOS:".bold(),
            "Debian/Ubuntu:".bold(),
            "Сайт:".bold()
        ),
        _ => format!("Установите {bin} через пакетный менеджер вашей системы."),
    }
}

/// Попытка автоустановки (интерактивная, только в TTY)
pub fn prompt_auto_install(bin: &str, no_auto: bool) -> bool {
    eprintln!("{} {} не найден в PATH.", "⚠".yellow(), bin.bold());
    eprintln!("{}", install_hint(bin));

    if no_auto {
        return false;
    }

    if !std::io::stdin().is_terminal() {
        eprintln!(
            "{}",
            "Неинтерактивный режим — автоустановка пропущена.".dimmed()
        );
        return false;
    }

    let Some(cmd) = detect_install_command(bin) else {
        return false;
    };

    let answer = dialoguer::Confirm::new()
        .with_prompt(format!("Запустить `{cmd}` для установки {bin}?"))
        .default(true)
        .interact()
        .unwrap_or(false);

    if !answer {
        return false;
    }

    eprintln!("{} Запускаю: {}", "→".cyan(), cmd.dimmed());
    match Command::new("sh").arg("-c").arg(&cmd).status() {
        Ok(s) if s.success() => {
            if is_available(bin) {
                eprintln!("{} {} успешно установлен!", "✓".green(), bin.green());
                true
            } else {
                eprintln!(
                    "{} Установка завершилась, но {} всё ещё не найден. Проверьте PATH.",
                    "⚠".yellow(),
                    bin
                );
                false
            }
        }
        Ok(s) => {
            eprintln!("{} Команда завершилась с кодом {:?}", "✗".red(), s.code());
            false
        }
        Err(e) => {
            eprintln!("{} Не удалось запустить установщик: {}", "✗".red(), e);
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
        return None;
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
        return None;
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

/// Проверка и автоустановка если нужно
pub fn ensure_available(bin: &str, no_auto: bool) -> Result<(), String> {
    if is_available(bin) {
        return Ok(());
    }
    if prompt_auto_install(bin, no_auto) && is_available(bin) {
        return Ok(());
    }
    Err(format!("{} не найден. {}", bin, install_hint(bin)))
}

/// Вывести статус всех зависимостей
pub fn check_all() {
    println!("{}", "Зависимости tummler:".bold());
    println!();

    let deps = [
        ("ffmpeg", "Аудио/видео конвертация", true),
        ("ffprobe", "Анализ медиафайлов (опционально)", false),
        ("pandoc", "Конвертация документов", true),
    ];

    for (bin, desc, required) in deps {
        let available = is_available(bin);
        let status = if available {
            "✓ установлен".green()
        } else if required {
            "✗ не найден".red()
        } else {
            "○ не найден (опционально)".yellow()
        };
        println!("  {:12}  {}  — {}", bin.bold(), status, desc);
        if !available {
            println!(
                "    {}",
                install_hint(bin).lines().next().unwrap_or("").dimmed()
            );
        }
    }

    println!();
    println!(
        "  {} {}",
        "Нативные (без зависимостей):".bold(),
        "картинки (image crate), архивы (zip/tar)".dimmed()
    );
    println!();

    println!("  {} {}", "OS:".dimmed(), std::env::consts::OS);
    println!("  {} {}", "Arch:".dimmed(), std::env::consts::ARCH);
}
