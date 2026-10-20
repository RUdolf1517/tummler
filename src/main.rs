mod cli;
mod completions;
mod convert;
mod deps;
mod error;
mod ui;
mod util;

use clap::{CommandFactory, Parser};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let raw_args: Vec<String> = env::args().collect();
    let processed = cli::preprocess_args(raw_args);
    let cli = cli::Cli::parse_from(processed);

    ui::init_color(cli.color, cli.no_color);
    ui::set_quiet(cli.quiet);

    let code = run(cli);
    if code != ui::EXIT_OK {
        std::process::exit(code);
    }
}

fn run(cli: cli::Cli) -> i32 {
    if let Some(shell) = cli.completions {
        let mut cmd = cli::Cli::command();
        completions::generate_completions(shell, &mut cmd);
        return ui::EXIT_OK;
    }

    if cli.list_formats {
        util::print_supported_formats();
        return ui::EXIT_OK;
    }

    if cli.check_deps {
        deps::check_all();
        return ui::EXIT_OK;
    }

    if cli.inputs.is_empty() {
        ui::error("укажите входной файл: tummler input.png -jpeg");
        ui::info("tummler --help — справка");
        return ui::EXIT_USAGE;
    }

    let stdin_count = cli.inputs.iter().filter(|p| util::is_stdio(p)).count();
    if stdin_count > 1 {
        ui::error("stdin («-») можно указать только один раз");
        return ui::EXIT_USAGE;
    }

    let format = match resolve_format(&cli) {
        Ok(f) => f,
        Err(code) => return code,
    };

    let target_ext = util::output_extension(&format);
    if !util::is_supported_output(&target_ext) {
        ui::error(format!("неподдерживаемый формат: {target_ext}"));
        ui::info("tummler --list-formats — доступные форматы");
        return ui::EXIT_USAGE;
    }

    let inputs = util::collect_inputs(&cli.inputs, cli.recursive);
    if inputs.is_empty() {
        ui::error("нет файлов для конвертации");
        return ui::EXIT_FAIL;
    }

    let mut failed = 0;
    let mut valid_inputs: Vec<PathBuf> = Vec::new();
    for p in &inputs {
        if util::is_stdio(p) {
            valid_inputs.push(p.clone());
            continue;
        }
        if !p.exists() {
            ui::error(format!("файл не найден: {}", p.display()));
            failed += 1;
            continue;
        }
        if !p.is_file() {
            ui::warn(format!("пропуск (не файл): {}", p.display()));
            failed += 1;
            continue;
        }
        valid_inputs.push(p.clone());
    }

    if valid_inputs.is_empty() {
        ui::error("нет валидных файлов");
        return ui::EXIT_FAIL;
    }

    let multiple = valid_inputs.len() > 1;
    let stdout_mode = cli.output.as_deref().is_some_and(util::is_stdio)
        || (valid_inputs.len() == 1 && util::is_stdio(&valid_inputs[0]) && cli.output.is_none());

    if stdout_mode && multiple {
        ui::error("stdout («-») нельзя использовать с несколькими входами");
        return ui::EXIT_USAGE;
    }

    if multiple {
        if let Some(ref out) = cli.output {
            if !util::is_stdio(out) && out.exists() && out.is_file() {
                ui::error(format!(
                    "для нескольких файлов -o должен быть директорией: {}",
                    out.display()
                ));
                return ui::EXIT_USAGE;
            }
        }
    }

    if cli.dry_run {
        ui::info("dry-run, файлы не будут изменены");
        for input in &valid_inputs {
            let output = if stdout_mode {
                PathBuf::from("-")
            } else {
                match util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple)
                {
                    Ok(p) => p,
                    Err(e) => {
                        ui::error(format!("{}: {e}", input.display()));
                        return ui::EXIT_USAGE;
                    }
                }
            };
            println!("{} -> {}", input.display(), output.display());
        }
        return ui::EXIT_OK;
    }

    let show_progress = multiple && !cli.verbose && !cli.quiet && ui::stderr_is_tty();
    let pb = if show_progress {
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

    for input in &valid_inputs {
        let mut _stdin_guard = None;
        let input_path: PathBuf = if util::is_stdio(input) {
            match materialize_stdin() {
                Ok(tmp) => {
                    let p = tmp.path().to_path_buf();
                    _stdin_guard = Some(tmp);
                    p
                }
                Err(e) => {
                    ui::error(format!("не удалось прочитать stdin: {e}"));
                    failed += 1;
                    continue;
                }
            }
        } else {
            input.clone()
        };

        let mut _stdout_guard = None;
        let output = if stdout_mode {
            match tempfile::Builder::new()
                .prefix("tummler-out-")
                .suffix(&format!(".{target_ext}"))
                .tempfile()
            {
                Ok(tmp) => {
                    let p = tmp.path().to_path_buf();
                    _stdout_guard = Some(tmp);
                    p
                }
                Err(e) => {
                    ui::error(format!("не удалось создать временный файл: {e}"));
                    failed += 1;
                    continue;
                }
            }
        } else {
            match util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple) {
                Ok(p) => p,
                Err(e) => {
                    ui::error(format!("{}: {e}", input.display()));
                    failed += 1;
                    continue;
                }
            }
        };

        if cli.verbose {
            ui::info(format!(
                "{} → {}",
                input.display(),
                if stdout_mode {
                    "-".to_string()
                } else {
                    output.display().to_string()
                }
            ));
        } else if let Some(ref bar) = pb {
            bar.set_message(format!("{} → {}", input.display(), output.display()));
        }

        if !stdout_mode && input_path == output {
            ui::warn(format!(
                "исходник и выход совпадают, пропуск: {}",
                input.display()
            ));
            failed += 1;
            continue;
        }

        match convert::convert_file(
            &input_path,
            &output,
            &target_ext,
            cli.quality,
            cli.overwrite || stdout_mode,
            cli.ffmpeg_args.as_deref(),
            cli.no_auto_install,
            cli.yes,
            cli.verbose,
        ) {
            Ok(()) => {
                if stdout_mode {
                    if let Err(e) = emit_stdout(&output) {
                        ui::error(format!("не удалось записать stdout: {e}"));
                        failed += 1;
                        continue;
                    }
                } else if cli.verbose {
                    ui::info(format!("готово {}", output.display().to_string().green()));
                }
                if let Some(ref bar) = pb {
                    bar.inc(1);
                }
                success += 1;
            }
            Err(e) => {
                ui::error(format!("{}: {e}", input.display()));
                failed += 1;
                if let Some(ref bar) = pb {
                    bar.inc(1);
                }
            }
        }
    }

    if let Some(bar) = pb {
        bar.finish_and_clear();
    }

    if (multiple || cli.verbose) && !cli.quiet {
        if failed == 0 {
            ui::info(format!("успешно: {success}"));
        } else {
            ui::info(format!("успешно: {success}, ошибок: {failed}"));
        }
    }

    if failed > 0 {
        ui::EXIT_FAIL
    } else {
        ui::EXIT_OK
    }
}

fn resolve_format(cli: &cli::Cli) -> Result<String, i32> {
    if let Some(ref f) = cli.format {
        return Ok(f.clone());
    }
    if let Some(ref out) = cli.output {
        if let Some(ext) = util::requested_ext_from_path(out) {
            if util::is_supported_output(&ext) {
                return Ok(ext);
            }
        }
    }
    ui::error("укажите формат: tummler input.png -jpeg");
    ui::info("или: tummler input.png --format jpeg");
    ui::info("или: tummler input.png -o output.webp");
    Err(ui::EXIT_USAGE)
}

fn materialize_stdin() -> io::Result<tempfile::NamedTempFile> {
    let mut tmp = tempfile::Builder::new()
        .prefix("tummler-stdin-")
        .tempfile()?;
    io::copy(&mut io::stdin().lock(), tmp.as_file_mut())?;
    tmp.flush()?;
    Ok(tmp)
}

fn emit_stdout(path: &Path) -> io::Result<()> {
    let mut f = std::fs::File::open(path)?;
    let mut stdout = io::stdout().lock();
    io::copy(&mut f, &mut stdout)?;
    stdout.flush()?;
    Ok(())
}
