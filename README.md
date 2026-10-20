<div align="center">

# tummler

**טומלער** — *the one who stirs things up*

One CLI that mixes any file into any format.  
Images · audio · video · documents · spreadsheets · archives.

[Install](#install) · [Usage](#usage) · [Formats](#formats) · [Name](#the-name)

[![license](https://img.shields.io/badge/license-MIT-0b3d91?style=flat-square)](LICENSE)
[![platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-111111?style=flat-square)](#install)
[![rust](https://img.shields.io/badge/rust-1.99%2B-dea584?style=flat-square)](https://www.rust-lang.org/)

</div>

---

```bash
tummler photo.png jpg
tummler photo.heic jpg --max 1920
tummler ./photos webp -r --skip-newer -o ./out
tummler video.mp4 gif
tummler report.docx pdf
tummler table.xlsx csv
tummler slides.pptx pdf
tummler backup.zip tar.gz
```

Format last. That’s the whole trick.

```
12 files, 48 MB → 11 MB (-77%)
```

## Install

**No Rust** — binary from GitHub Releases into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

**Homebrew** (no Rust)

```bash
brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
brew install tummler
```

Intel Mac: `brew install --HEAD tummler` (builds with Rust).

**Cargo** (needs [Rust 1.99+](https://rustup.rs))

```bash
# from git:
cargo install --git https://github.com/RUdolf1517/tummler
```

macOS (Apple Silicon) and Linux, x86_64 and ARM. ffmpeg for video/HEIC, pandoc for documents — tummler offers to install them in a TTY. Slides use LibreOffice only if `soffice` is already on PATH (never installed).

<details>
<summary>From source</summary>

```bash
# macOS
brew install pkg-config xz

# Debian/Ubuntu
sudo apt install pkg-config liblzma-dev libbz2-dev

git clone https://github.com/RUdolf1517/tummler
cd tummler
cargo build --release
```

</details>

## Usage

```bash
tummler image.png jpg                      # → image.jpg
tummler image.png jpeg                     # keeps .jpeg
tummler image.png -o image.webp            # format from -o
tummler -- -file.png jpeg                  # name starts with -
cat in.png | tummler - jpeg > out.jpeg     # unix filter
```

### Batch

```bash
tummler a.png b.jpg c.bmp webp -o ./converted/
tummler ./photos avif --overwrite
tummler ./photos webp -r -o ./out -j 8
tummler ./photos jpeg -r -n                                          # dry-run
tummler ./photos webp -r --include '*.png' --exclude '*_raw*' --skip-newer -o ./out
tummler ./sheets csv -o ./out
```

TTY: spinner for one file, progress bar for a batch. `--quiet` and pipes stay silent. Then a size summary.

### Quality & size

```bash
tummler photo.png jpeg -q 90
tummler photo.png jpg --max 1920          # longest side, never upscales
tummler photo.png webp --resize 50%       # lossless unless -q
tummler photo.png webp -q 80              # lossy (ffmpeg)
tummler photo.png avif -q 70
tummler photo.png jpg --resize 800x       # width 800, height follows
tummler video.mp4 webm -q 80              # CRF ~22, not x264 “slow”
tummler video.mp4 mp3 -q 85               # bitrate
```

### Documents

Word / ODT → PDF goes through **pandoc** (auto-installed). A pandoc PDF needs LaTeX. If `soffice` is already on PATH, tummler uses it instead — better Word layout, never installed.

```bash
tummler report.docx pdf
tummler notes.odt pdf
tummler notes.md html
```

### Spreadsheets

Excel / ODS → csv or a padded text table. First sheet by default. No LibreOffice.

```bash
tummler table.xlsx csv
tummler table.xlsx txt --sheet Sales
tummler table.xlsx csv --all-sheets -o ./out
tummler a.xlsx b.xls csv -o ./out/
```

### Slides

Decks (`pptx`/`ppsx`/`odp`/`ppt`) go through **LibreOffice only if `soffice` is already on PATH**. tummler never asks to install it and never installs it. `pptx` ↔ `ppsx` is a copy (same OOXML package) and does not need soffice.

Document → deck goes through pandoc (`odt`/`docx`/`md` → `pptx`). `pdf → pptx` is not supported.

```bash
tummler slides.pptx pdf        # needs soffice already installed
tummler slides.pptx ppsx       # copy, no soffice
tummler notes.odt pptx         # pandoc
tummler deck.odp pptx          # needs soffice already installed
```

### Overwrite

```bash
tummler img.png jpeg                      # error if img.jpeg exists
tummler img.png jpeg --overwrite
tummler ./photos webp -r --skip-newer     # folders: skip what’s already done
```

<details>
<summary>Full flag list</summary>

```
tummler [OPTIONS] <INPUT>... [FORMAT]

  <INPUT>...                files or directories. "-" = stdin
  [FORMAT]                  last: jpg, webp, mp4, pdf, tar.gz…

  -f, --format <EXT>        format explicitly
  -o, --output <PATH>       file or directory. "-" = stdout
  -q, --quality <1-100>     lossy quality
      --overwrite, --force  replace existing files
  -r, --recursive           walk directories
      --include <GLOB>      keep matching files (repeatable)
      --exclude <GLOB>      drop matching files (repeatable)
      --skip-newer          skip if output is up to date
      --max <PX>            longest side (images/video)
      --resize <SPEC>       50% | 800x | x600 | 800x600
  -j, --jobs <N>            parallel jobs (0 = CPUs)
  -v, --verbose             log on stderr
      --quiet               errors only
  -n, --dry-run             print the plan
      --color <WHEN>        auto | always | never
      --no-color
  -i, --info                about
      --list-formats
      --check-deps
      --no-auto-install
  -y, --yes                 install deps without asking
      --sheet <NAME>        spreadsheet sheet (`*` = all, one file each)
      --all-sheets          every sheet → `table-Sheet1.csv`
      --ffmpeg-args <ARGS>
      --completions <SHELL> bash | zsh | fish | powershell | elvish
```

Exit: `0` ok · `1` conversion failed · `2` bad args · `130` Ctrl-C (kills ffmpeg).

stdout is data. stderr is chatter. `NO_COLOR` / `TERM=dumb` respected. GNU-style errors: `tummler: file not found: foo.png`.

</details>

## Formats

| | | |
|---|---|---|
| **Images** | png jpg gif bmp ico tiff webp avif heic pnm tga dds hdr qoi exr | Rust · HEIC via ffmpeg |
| **Audio / video** | mp4 avi mkv mov webm gif mp3 wav flac aac ogg m4a opus … | ffmpeg |
| **Documents** | doc/docx/odt/rtf → pdf via pandoc (soffice if already present); md/html/tex/… | pandoc · LaTeX for pandoc PDF · optional LibreOffice |
| **Spreadsheets** | xlsx xlsm xlsb xls ods → csv txt | Rust (calamine) |
| **Slides** | pptx/ppsx/odp/ppt → pdf and between decks if soffice is present; pptx ↔ ppsx copy; odt/docx/md → pptx | optional LibreOffice · pandoc |
| **Archives** | zip tar tar.gz tar.bz2 tar.xz tar.zst gz bz2 xz zst | Rust |

```bash
tummler --list-formats
tummler --check-deps
```

## Completions

```bash
tummler --completions bash | sudo tee /etc/bash_completion.d/tummler
tummler --completions zsh  > ~/.zfunc/_tummler
tummler --completions fish > ~/.config/fish/completions/tummler.fish
```

## How it works

```
tummler ─┬─ images       →  image crate
         ├─ archives     →  zip / tar / gz / bz2 / xz / zstd
         ├─ audio/video  →  ffmpeg
         ├─ spreadsheets →  calamine (xlsx/xls/ods → csv/txt)
         ├─ slides       →  soffice if already installed; pptx ↔ ppsx is a copy
         └─ documents    →  pandoc; Word/ODT → pdf via soffice if already installed
```

Image and archive batches run in parallel (`-j`). ffmpeg/pandoc are checked once, then the pool starts. Writes go through a staging file, then an atomic rename.

## The name

Yiddish **tummler** (טומלער): *the one who stirs things up.*

In the Catskills resorts the tummler mixed the guests — introduced them, got the room going, kept the night alive. It also echoes **tumbler**, the mixing glass.

This CLI mixes files the same way.

```bash
tummler -i
```

Written by AI. Directed by [RUdolf1517](https://github.com/RUdolf1517).  
[rudolfzinovev@protonmail.com](mailto:rudolfzinovev@protonmail.com)

## License

[MIT](LICENSE)
