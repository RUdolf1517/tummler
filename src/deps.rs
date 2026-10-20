use colored::Colorize;
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

/// Попытка автоустановки (интерактивная)
pub fn prompt_auto_install(bin: &str, no_auto: bool) -> bool {
    if no_auto {
        eprintln!("{} {}", "✗".red(), format!("{bin} не найден").red());
        eprintln!("{}", install_hint(bin));
        return false;
    }

    eprintln!(
        "{} {} не найден в PATH.",
        "⚠".yellow(),
        bin.bold()
    );
    eprintln!("{}", install_hint(bin));

    // Определяем пакетный менеджер
    let (os, install_cmd) = detect_install_command(bin);
    if let Some(cmd) = install_cmd {
        eprintln!("\n{} Попробовать автоустановку? [Y/n]: ", "?".cyan());
        // Читаем ввод
        let mut input = String::new();
        // Используем dialoguer если доступен, иначе stdin
        let answer = if dialoguer::Confirm::new()
            .with_prompt(format!("Запустить `{cmd}` для установки {bin}?"))
            .default(true)
            .interact()
            .unwrap_or(false)
        {
            true
        } else {
            // fallback: уже спросил, но если dialoguer не сработал
            let _ = std::io::stdin().read_line(&mut input);
            input.trim().to_lowercase() != "n" && input.trim().to_lowercase() != "no"
        };

        if answer {
            eprintln!("{} Запускаю: {}", "→".cyan(), cmd.dimmed());
            let status = if os == "macos" {
                Command::new("sh").arg("-c").arg(&cmd).status()
            } else {
                Command::new("sh").arg("-c").arg(&cmd).status()
            };
            match status {
                Ok(s) if s.success() => {
                    if is_available(bin) {
                        eprintln!("{} {} успешно установлен!", "✓".green(), bin.green());
                        return true;
                    } else {
                        eprintln!("{} Установка завершилась, но {} всё ещё не найден. Проверьте PATH.", "⚠".yellow(), bin);
                        return false;
                    }
                }
                Ok(s) => {
                    eprintln!("{} Команда завершилась с кодом {:?}", "✗".red(), s.code());
                    return false;
                }
                Err(e) => {
                    eprintln!("{} Не удалось запустить установщик: {}", "✗".red(), e);
                    return false;
                }
            }
        }
    }

    false
}

fn detect_install_command(bin: &str) -> (&'static str, Option<String>) {
    #[cfg(target_os = "macos")]
    {
        if which::which("brew").is_ok() {
            return ("macos", Some(format!("brew install {}", bin_for_brew(bin))));
        }
        return ("macos", None);
    }
    #[cfg(target_os = "linux")]
    {
        if which::which("brew").is_ok() {
            return ("linux", Some(format!("brew install {}", bin_for_brew(bin))));
        }
        if which::which("apt").is_ok() || which::which("apt-get").is_ok() {
            return ("linux", Some(format!("sudo apt update && sudo apt install -y {}", bin)));
        }
        if which::which("dnf").is_ok() {
            return ("linux", Some(format!("sudo dnf install -y {}", bin)));
        }
        if which::which("pacman").is_ok() {
            return ("linux", Some(format!("sudo pacman -S --noconfirm {}", bin)));
        }
        if which::which("zypper").is_ok() {
            return ("linux", Some(format!("sudo zypper install -y {}", bin)));
        }
        return ("linux", None);
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        return ("other", None);
    }
}

fn bin_for_brew(bin: &str) -> &str {
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
    if prompt_auto_install(bin, no_auto) {
        // повторная проверка
        if is_available(bin) {
            return Ok(());
        }
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
            println!("    {}", install_hint(bin).lines().next().unwrap_or("").dimmed());
        }
    }

    println!();
    println!("  {} {}", "Нативные (без зависимостей):".bold(), "картинки (image crate), архивы (zip/tar)".dimmed());
    println!();

    // Информация о системе
    println!("  {} {}", "OS:".dimmed(), std::env::consts::OS);
    println!("  {} {}", "Arch:".dimmed(), std::env::consts::ARCH);
}
