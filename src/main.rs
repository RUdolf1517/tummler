mod cli;
mod completions;
mod convert;
mod deps;
mod error;
mod util;

use clap::{CommandFactory, Parser};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::path::PathBuf;

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let processed = cli::preprocess_args(raw_args);

    let cli = cli::Cli::parse_from(processed);

    if let Some(shell) = cli.completions {
        let mut cmd = cli::Cli::command();
        completions::generate_completions(shell, &mut cmd);
        return;
    }

    if cli.list_formats {
        util::print_supported_formats();
        return;
    }

    if cli.check_deps {
        deps::check_all();
        return;
    }

    if cli.inputs.is_empty() {
        eprintln!(
            "{} Укажите входной файл: {}",
            "✗".red(),
            "tummler input.png -jpeg".yellow()
        );
        eprintln!("  {}", "tummler --help  — справка".dimmed());
        std::process::exit(1);
    }

    let format = match cli.format {
        Some(ref f) => f.clone(),
        None => {
            eprintln!(
                "{} Укажите целевой формат: {}",
                "✗".red(),
                "tummler input.png -jpeg".yellow()
            );
            eprintln!(
                "  или: {}",
                "tummler input.png --format jpeg".dimmed()
            );
            eprintln!(
                "  {}",
                "tummler --list-formats  — список всех форматов".dimmed()
            );
            std::process::exit(1);
        }
    };

    let target_ext = util::output_extension(&format);
    if !util::is_supported_output(&target_ext) {
        eprintln!(
            "{} Неподдерживаемый целевой формат: {}",
            "✗".red(),
            target_ext.red()
        );
        eprintln!(
            "  {}",
            "tummler --list-formats — посмотреть доступные".dimmed()
        );
        std::process::exit(1);
    }

    let inputs = util::collect_inputs(&cli.inputs, cli.recursive);

    if inputs.is_empty() {
        eprintln!("{} Нет файлов для конвертации", "✗".red());
        std::process::exit(1);
    }

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

    if multiple {
        if let Some(ref out) = cli.output {
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

    if cli.dry_run {
        println!("{}", "Dry run — файлы не будут конвертированы:".bold());
        for input in &valid_inputs {
            let output =
                util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple)
                    .unwrap_or_else(|e| {
                        eprintln!("{} {}: {}", "✗".red(), input.display(), e);
                        std::process::exit(1);
                    });
            println!(
                "  {} → {}",
                input.display(),
                output.display().to_string().green()
            );
        }
        return;
    }

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
        let output =
            match util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{} {}: {}", "✗".red(), input.display(), e);
                    failed += 1;
                    continue;
                }
            };

        if cli.verbose {
            println!(
                "{} {} → {}",
                "→".cyan(),
                input.display(),
                output.display().to_string().green()
            );
        } else if let Some(ref bar) = pb {
            bar.set_message(format!("{} → {}", input.display(), output.display()));
        }

        if input == &output {
            eprintln!(
                "{} Исходник и выход совпадают, пропуск: {}",
                "⚠".yellow(),
                input.display()
            );
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
                } else if !multiple {
                    println!("{} {}", "✓".green(), output.display().to_string().green());
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
            }
        }
    }

    if let Some(bar) = pb {
        bar.finish_with_message("Готово".green().to_string());
    }

    if multiple || cli.verbose {
        println!();
        if failed == 0 {
            println!(
                "{} Успешно конвертировано: {}",
                "✓".green().to_string().bold(),
                success.to_string().green().bold()
            );
        } else {
            println!(
                "{} Успешно: {}, ошибок: {}",
                if success > 0 { "⚠".yellow() } else { "✗".red() },
                success.to_string().green(),
                failed.to_string().red()
            );
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }
}
