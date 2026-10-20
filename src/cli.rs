use clap::Parser;
use std::path::PathBuf;

use crate::completions::ShellKind;
use crate::ui::ColorWhen;
use crate::util;

const AFTER_HELP: &str = "\
Примеры:
  tummler photo.png -jpeg
  tummler photo.png -o photo.webp
  tummler photo.png --format webp -q 80
  cat in.png | tummler - -jpeg > out.jpeg
  tummler -- -file.png -jpeg

Коды выхода:
  0  успех
  1  ошибка конвертации
  2  неверные аргументы

Сахар `-jpeg` — это `--format jpeg`, только для известных форматов.
Короткие флаги склеиваются по POSIX: -rvn.
`--` отделяет файлы, чьи имена начинаются с `-`.
";

/// tummler — универсальный конвертер файлов
///
/// Примеры:
///   tummler photo.png -jpeg
///   tummler photo.png --format webp -q 80
///   tummler photo.png -o photo.webp
///   cat in.png | tummler - -jpeg > out.jpeg
#[derive(Parser, Debug)]
#[command(
    name = "tummler",
    version,
    about,
    after_help = AFTER_HELP,
    arg_required_else_help = true,
    next_line_help = false
)]
pub struct Cli {
    /// Входные файлы или директории. «-» = stdin
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<PathBuf>,

    /// Целевой формат (jpeg, webp, mp4, pdf, …). Сахар: `-jpeg`
    #[arg(short = 'f', long = "format", value_name = "EXT")]
    pub format: Option<String>,

    /// Выходной файл или директория. «-» = stdout
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Качество lossy-форматов 1–100
    #[arg(short = 'q', long = "quality", value_name = "1-100", value_parser = clap::value_parser!(u8).range(1..=100))]
    pub quality: Option<u8>,

    /// Перезаписать существующие файлы
    #[arg(long = "overwrite", visible_alias = "force")]
    pub overwrite: bool,

    /// Рекурсивный обход директорий
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Подробный лог на stderr
    #[arg(short = 'v', long = "verbose", conflicts_with = "quiet")]
    pub verbose: bool,

    /// Только ошибки (для скриптов)
    #[arg(long = "quiet", conflicts_with = "verbose")]
    pub quiet: bool,

    /// Показать план, ничего не писать
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Когда красить вывод: auto, always, never
    #[arg(long = "color", value_name = "WHEN", value_enum, num_args = 0..=1, default_value_t = ColorWhen::Auto, default_missing_value = "always")]
    pub color: ColorWhen,

    /// То же, что --color never
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Список поддерживаемых форматов
    #[arg(long = "list-formats")]
    pub list_formats: bool,

    /// Проверить внешние зависимости
    #[arg(long = "check-deps")]
    pub check_deps: bool,

    /// Не предлагать автоустановку зависимостей
    #[arg(long = "no-auto-install")]
    pub no_auto_install: bool,

    /// Ставить зависимости без вопроса (для скриптов)
    #[arg(short = 'y', long = "yes", conflicts_with = "no_auto_install")]
    pub yes: bool,

    /// Дополнительные аргументы ffmpeg
    #[arg(long = "ffmpeg-args", value_name = "ARGS")]
    pub ffmpeg_args: Option<String>,

    /// Скрипт автодополнения оболочки
    #[arg(long = "completions", value_name = "SHELL", value_enum)]
    pub completions: Option<ShellKind>,
}

/// GNU/POSIX предобработка argv + сахар `-jpeg` только для известных форматов.
pub fn preprocess_args(raw: Vec<String>) -> Vec<String> {
    if raw.is_empty() {
        return raw;
    }

    let flags_with_value = [
        "-o",
        "--output",
        "-f",
        "--format",
        "-q",
        "--quality",
        "--ffmpeg-args",
        "--completions",
    ];
    let known_flags = [
        "-o",
        "--output",
        "-f",
        "--format",
        "-q",
        "--quality",
        "-r",
        "--recursive",
        "-v",
        "--verbose",
        "-n",
        "--dry-run",
        "-y",
        "--yes",
        "--overwrite",
        "--force",
        "--quiet",
        "--color",
        "--no-color",
        "--list-formats",
        "--check-deps",
        "--no-auto-install",
        "-h",
        "--help",
        "-V",
        "--version",
        "--ffmpeg-args",
        "--completions",
    ];

    let mut out = vec![raw[0].clone()];
    let mut i = 1;
    while i < raw.len() {
        let arg = &raw[i];

        // POSIX: всё после `--` — операнды, даже если начинается с `-`
        if arg == "--" {
            out.extend_from_slice(&raw[i..]);
            break;
        }

        if arg.starts_with("--format=")
            || arg.starts_with("-f=")
            || arg.starts_with("--completions=")
            || arg.starts_with("--color=")
            || arg.starts_with("--quality=")
            || arg.starts_with("--output=")
        {
            out.push(arg.clone());
            i += 1;
            continue;
        }

        // --color [auto|always|never] — значение опционально
        if arg == "--color" {
            out.push(arg.clone());
            if i + 1 < raw.len() && matches!(raw[i + 1].as_str(), "auto" | "always" | "never") {
                out.push(raw[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if flags_with_value.contains(&arg.as_str()) {
            out.push(arg.clone());
            if i + 1 < raw.len() {
                out.push(raw[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if known_flags.contains(&arg.as_str()) {
            out.push(arg.clone());
            i += 1;
            continue;
        }

        // Сахар `-jpeg` / `-tar.gz` только если это известный формат.
        // Иначе оставляем clap'у: `-rv` = `-r -v`, неизвестный флаг — ошибка.
        if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 1 {
            let stripped = arg[1..].trim_start_matches('.').to_lowercase();
            if stripped
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
                && stripped.len() >= 2
                && util::is_supported_output(&stripped)
            {
                out.push("--format".to_string());
                out.push(stripped);
                i += 1;
                continue;
            }
        }

        out.push(arg.clone());
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dash_format() {
        let raw = vec!["tummler".into(), "img.png".into(), "-jpeg".into()];
        assert_eq!(
            preprocess_args(raw),
            vec!["tummler", "img.png", "--format", "jpeg"]
        );
    }

    #[test]
    fn test_dash_tar_gz() {
        let raw = vec!["tummler".into(), "archive.zip".into(), "-tar.gz".into()];
        assert_eq!(
            preprocess_args(raw),
            vec!["tummler", "archive.zip", "--format", "tar.gz"]
        );
    }

    #[test]
    fn test_clustered_short_flags_not_eaten() {
        let raw = vec!["tummler".into(), "a.png".into(), "-rvn".into(), "-jpeg".into()];
        assert_eq!(
            preprocess_args(raw),
            vec!["tummler", "a.png", "-rvn", "--format", "jpeg"]
        );
    }

    #[test]
    fn test_unknown_dash_not_format() {
        let raw = vec!["tummler".into(), "a.png".into(), "-xyzzy".into()];
        assert_eq!(preprocess_args(raw), vec!["tummler", "a.png", "-xyzzy"]);
    }

    #[test]
    fn test_end_of_options() {
        let raw = vec![
            "tummler".into(),
            "--".into(),
            "-weird.png".into(),
            "-jpeg".into(),
        ];
        assert_eq!(
            preprocess_args(raw),
            vec!["tummler", "--", "-weird.png", "-jpeg"]
        );
    }

    #[test]
    fn test_explicit_format_preserved() {
        let raw = vec![
            "tummler".into(),
            "a.png".into(),
            "--format".into(),
            "webp".into(),
        ];
        assert_eq!(preprocess_args(raw), vec!["tummler", "a.png", "--format", "webp"]);
    }

    #[test]
    fn test_mixed() {
        let raw = vec![
            "tummler".into(),
            "a.png".into(),
            "-jpeg".into(),
            "-o".into(),
            "out.jpg".into(),
        ];
        assert_eq!(
            preprocess_args(raw),
            vec!["tummler", "a.png", "--format", "jpeg", "-o", "out.jpg"]
        );
    }

    #[test]
    fn test_quality_and_format() {
        let raw = vec![
            "tummler".into(),
            "a.png".into(),
            "-q".into(),
            "80".into(),
            "-webp".into(),
        ];
        assert_eq!(
            preprocess_args(raw),
            vec!["tummler", "a.png", "-q", "80", "--format", "webp"]
        );
    }

    #[test]
    fn test_short_flags_not_formats() {
        let raw = vec!["tummler".into(), "a.png".into(), "-r".into(), "-v".into()];
        assert_eq!(preprocess_args(raw), vec!["tummler", "a.png", "-r", "-v"]);
    }

    #[test]
    fn test_empty() {
        assert!(preprocess_args(vec![]).is_empty());
    }
}
