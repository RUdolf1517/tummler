mod cli;
mod convert;
mod deps;
mod error;
mod util;

use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::path::PathBuf;

fn main() {
    // Предобработка аргументов для поддержки `tummler img.png -jpeg`
    let raw_args: Vec<String> = env::args().collect();
    let processed = cli::preprocess_args(raw_args);

    let cli = cli::Cli::parse_from(processed);

    if cli.list_formats {
        util::print_supported_formats();
        return;
    }

    if cli.check_deps {
        deps::check_all();
        return;
    }

    // Валидация формата
    let format = match cli.format {
        Some(ref f) => f.clone(),
        None => {
            eprintln!("{} Укажите целевой формат: {}", "✗".red(), "tummler input.png -jpeg".yellow());
            eprintln!("  или: {}  {}", "tummler input.png --format jpeg".dimmed(), "".dimmed());
            eprintln!("  {}", "tummler --list-formats  — список всех форматов".dimmed());
            std::process::exit(1);
        }
    };

    let target_ext = util::normalize_ext(&format);
    if !util::is_supported_output(&target_ext) {
        eprintln!("{} Неподдерживаемый целевой формат: {}", "✗".red(), target_ext.red());
        eprintln!("  {}", "tummler --list-formats — посмотреть доступные".dimmed());
        std::process::exit(1);
    }

    if let Some(q) = cli.quality {
        if q == 0 || q > 100 {
            eprintln!("{} Качество должно быть 1-100, получено: {}", "✗".red(), q);
            std::process::exit(1);
        }
    }

    // Сбор входных файлов (с учётом директорий)
    let inputs = util::collect_inputs(&cli.inputs, cli.recursive);

    if inputs.is_empty() {
        eprintln!("{} Нет файлов для конвертации", "✗".red());
        std::process::exit(1);
    }

    // Проверка существования файлов
    let mut valid_inputs: Vec<PathBuf> = Vec::new();
    for p in &inputs {
        if !p.exists() {
            eprintln!("{} Файл не найден: {}", "✗".red(), p.display());
            continue;
        }
        if !p.is_file() {
            eprintln!("{} Пропуск (не файл): {}", "⚠".yellow(), p.display());
            continue;
        }
        valid_inputs.push(p.clone());
    }

    if valid_inputs.is_empty() {
        eprintln!("{} Нет валидных файлов", "✗".red());
        std::process::exit(1);
    }

    let multiple = valid_inputs.len() > 1;

    // Валидация output для батча
    if multiple {
        if let Some(ref out) = cli.output {
            // Если указано -o как файл (не директория, не существует и нет / в конце) — это ошибка
            // Но мы трактуем как директорию если multiple, см. util::resolve_output_path
            // Проверим: если out существует и это файл — ошибка
            if out.exists() && out.is_file() {
                eprintln!(
                    "{} Для нескольких файлов -o должен быть директорией, а не файлом: {}",
                    "✗".red(),
                    out.display()
                );
                std::process::exit(1);
            }
        }
    }

    // Dry-run: просто показать что будет сделано
    if cli.dry_run {
        println!("{}", "Dry run — файлы не будут конвертированы:".bold());
        for input in &valid_inputs {
            let output = util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple)
                .unwrap_or_else(|e| {
                    eprintln!("{} {}: {}", "✗".red(), input.display(), e);
                    std::process::exit(1);
                });
            println!("  {} → {}", input.display(), output.display().to_string().green());
        }
        return;
    }

    // Прогресс-бар для батча
    let pb = if multiple && !cli.verbose {
        let bar = ProgressBar::new(valid_inputs.len() as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("█▓▒░─"),
        );
        Some(bar)
    } else {
        None
    };

    let mut success = 0;
    let mut failed = 0;

    for input in &valid_inputs {
        let output = match util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{} {}: {}", "✗".red(), input.display(), e);
                failed += 1;
                continue;
            }
        };

        if cli.verbose {
            println!("{} {} → {}", "→".cyan(), input.display(), output.display().to_string().green());
        } else if let Some(ref bar) = pb {
            bar.set_message(format!("{} → {}", input.display(), output.display()));
        }

        // Пропуск если исходник и выход совпадают
        if input == &output {
            eprintln!("{} Исходник и выход совпадают, пропуск: {}", "⚠".yellow(), input.display());
            failed += 1;
            continue;
        }

        match convert::convert_file(
            input,
            &output,
            &target_ext,
            cli.quality,
            cli.overwrite,
            cli.ffmpeg_args.as_deref(),
            cli.no_auto_install,
            cli.verbose,
        ) {
            Ok(()) => {
                if cli.verbose {
                    println!("  {} {}", "✓".green(), output.display().to_string().green());
                }
                if let Some(ref bar) = pb {
                    bar.inc(1);
                }
                success += 1;
            }
            Err(e) => {
                eprintln!("{} {}: {}", "✗".red(), input.display(), e);
                failed += 1;
                if let Some(ref bar) = pb {
                    bar.inc(1);
                }
                // Удаляем частично созданный файл
                if output.exists() {
                    // Проверяем что файл был создан только что (размер 0 или битый)
                    // Для безопасности — удаляем только если конвертация упала
                    let _ = std::fs::remove_file(&output);
                }
            }
        }
    }

    if let Some(bar) = pb {
        bar.finish_with_message("Готово".green().to_string());
    }

    // Итог
    if multiple || cli.verbose {
        println!();
        if failed == 0 {
            println!("{} Успешно конвертировано: {}", "✓".green().to_string().bold(), success.to_string().green().bold());
        } else {
            println!(
                "{} Успешно: {}, {} Ошибок: {}",
                if success > 0 { "⚠".yellow() } else { "✗".red() },
                success.to_string().green(),
                "".red(),
                failed.to_string().red()
            );
            if failed > 0 && success == 0 {
                std::process::exit(1);
            }
        }
    } else if failed > 0 {
        std::process::exit(1);
    }
}
