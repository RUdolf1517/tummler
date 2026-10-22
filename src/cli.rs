use clap::Parser;
use std::path::PathBuf;

use crate::completions::ShellKind;
use crate::ui::ColorWhen;

const AFTER_HELP: &str = "\
Examples:
  tummler photo.png jpg
  tummler icon.svg png
  tummler photo.png jpeg -q 80
  tummler photo.png -o photo.webp
  cat in.png | tummler - jpeg > out.jpeg
  tummler -- -file.png jpeg
  tummler ./photos webp -r --include '*.png' --skip-newer
  tummler ./photos webp -r -o ./out --preserve-structure
  tummler photo.png jpg --preserve-mtime
  tummler photo.png webp --suffix _opt
  tummler ./photos webp -r --timeout 120
  tummler photo.heic jpg --max 1920
  tummler table.xlsx csv
  tummler table.csv xlsx
  tummler table.xls xlsx
  tummler table.xlsx txt --sheet Sales
  tummler table.xlsx csv --sheet 2
  tummler table.xlsx csv --all-sheets
  tummler video.mkv mp4
  tummler slides.pptx pdf
  tummler report.docx pdf
  tummler notes.odt pdf
  tummler notes.odt pptx
  tummler deck.odp pptx
  tummler report.pdf png
  tummler report.pdf png --page 2
  tummler report.pdf docx
  tummler video.mkv mp4 --strip-meta
  tummler --check-deps
  tummler -i

Exit codes:
  0  success
  1  conversion error
  2  invalid arguments
  130 interrupted (Ctrl-C)

The last argument is the format if it is a known extension.
`--` separates files whose names start with `-`.
";

/// tummler — mix any file into any format
///
/// Examples:
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
    /// Input files or directories, format last: tummler img.png jpg
    #[arg(value_name = "INPUT|FORMAT")]
    pub inputs: Vec<PathBuf>,

    /// Target format (jpeg, webp, mp4, pdf, …)
    #[arg(short = 'f', long = "format", value_name = "EXT")]
    pub format: Option<String>,

    /// Output file or directory. "-" = stdout
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Quality for lossy formats 1–100
    #[arg(short = 'q', long = "quality", value_name = "1-100", value_parser = clap::value_parser!(u8).range(1..=100))]
    pub quality: Option<u8>,

    /// Overwrite existing files
    #[arg(long = "overwrite", visible_alias = "force")]
    pub overwrite: bool,

    /// Recurse into directories
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,

    /// Keep subdirectories under -o (with -r). Default flattens to the output dir
    #[arg(long = "preserve-structure")]
    pub preserve_structure: bool,

    /// Include only files matching GLOB (repeatable). Matches name or path.
    #[arg(long = "include", value_name = "GLOB", action = clap::ArgAction::Append)]
    pub include: Vec<String>,

    /// Exclude files matching GLOB (repeatable)
    #[arg(long = "exclude", value_name = "GLOB", action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Skip if the output exists and is newer than (or as new as) the input
    #[arg(long = "skip-newer")]
    pub skip_newer: bool,

    /// Copy the input's modification time onto the output (like cp -p)
    #[arg(long = "preserve-mtime")]
    pub preserve_mtime: bool,

    /// Prepend to the output stem (`--prefix opt_` → `opt_photo.webp`)
    #[arg(long = "prefix", value_name = "STR")]
    pub prefix: Option<String>,

    /// Append to the output stem (`--suffix _opt` → `photo_opt.webp`)
    #[arg(long = "suffix", value_name = "STR")]
    pub suffix: Option<String>,

    /// Longest side in pixels (images/video; keep aspect, never upscale)
    #[arg(long = "max", value_name = "PX", value_parser = clap::value_parser!(u32).range(1..))]
    pub max: Option<u32>,

    /// Resize: 50%, 800x, x600, 800x600 (images/video; keep aspect)
    #[arg(long = "resize", value_name = "SPEC")]
    pub resize: Option<String>,

    /// PDF page (1-based). Default: all pages stacked in one image
    #[arg(long = "page", value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    pub page: Option<u32>,

    /// Drop EXIF/GPS and author tags (ffmpeg; pandoc title/author/date)
    #[arg(long = "strip-meta")]
    pub strip_meta: bool,

    /// Kill one file's converter after SECS (default: no limit)
    #[arg(long = "timeout", value_name = "SECS", value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: Option<u64>,

    /// Verbose log on stderr
    #[arg(short = 'v', long = "verbose", conflicts_with = "quiet")]
    pub verbose: bool,

    /// Errors only (for scripts)
    #[arg(long = "quiet", conflicts_with = "verbose")]
    pub quiet: bool,

    /// Print the plan, write nothing
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Parallel jobs (0 = number of CPUs)
    #[arg(short = 'j', long = "jobs", value_name = "N", default_value_t = 0)]
    pub jobs: usize,

    /// When to color output: auto, always, never
    #[arg(long = "color", value_name = "WHEN", value_enum, num_args = 0..=1, default_value_t = ColorWhen::Auto, default_missing_value = "always")]
    pub color: ColorWhen,

    /// Same as --color never
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// About tummler
    #[arg(short = 'i', long = "info")]
    pub info: bool,

    /// List supported formats
    #[arg(long = "list-formats")]
    pub list_formats: bool,

    /// Status of ffmpeg/pandoc/poppler; in a TTY, offer to install missing ones
    #[arg(long = "check-deps")]
    pub check_deps: bool,

    /// Do not offer to auto-install dependencies
    #[arg(long = "no-auto-install")]
    pub no_auto_install: bool,

    /// Install dependencies without prompting (for scripts)
    #[arg(short = 'y', long = "yes", conflicts_with = "no_auto_install")]
    pub yes: bool,

    /// Extra ffmpeg arguments
    #[arg(long = "ffmpeg-args", value_name = "ARGS")]
    pub ffmpeg_args: Option<String>,

    /// Spreadsheet sheet: name, 1-based index, or `*` (every sheet → its own file)
    #[arg(long = "sheet", value_name = "NAME|N")]
    pub sheet: Option<String>,

    /// Every spreadsheet sheet → its own file (`table-Sheet1.csv`)
    #[arg(long = "all-sheets", conflicts_with = "sheet")]
    pub all_sheets: bool,

    /// Shell completion script
    #[arg(long = "completions", value_name = "SHELL", value_enum)]
    pub completions: Option<ShellKind>,

    /// Write the man page (groff) to stdout
    #[arg(long = "man")]
    pub man: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn strip_meta_and_check_deps_parse() {
        let c = Cli::try_parse_from(["tummler", "--strip-meta", "a.mkv", "mp4"]).unwrap();
        assert!(c.strip_meta);
        assert!(!c.check_deps);

        let c = Cli::try_parse_from(["tummler", "--check-deps"]).unwrap();
        assert!(c.check_deps);
        assert!(!c.strip_meta);

        let c = Cli::try_parse_from(["tummler", "--check-deps", "-y"]).unwrap();
        assert!(c.check_deps);
        assert!(c.yes);

        let c = Cli::try_parse_from(["tummler", "--preserve-mtime", "a.png", "jpg"]).unwrap();
        assert!(c.preserve_mtime);
        assert!(!c.skip_newer);

        let c = Cli::try_parse_from([
            "tummler",
            "--preserve-structure",
            "-r",
            "-o",
            "./out",
            "./photos",
            "webp",
        ])
        .unwrap();
        assert!(c.preserve_structure);
        assert!(c.recursive);

        let c = Cli::try_parse_from(["tummler", "--suffix", "_opt", "a.png", "webp"]).unwrap();
        assert_eq!(c.suffix.as_deref(), Some("_opt"));
        let c = Cli::try_parse_from(["tummler", "--prefix", "opt_", "a.png", "webp"]).unwrap();
        assert_eq!(c.prefix.as_deref(), Some("opt_"));

        let c = Cli::try_parse_from(["tummler", "--timeout", "120", "a.mkv", "mp4"]).unwrap();
        assert_eq!(c.timeout, Some(120));
        assert!(Cli::try_parse_from(["tummler", "--timeout", "0", "a.mkv", "mp4"]).is_err());
    }
}
