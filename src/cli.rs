use clap::Parser;
use std::path::PathBuf;

use crate::completions::ShellKind;

/// tummler — универсальный конвертер файлов
///
/// Примеры:
///   tummler photo.png -jpeg                 # photo.png → photo.jpeg
///   tummler photo.png --format webp -q 80  # с качеством
///   tummler video.mp4 -gif                  # через ffmpeg
///   tummler doc.docx -pdf                   # через pandoc
///   tummler archive.zip -tar.gz             # перепаковка архива
///   tummler ./photos -webp -r               # батч + рекурсия
///   tummler a.png b.jpg -avif -o ./out      # несколько файлов
#[derive(Parser, Debug)]
#[command(name = "tummler", version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// Входные файлы / директории (один или несколько)
    #[arg(value_name = "INPUT")]
    pub inputs: Vec<PathBuf>,

    /// Целевой формат (расширение без точки): jpeg, webp, mp4, pdf и т.д.
    /// Можно передать как `-jpeg` / `-webp` или `--format jpeg`
    #[arg(short = 'f', long = "format", value_name = "EXT")]
    pub format: Option<String>,

    /// Выходной файл или директория
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Качество для lossy-форматов 1-100 (jpeg, webp, видео)
    #[arg(short = 'q', long = "quality", value_name = "1-100", value_parser = clap::value_parser!(u8).range(1..=100))]
    pub quality: Option<u8>,

    /// Перезаписывать существующие файлы без спроса
    #[arg(long = "overwrite")]
    pub overwrite: bool,

    /// Рекурсивный обход директорий
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Подробный вывод
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Показать что будет сделано, но не конвертировать
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Список поддерживаемых форматов
    #[arg(long = "list-formats")]
    pub list_formats: bool,

    /// Проверить наличие внешних зависимостей (ffmpeg, pandoc)
    #[arg(long = "check-deps")]
    pub check_deps: bool,

    /// Не предлагать автоустановку зависимостей, только подсказка
    #[arg(long = "no-auto-install")]
    pub no_auto_install: bool,

    /// Дополнительные аргументы для ffmpeg (например: --ffmpeg-args "-crf 23 -preset fast")
    #[arg(long = "ffmpeg-args", value_name = "ARGS")]
    pub ffmpeg_args: Option<String>,

    /// Сгенерировать скрипт автодополнения для оболочки
    #[arg(long = "completions", value_name = "SHELL", value_enum)]
    pub completions: Option<ShellKind>,
}

/// Результат предобработки argv: извлекаем `-ext` как формат
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
        "--overwrite",
        "--dry-run",
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

        if arg.starts_with("--format=")
            || arg.starts_with("-f=")
            || arg.starts_with("--completions=")
        {
            out.push(arg.clone());
            i += 1;
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

        // паттерн `-jpeg` / `-tar.gz` / `-pdf` → это формат
        if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 1 {
            let stripped = &arg[1..];
            let looks_like_ext = stripped
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
                && stripped.chars().any(|c| c.is_ascii_alphabetic())
                && stripped.len() >= 2
                && !known_flags.contains(&arg.as_str());

            if looks_like_ext {
                out.push("--format".to_string());
                let ext = stripped.trim_start_matches('.').to_lowercase();
                out.push(ext);
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
        let processed = preprocess_args(raw);
        assert_eq!(processed, vec!["tummler", "img.png", "--format", "jpeg"]);
    }

    #[test]
    fn test_dash_tar_gz() {
        let raw = vec!["tummler".into(), "archive.zip".into(), "-tar.gz".into()];
        let processed = preprocess_args(raw);
        assert_eq!(
            processed,
            vec!["tummler", "archive.zip", "--format", "tar.gz"]
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
        let processed = preprocess_args(raw);
        assert_eq!(processed, vec!["tummler", "a.png", "--format", "webp"]);
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
        let processed = preprocess_args(raw);
        assert_eq!(
            processed,
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
        let processed = preprocess_args(raw);
        assert_eq!(
            processed,
            vec!["tummler", "a.png", "-q", "80", "--format", "webp"]
        );
    }

    #[test]
    fn test_short_flags_not_formats() {
        let raw = vec!["tummler".into(), "a.png".into(), "-r".into(), "-v".into()];
        let processed = preprocess_args(raw);
        assert_eq!(processed, vec!["tummler", "a.png", "-r", "-v"]);
    }

    #[test]
    fn test_empty() {
        let processed = preprocess_args(vec![]);
        assert!(processed.is_empty());
    }
}
