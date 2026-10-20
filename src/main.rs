mod cli;
mod completions;
mod convert;
mod deps;
mod error;
mod geom;
mod interrupt;
mod ui;
mod util;

use clap::{CommandFactory, Parser};
use convert::ConvertOpts;
use geom::Geometry;
use rayon::prelude::*;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

fn main() {
    let cli = cli::Cli::parse();

    interrupt::install();
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
        ui::error("specify an input file: tummler input.png jpg");
        ui::info("tummler --help for usage");
        return ui::EXIT_USAGE;
    }

    let stdin_count = cli.inputs.iter().filter(|p| util::is_stdio(p)).count();
    if stdin_count > 1 {
        ui::error("stdin ('-') may be given only once");
        return ui::EXIT_USAGE;
    }
    if stdin_count == 1 && cli.inputs.len() > 1 {
        ui::error("cannot mix stdin ('-') with other files");
        return ui::EXIT_USAGE;
    }

    let format = match resolve_format(&cli) {
        Ok(f) => f,
        Err(code) => return code,
    };

    let target_ext = util::output_extension(&format);
    if !util::is_supported_output(&target_ext) {
        ui::error(format!("unsupported format: {target_ext}"));
        ui::info("tummler --list-formats for available formats");
        return ui::EXIT_USAGE;
    }

    let geometry = match build_geometry(&cli) {
        Ok(g) => g,
        Err(msg) => {
            ui::error(msg);
            return ui::EXIT_USAGE;
        }
    };
    if !geometry.is_noop() {
        let kind = util::classify_ext(&target_ext);
        if !matches!(kind, util::FormatKind::Image | util::FormatKind::Media) {
            ui::warn("--max/--resize ignored for this format");
        }
    }

    let inputs = util::collect_inputs(&cli.inputs, cli.recursive);
    let inputs: Vec<PathBuf> = inputs
        .into_iter()
        .filter(|p| util::passes_filters(p, &cli.include, &cli.exclude))
        .collect();
    if inputs.is_empty() {
        if !cli.include.is_empty() || !cli.exclude.is_empty() {
            ui::error("no files to convert (filtered by --include/--exclude)");
        } else {
            ui::error("no files to convert");
        }
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
            ui::error(format!("file not found: {}", p.display()));
            failed += 1;
            continue;
        }
        if !p.is_file() {
            ui::warn(format!("skipping (not a file): {}", p.display()));
            failed += 1;
            continue;
        }
        valid_inputs.push(p.clone());
    }

    if valid_inputs.is_empty() {
        ui::error("no valid files");
        return ui::EXIT_FAIL;
    }

    let multiple = valid_inputs.len() > 1;
    let stdout_mode = cli.output.as_deref().is_some_and(util::is_stdio)
        || (valid_inputs.len() == 1 && util::is_stdio(&valid_inputs[0]) && cli.output.is_none());

    if stdout_mode && multiple {
        ui::error("stdout ('-') cannot be used with multiple inputs");
        return ui::EXIT_USAGE;
    }

    if multiple {
        if let Some(ref out) = cli.output {
            if !util::is_stdio(out) && out.exists() && out.is_file() {
                ui::error(format!(
                    "for multiple files -o must be a directory: {}",
                    out.display()
                ));
                return ui::EXIT_USAGE;
            }
        }
    }

    if cli.dry_run {
        ui::info("dry-run, files will not be changed");
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
            let skip = cli.skip_newer && util::output_up_to_date(input, &output);
            if skip {
                println!("{} -> {}  skip", input.display(), output.display());
            } else {
                println!("{} -> {}", input.display(), output.display());
            }
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
        return run_stdio(cli, &valid_inputs, &target_ext, stdout_mode, failed, &geometry);
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
        if input == &output && !cli.overwrite && !cli.skip_newer {
            ui::error(format!(
                "source and destination are the same: {} (use --overwrite to replace in place)",
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
    let skip_newer = cli.skip_newer;
    let no_auto = cli.no_auto_install;
    let yes = cli.yes;
    let verbose = cli.verbose;
    let target_ext_owned = target_ext.clone();

    let success = AtomicUsize::new(0);
    let fail_extra = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    let bytes_in = AtomicU64::new(0);
    let bytes_out = AtomicU64::new(0);

    let run_one = |t: &Task| {
        if verbose {
            ui::info(format!("{} → {}", t.label_in, t.label_out));
        }
        match execute_task(
            t,
            &target_ext_owned,
            quality,
            overwrite,
            skip_newer,
            ffmpeg_args.as_deref(),
            no_auto,
            yes,
            verbose,
            &geometry,
        ) {
            TaskResult::Ok { bin, bout } => {
                success.fetch_add(1, Ordering::Relaxed);
                bytes_in.fetch_add(bin, Ordering::Relaxed);
                bytes_out.fetch_add(bout, Ordering::Relaxed);
                if verbose {
                    ui::info(format!("done {}", t.label_out));
                }
            }
            TaskResult::Skip => {
                skipped.fetch_add(1, Ordering::Relaxed);
                if verbose {
                    ui::info(format!("skip {} (up to date)", t.label_out));
                }
            }
            TaskResult::Err(e) => {
                ui::error(format!("{}: {e}", t.label_in));
                fail_extra.fetch_add(1, Ordering::Relaxed);
            }
        }
    };

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
            if let Some(ref bar) = bar {
                bar.set_message(format!("{} → {}", t.label_in, t.label_out));
            }
            run_one(t);
            if let Some(ref bar) = bar {
                bar.inc(1);
            }
        }

        if let Some(ref bar) = bar {
            let s = success.load(Ordering::Relaxed);
            ui::finish_ok(Some(bar), format!("done {s}/{}", tasks.len()));
        } else if let Some(ref sp) = spin {
            if fail_extra.load(Ordering::Relaxed) == 0 {
                ui::finish_ok(
                    Some(sp),
                    format!("{} → {}", tasks[0].label_in, tasks[0].label_out),
                );
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
                if let Some(ref bar) = bar {
                    bar.set_message(format!("{} → {}", t.label_in, t.label_out));
                }
                run_one(t);
                if let Some(ref bar) = bar {
                    bar.inc(1);
                }
            });
        });
        if let Some(ref bar) = bar {
            let s = success.load(Ordering::Relaxed);
            ui::finish_ok(Some(bar), format!("done {s}/{}", tasks.len()));
        }
    }

    failed += fail_extra.load(Ordering::Relaxed);
    let success = success.load(Ordering::Relaxed);
    let skipped = skipped.load(Ordering::Relaxed);
    let bytes_in = bytes_in.load(Ordering::Relaxed);
    let bytes_out = bytes_out.load(Ordering::Relaxed);

    if !cli.quiet {
        ui::info(util::format_summary(
            success, skipped, failed, bytes_in, bytes_out,
        ));
    }

    if failed > 0 {
        ui::EXIT_FAIL
    } else {
        ui::EXIT_OK
    }
}

enum TaskResult {
    Ok { bin: u64, bout: u64 },
    Skip,
    Err(String),
}

fn execute_task(
    t: &Task,
    target_ext: &str,
    quality: Option<u8>,
    overwrite: bool,
    skip_newer: bool,
    ffmpeg_args: Option<&str>,
    no_auto: bool,
    yes: bool,
    verbose: bool,
    geometry: &Geometry,
) -> TaskResult {
    if skip_newer && util::output_up_to_date(&t.input, &t.output) {
        return TaskResult::Skip;
    }
    let bin = util::file_size(&t.input);
    let overwrite = overwrite || (skip_newer && t.output.exists());
    let opts = ConvertOpts {
        quality,
        overwrite,
        ffmpeg_args,
        no_auto_install: no_auto,
        assume_yes: yes,
        verbose,
        geometry,
    };
    match convert::convert_file(&t.input, &t.output, target_ext, &opts) {
        Ok(()) => TaskResult::Ok {
            bin,
            bout: util::file_size(&t.output),
        },
        Err(e) => TaskResult::Err(e.to_string()),
    }
}

fn build_geometry(cli: &cli::Cli) -> Result<Geometry, String> {
    let resize = match cli.resize.as_deref() {
        Some(s) => Some(geom::parse_resize(s)?),
        None => None,
    };
    Ok(Geometry {
        max: cli.max,
        resize,
    })
}

fn run_stdio(
    cli: cli::Cli,
    valid_inputs: &[PathBuf],
    target_ext: &str,
    stdout_mode: bool,
    mut failed: usize,
    geometry: &Geometry,
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
                ui::error(format!("failed to read stdin: {e}"));
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
                ui::error(format!("failed to create temporary file: {e}"));
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

    if cli.skip_newer && !stdout_mode && util::output_up_to_date(&input_path, &output) {
        ui::info(format!("skip {} (up to date)", output.display()));
        return ui::EXIT_OK;
    }

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

    let bin = util::file_size(&input_path);
    let opts = ConvertOpts {
        quality: cli.quality,
        overwrite: cli.overwrite || stdout_mode || (cli.skip_newer && output.exists()),
        ffmpeg_args: cli.ffmpeg_args.as_deref(),
        no_auto_install: cli.no_auto_install,
        assume_yes: cli.yes,
        verbose: cli.verbose,
        geometry,
    };

    match convert::convert_file(&input_path, &output, target_ext, &opts) {
        Ok(()) => {
            if stdout_mode {
                if let Err(e) = emit_stdout(&output) {
                    ui::finish_clear(spin.as_ref());
                    ui::error(format!("failed to write stdout: {e}"));
                    return ui::EXIT_FAIL;
                }
            }
            success += 1;
            ui::finish_ok(spin.as_ref(), format!("{label_in} → {label_out}"));
            if !cli.quiet {
                let bout = util::file_size(&output);
                ui::info(util::format_summary(success, 0, failed, bin, bout));
            }
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
    ui::error("specify a format: tummler input.png jpg");
    ui::info("or: tummler input.png -f jpeg");
    ui::info("or: tummler input.png -o output.webp");
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
