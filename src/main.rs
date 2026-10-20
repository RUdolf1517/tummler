mod cli;
mod completions;
mod convert;
mod deps;
mod error;
mod ui;
mod util;

use clap::{CommandFactory, Parser};
use rayon::prelude::*;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let cli = cli::Cli::parse();

    ui::init_color(cli.color, cli.no_color);
    ui::set_quiet(cli.quiet);

    let code = run(cli);
    if code != ui::EXIT_OK {
        std::process::exit(code);
    }
}

struct Task {
    input: PathBuf,
    output: PathBuf,
    label_in: String,
    label_out: String,
}

fn run(mut cli: cli::Cli) -> i32 {
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

    if cli.format.is_none() {
        if let Some(fmt) = util::take_positional_format(&mut cli.inputs) {
            cli.format = Some(fmt);
        }
    }

    if cli.inputs.is_empty() {
        ui::error("укажите входной файл: tummler input.png jpg");
        ui::info("tummler --help — справка");
        return ui::EXIT_USAGE;
    }

    let stdin_count = cli.inputs.iter().filter(|p| util::is_stdio(p)).count();
    if stdin_count > 1 {
        ui::error("stdin («-») можно указать только один раз");
        return ui::EXIT_USAGE;
    }
    if stdin_count == 1 && cli.inputs.len() > 1 {
        ui::error("stdin («-») нельзя смешивать с другими файлами");
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

    let mut failed = 0usize;
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

    if let Err(e) = convert::prefetch_deps(&valid_inputs, &target_ext, cli.no_auto_install, cli.yes)
    {
        ui::error(e);
        return ui::EXIT_FAIL;
    }

    let has_stdio = stdout_mode || valid_inputs.iter().any(|p| util::is_stdio(p));
    if has_stdio {
        return run_stdio(cli, &valid_inputs, &target_ext, stdout_mode, failed);
    }

    let mut tasks: Vec<Task> = Vec::with_capacity(valid_inputs.len());
    for input in &valid_inputs {
        let output =
            match util::resolve_output_path(input, &target_ext, cli.output.as_deref(), multiple) {
                Ok(p) => p,
                Err(e) => {
                    ui::error(format!("{}: {e}", input.display()));
                    failed += 1;
                    continue;
                }
            };
        if input == &output && !cli.overwrite {
            ui::error(format!(
                "исходник и выход совпадают: {} (нужен --overwrite для записи на месте)",
                input.display()
            ));
            failed += 1;
            continue;
        }
        tasks.push(Task {
            label_in: input.display().to_string(),
            label_out: output.display().to_string(),
            input: input.clone(),
            output,
        });
    }

    if tasks.is_empty() {
        return ui::EXIT_FAIL;
    }

    let jobs = resolve_jobs(cli.jobs, tasks.len());
    let ffmpeg_args = cli.ffmpeg_args.clone();
    let quality = cli.quality;
    let overwrite = cli.overwrite;
    let no_auto = cli.no_auto_install;
    let yes = cli.yes;
    let verbose = cli.verbose;
    let target_ext_owned = target_ext.clone();

    let success = AtomicUsize::new(0);
    let fail_extra = AtomicUsize::new(0);

    if jobs == 1 {
        let show_bar = multiple && !verbose;
        let bar = if show_bar {
            ui::batch_bar(tasks.len() as u64)
        } else {
            None
        };
        let spin = if !multiple && !verbose {
            tasks
                .first()
                .and_then(|t| ui::spinner(format!("{} → {}", t.label_in, t.label_out)))
        } else {
            None
        };

        for t in &tasks {
            if verbose {
                ui::info(format!("{} → {}", t.label_in, t.label_out));
            } else if let Some(ref bar) = bar {
                bar.set_message(format!("{} → {}", t.label_in, t.label_out));
            }
            match convert::convert_file(
                &t.input,
                &t.output,
                &target_ext_owned,
                quality,
                overwrite,
                ffmpeg_args.as_deref(),
                no_auto,
                yes,
                verbose,
            ) {
                Ok(()) => {
                    success.fetch_add(1, Ordering::Relaxed);
                    if verbose {
                        ui::info(format!("готово {}", t.label_out));
                    }
                }
                Err(e) => {
                    ui::error(format!("{}: {e}", t.label_in));
                    fail_extra.fetch_add(1, Ordering::Relaxed);
                }
            }
            if let Some(ref bar) = bar {
                bar.inc(1);
            }
        }

        if let Some(ref bar) = bar {
            let s = success.load(Ordering::Relaxed);
            ui::finish_ok(Some(bar), format!("готово {s}/{}", tasks.len()));
        } else if let Some(ref sp) = spin {
            if fail_extra.load(Ordering::Relaxed) == 0 {
                ui::finish_ok(Some(sp), format!("{} → {}", tasks[0].label_in, tasks[0].label_out));
            } else {
                ui::finish_clear(Some(sp));
            }
        }
    } else {
        let bar = if !verbose {
            ui::batch_bar(tasks.len() as u64)
        } else {
            None
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build()
            .expect("thread pool");
        pool.install(|| {
            tasks.par_iter().for_each(|t| {
                if verbose {
                    ui::info(format!("{} → {}", t.label_in, t.label_out));
                } else if let Some(ref bar) = bar {
                    bar.set_message(format!("{} → {}", t.label_in, t.label_out));
                }
                match convert::convert_file(
                    &t.input,
                    &t.output,
                    &target_ext_owned,
                    quality,
                    overwrite,
                    ffmpeg_args.as_deref(),
                    no_auto,
                    yes,
                    verbose,
                ) {
                    Ok(()) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        ui::error(format!("{}: {e}", t.label_in));
                        fail_extra.fetch_add(1, Ordering::Relaxed);
                    }
                }
                if let Some(ref bar) = bar {
                    bar.inc(1);
                }
            });
        });
        if let Some(ref bar) = bar {
            let s = success.load(Ordering::Relaxed);
            ui::finish_ok(Some(bar), format!("готово {s}/{}", tasks.len()));
        }
    }

    failed += fail_extra.load(Ordering::Relaxed);
    let success = success.load(Ordering::Relaxed);

    if failed > 0 && !cli.quiet {
        ui::info(format!("успешно: {success}, ошибок: {failed}"));
    }

    if failed > 0 {
        ui::EXIT_FAIL
    } else {
        ui::EXIT_OK
    }
}

fn run_stdio(
    cli: cli::Cli,
    valid_inputs: &[PathBuf],
    target_ext: &str,
    stdout_mode: bool,
    mut failed: usize,
) -> i32 {
    let mut success = 0;
    let input = &valid_inputs[0];

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
                return ui::EXIT_FAIL;
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
                return ui::EXIT_FAIL;
            }
        }
    } else {
        match util::resolve_output_path(input, target_ext, cli.output.as_deref(), false) {
            Ok(p) => p,
            Err(e) => {
                ui::error(format!("{}: {e}", input.display()));
                return ui::EXIT_USAGE;
            }
        }
    };

    let label_out = if stdout_mode {
        "-".to_string()
    } else {
        output.display().to_string()
    };
    let label_in = input.display().to_string();
    let spin = if !cli.verbose {
        ui::spinner(format!("{label_in} → {label_out}"))
    } else {
        None
    };

    if cli.verbose {
        ui::info(format!("{label_in} → {label_out}"));
    }

    match convert::convert_file(
        &input_path,
        &output,
        target_ext,
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
                    ui::finish_clear(spin.as_ref());
                    ui::error(format!("не удалось записать stdout: {e}"));
                    return ui::EXIT_FAIL;
                }
            }
            success += 1;
            ui::finish_ok(spin.as_ref(), format!("{label_in} → {label_out}"));
        }
        Err(e) => {
            ui::finish_clear(spin.as_ref());
            ui::error(format!("{label_in}: {e}"));
            failed += 1;
        }
    }

    if failed > 0 {
        ui::EXIT_FAIL
    } else {
        let _ = success;
        ui::EXIT_OK
    }
}

fn resolve_jobs(requested: usize, nfiles: usize) -> usize {
    if nfiles <= 1 {
        return 1;
    }
    let auto = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let n = if requested == 0 { auto } else { requested };
    n.min(nfiles).max(1)
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
    ui::error("укажите формат: tummler input.png jpg");
    ui::info("или: tummler input.png -f jpeg");
    ui::info("или: tummler input.png -o output.webp");
    Err(ui::EXIT_USAGE)
}

fn materialize_stdin() -> io::Result<tempfile::NamedTempFile> {
    let mut stdin = io::stdin().lock();
    let mut header = [0u8; 16];
    let n = stdin.read(&mut header)?;
    let suffix = util::sniff_suffix(&header[..n]);
    let mut tmp = tempfile::Builder::new()
        .prefix("tummler-stdin-")
        .suffix(suffix)
        .tempfile()?;
    tmp.write_all(&header[..n])?;
    io::copy(&mut stdin, tmp.as_file_mut())?;
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
