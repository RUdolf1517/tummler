# tummler 🌀

A universal file converter — one CLI for images, audio/video, documents, and archives.

```bash
tummler photo.png jpg                # photo.png → photo.jpg
tummler photo.png jpeg -q 80         # → photo.jpeg, with quality
tummler video.mp4 gif                # via ffmpeg
tummler doc.docx pdf                 # via pandoc
tummler archive.zip tar.gz           # repack
tummler ./photos webp -r -o ./out    # batch + recursive
tummler a.png b.jpg avif -o ./out    # multiple files
tummler photo.heic jpg --max 1920    # iPhone photo, longest side 1920
```

Supports **macOS** and **Linux** (x86_64 and ARM).

## Install

One command:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

Until a release is published, install from this branch:

```bash
cargo install --git https://github.com/RUdolf1517/tummler --branch arena/ce98465f-tummler
```

Building from source needs Rust 1.74+. The script installs the binary to `~/.local/bin`.

### Prebuilt binaries

Download the latest release from [Releases](https://github.com/RUdolf1517/tummler/releases):

| Platform | File |
|----------|------|
| macOS Apple Silicon | `tummler-aarch64-apple-darwin` |
| macOS Intel | `tummler-x86_64-apple-darwin` |
| Linux x86_64 | `tummler-x86_64-unknown-linux-gnu` |
| Linux ARM64 | `tummler-aarch64-unknown-linux-gnu` |

```bash
chmod +x tummler-*
sudo mv tummler-* /usr/local/bin/tummler
tummler --help
```

### From source

On Linux you need compression headers:

```bash
# Debian/Ubuntu
sudo apt install pkg-config liblzma-dev libbz2-dev cmake
```

```bash
git clone https://github.com/RUdolf1517/tummler
cd tummler
cargo build --release
./target/release/tummler --help
```

## Usage

```
tummler [OPTIONS] <INPUT>... [FORMAT]

Arguments:
  <INPUT>...                Files or directories. "-" = stdin
  [FORMAT]                  Target format last: jpg, webp, mp4, pdf, tar.gz…

Options:
  -f, --format <EXT>        Format explicitly (instead of positional)
  -o, --output <PATH>       Output file or directory. "-" = stdout
  -q, --quality <1-100>     Quality for lossy formats
      --overwrite, --force  Overwrite existing files
  -r, --recursive           Recurse into directories
      --include <GLOB>      Include only matching files (repeatable)
      --exclude <GLOB>      Skip matching files (repeatable)
      --skip-newer          Skip if output exists and is up to date
      --max <PX>            Longest side (images/video, no upscale)
      --resize <SPEC>       50%, 800x, x600, 800x600
  -j, --jobs <N>            Parallel jobs (0 = number of CPUs)
  -v, --verbose             Verbose log on stderr
      --quiet               Errors only (for scripts)
  -n, --dry-run             Print the plan, write nothing
      --color <WHEN>        auto | always | never
      --no-color            Same as --color never
      --list-formats        List formats
      --check-deps          Check ffmpeg/pandoc
      --no-auto-install     Do not install dependencies
  -y, --yes                 Install dependencies without prompting
      --ffmpeg-args <ARGS>  Extra ffmpeg arguments
      --completions <SHELL> bash | zsh | fish | powershell | elvish
  -h, --help                Help
  -V, --version             Version
```

The main syntax puts the format last:

```bash
tummler image.png jpg                    # → image.jpg
tummler image.png jpeg                   # → image.jpeg
tummler image.png webp -o out/
tummler image.png -o image.webp          # format from -o extension
tummler -- -file.png jpeg                # file whose name starts with -
cat in.png | tummler - jpeg > out.jpeg
```

`-f` / `--format` also work: `tummler image.png -f webp`.

### Batch conversion

```bash
# Several files into a directory (in parallel)
tummler a.png b.jpg c.bmp webp -o ./converted/

# Whole directory
tummler ./photos avif --overwrite

# Recursive, 8 threads
tummler ./photos webp -r -o ./out -j 8

# Dry-run — plan on stdout (pipeable)
tummler ./photos jpeg -r -n

# Only PNGs, skip work that's already done
tummler ./photos webp -r --include '*.png' --exclude '*_raw*' --skip-newer -o ./out
```

On a TTY tummler shows a spinner for a single file and a progress bar for a batch. Pipes and `--quiet` disable animation. After a batch it prints a size summary: `12 files, 48 MB → 11 MB (-77%)`.

### Quality

```bash
tummler photo.png jpeg -q 90    # high-quality jpeg (default 90)
tummler photo.png webp -q 80
tummler video.mp4 webm -q 70    # video (crf)
tummler video.mp4 mp3 -q 85     # audio (256k bitrate)
```

### Resize

```bash
tummler photo.png jpg --max 1920     # longest side ≤ 1920, never upscale
tummler photo.png webp --resize 50%
tummler photo.png jpg --resize 800x  # width 800, height proportional
tummler video.mp4 gif --max 480
```

### Overwrite

```bash
tummler img.png jpeg              # error if img.jpeg already exists
tummler img.png jpeg --overwrite  # overwrite
tummler ./photos webp -r --skip-newer   # folders: skip up-to-date outputs
```

## Supported formats

| Category | Formats | Dependencies |
|----------|---------|--------------|
| 🖼 Images | `png, jpg/jpeg, gif, bmp, ico, tiff, webp, avif, heic/heif, pnm, tga, dds, hdr, farbfeld, qoi, exr` | none (pure Rust); HEIC via `ffmpeg` |
| 🎬 Audio/Video | `mp4, avi, mkv, mov, webm, flv, wmv, mpg, 3gp, m4v, ts, mp3, wav, flac, aac, ogg, m4a, opus, wma, aiff` | `ffmpeg` |
| 📄 Documents | `pdf, docx, odt, html, md, tex, rst, epub, pptx, csv, json, txt, rtf` | `pandoc` (+ LaTeX for PDF) |
| 📦 Archives | `zip, tar, tar.gz/tgz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst` | none (pure Rust) |

List them in the terminal:

```bash
tummler --list-formats
```

## Dependencies

Images and archives need no external programs. Everything else needs:

- **ffmpeg** — audio/video (`brew install ffmpeg` / `sudo apt install ffmpeg`)
- **pandoc** — documents (`brew install pandoc` / `sudo apt install pandoc`)
  - PDF output needs LaTeX: `brew install --cask mactex` / `sudo apt install texlive-latex-recommended`

### Auto-install

If a dependency is missing, tummler offers to install it in a TTY. In scripts: `--yes` to install, `--no-auto-install` to never install.

```bash
tummler video.mp4 gif --no-auto-install
tummler --check-deps
```

## CLI conventions

Follows POSIX, GNU, and [clig.dev](https://clig.dev/):

| Rule | tummler |
|------|---------|
| stdout = data, stderr = messages | progress and spinner on stderr, TTY only |
| Exit codes | `0` success, `1` conversion error, `2` bad arguments, `130` Ctrl-C |
| Color | `NO_COLOR`, `FORCE_COLOR`, `TERM=dumb`, `--color`, `--no-color` |
| `--` end of options | `tummler -- -file.png jpeg` |
| stdin/stdout | `tummler - jpeg` reads stdin; without `-o` writes stdout |
| Short flags | cluster: `-rvn` |
| `--quiet` | errors only; `-q` is quality |

GNU-style errors: `tummler: file not found: foo.png`.

## Completions

```bash
# bash
tummler --completions bash | sudo tee /etc/bash_completion.d/tummler

# zsh
mkdir -p ~/.zfunc
tummler --completions zsh > ~/.zfunc/_tummler

# fish
tummler --completions fish > ~/.config/fish/completions/tummler.fish
```

## Examples

```bash
# Images
tummler screenshot.png jpeg -q 85 -o compressed.jpg
tummler logo.png ico
tummler photo.heic jpg --max 1920
tummler ./wallpapers avif -r -o ./avif -j 8 --skip-newer

# Video/audio
tummler input.mp4 webm
tummler input.avi mp4 --ffmpeg-args "-crf 23 -preset fast"
tummler podcast.wav mp3 -q 90
tummler video.mp4 gif

# Documents
tummler report.docx pdf
tummler README.md html -o site.html

# Archives
tummler backup.zip tar.gz
tummler huge.log gz

# Batch
tummler . webp -r --overwrite -q 80
tummler a.png b.jpg c.tiff png -o ./pngs --overwrite
```

## How it works

```
tummler ─┬─ Images      →  image crate (native)
         ├─ Archives    →  zip/tar/flate2/bzip2/xz2/zstd (native)
         ├─ Audio/Video →  ffmpeg (subprocess)
         └─ Documents   →  pandoc (subprocess)
```

Image and archive batches run in parallel (`-j`). ffmpeg/pandoc are checked once before the pool starts.

## Development

```bash
cargo run -- photo.png jpg --dry-run
cargo run -- --list-formats
cargo test
cargo build --release
```

## License

MIT — see [LICENSE](LICENSE).
