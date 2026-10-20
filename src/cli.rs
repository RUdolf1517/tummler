use clap::Parser;
use std::path::PathBuf;

use crate::completions::ShellKind;
use crate::ui::ColorWhen;

const AFTER_HELP: &str = "\
Примеры:
  tummler photo.png jpg
  tummler photo.png jpeg -q 80
  tummler photo.png -o photo.webp
  cat in.png | tummler - jpeg > out.jpeg
  tummler -- -file.png jpeg

Коды выхода:
  0  успех
  1  ошибка конвертации
  2  неверные аргументы

Последний аргумент — формат, если это известное расширение.
`--` отделяет файлы, чьи имена начинаются с `-`.
";

/// tummler — универсальный конвертер файлов
///
/// Примеры:
///   tummler photo.png jpg
///   tummler photo.png webp -q 80
///   tummler photo.png -o photo.webp
///   cat in.png | tummler - jpeg > out.jpeg
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
    /// Входные файлы или директории, последним — формат: tummler img.png jpg
    #[arg(value_name = "INPUT|FORMAT")]
    pub inputs: Vec<PathBuf>,

    /// Целевой формат (jpeg, webp, mp4, pdf, …)
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

    /// Параллельные задачи (0 = число CPU)
    #[arg(short = 'j', long = "jobs", value_name = "N", default_value_t = 0)]
    pub jobs: usize,

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
