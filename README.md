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
tummler report.pdf png                  # all pages, one image
tummler report.pdf png --page 2
tummler table.xlsx csv --sheet 2
tummler slides.pptx pdf                 # soffice if already on PATH
tummler backup.zip tar.gz
tummler video.mkv mp4 --strip-meta
```

Format last. That’s the whole trick.

```
12 files, 48 MB → 11 MB (-77%)
```

## Install

**Binary** (no Rust) into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

**Homebrew** (no Rust). Third-party tap — Homebrew 6+ [tap trust](https://docs.brew.sh/Tap-Trust). A fully qualified name trusts only this formula:

```bash
brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
brew install RUdolf1517/tummler/tummler
```

After that, short name:

```bash
brew trust --formula RUdolf1517/tummler/tummler
brew install tummler
```

**Cargo** ([Rust 1.99+](https://rustup.rs)):

```bash
cargo install --git https://github.com/RUdolf1517/tummler
```

macOS (Apple Silicon and Intel) and Linux, x86_64 and ARM.

ffmpeg (video/HEIC), pandoc (documents), poppler (PDF → png/docx) — tummler offers to install them in a TTY (`tummler --check-deps`, or the first conversion that needs them). LibreOffice (`soffice`) is used only if already on PATH. Never asked for, never installed.

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
tummler ./photos jpeg -r -n                # dry-run
tummler ./photos webp -r --include '*.png' --exclude '*_raw*' --skip-newer -o ./out
```

TTY: spinner for one file, progress bar for a batch. `--quiet` and pipes stay silent. Then a size summary.

### Images & video

```bash
tummler photo.png jpeg -q 90
tummler photo.png jpg --max 1920           # longest side, never upscales
tummler photo.png jpg --resize 800x        # width 800, height follows
tummler photo.png webp --resize 50%        # lossless unless -q
tummler photo.png webp -q 80               # lossy (ffmpeg)
tummler photo.png avif -q 70
tummler icon.svg png
tummler video.mkv mp4                      # remux (`-c copy`) if codecs fit
tummler video.mp4 webm -q 80               # CRF ~22, not x264 “slow”
tummler video.mp4 mp3 -q 85                # bitrate
tummler video.mkv mp4 --strip-meta         # drop container tags (GPS/author)
```

JPEG `--max` / `--resize` decodes at 1/2, 1/4 or 1/8 (DCT) when that still covers the target.

### Documents & PDF

Word / ODT → PDF: **pandoc** (auto-installed; a pandoc PDF needs LaTeX). If `soffice` is already on PATH, that is used instead — better Word layout, never installed.

```bash
tummler report.docx pdf
tummler notes.odt pdf
tummler notes.md html
tummler report.pdf docx
tummler report.pdf png                     # every page, stacked in one PNG
tummler report.pdf png --page 2            # one page (1-based)
tummler report.pdf png --max 1920
```

PDF → png/jpg: poppler `pdftoppm`, raster at `--max` / `--resize`. PDF → docx: LibreOffice if `soffice` is on PATH (`writer_pdf_import`); otherwise `pdftotext` + pandoc (text only). `pdf → pptx` is not supported.

### Spreadsheets

Excel / ODS → csv or a padded text table. csv / xls / ods → xlsx. First sheet by default. No LibreOffice.

```bash
tummler table.xlsx csv
tummler table.csv xlsx
tummler table.xls xlsx
tummler table.xlsx txt --sheet Sales       # by name
tummler table.xlsx csv --sheet 2           # 1-based index
tummler table.xlsx csv --all-sheets -o ./out
tummler a.xlsx b.xls csv -o ./out/
```

A sheet actually named `2` wins over index 2.

### Slides

Decks (`pptx` / `ppsx` / `odp` / `ppt`) go through **LibreOffice only if `soffice` is already on PATH**. tummler never asks to install it. `pptx` ↔ `ppsx` is a copy (same OOXML package).

Document → deck is pandoc (`odt` / `docx` / `md` → `pptx`). `pdf → pptx` is not supported.

```bash
tummler slides.pptx pdf        # needs soffice already installed
tummler slides.pptx ppsx       # copy, no soffice
tummler notes.odt pptx         # pandoc
tummler deck.odp pptx          # needs soffice already installed
```

### Overwrite

```bash
tummler img.png jpeg                       # error if img.jpeg exists
tummler img.png jpeg --overwrite
tummler ./photos webp -r --skip-newer      # folders: skip what’s already done
```

<details>
<summary>Flags</summary>

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
      --page <N>            PDF page (1-based; default: all pages in one image)
      --strip-meta          drop EXIF/GPS/author (ffmpeg; pandoc title/author/date)
      --sheet <NAME|N>      spreadsheet sheet (name, 1-based index, or *)
      --all-sheets          every sheet → table-Sheet1.csv
  -j, --jobs <N>            parallel jobs (0 = CPUs)
  -v, --verbose             log on stderr
      --quiet               errors only
  -n, --dry-run             print the plan
      --color <WHEN>        auto | always | never
      --no-color
  -i, --info                about
      --list-formats
      --check-deps          status; TTY offers to install ffmpeg/pandoc/poppler
      --no-auto-install
  -y, --yes                 install deps without asking
      --ffmpeg-args <ARGS>
      --completions <SHELL> bash | zsh | fish | powershell | elvish
      --man                 groff man page to stdout
```

Exit: `0` ok · `1` conversion failed · `2` bad args · `130` Ctrl-C (kills ffmpeg/pandoc/pdftoppm).

stdout is data. stderr is chatter. `NO_COLOR` / `TERM=dumb` respected. GNU-style errors: `tummler: file not found: foo.png`.

</details>

## Formats

| | Converts | Engine |
|---|---|---|
| **Images** | png jpg gif bmp ico tiff webp · svg→raster · avif/heic | Rust · resvg · ffmpeg |
| **Audio / video** | mp4 mkv mov webm gif · mp3 wav flac aac ogg m4a opus … | ffmpeg |
| **Documents** | doc/docx/odt/rtf → pdf · md/html/tex/ipynb/… · pdf → png/jpg (`--page N`) · pdf → docx | pandoc · poppler · optional soffice |
| **Spreadsheets** | xlsx/xls/ods → csv txt · csv/xls/ods → xlsx | calamine · rust_xlsxwriter |
| **Slides** | pptx/ppsx/odp/ppt → pdf and between decks · pptx ↔ ppsx copy · odt/docx/md → pptx | optional soffice · pandoc |
| **Archives** | zip tar tar.gz tar.bz2 tar.xz tar.zst gz bz2 xz zst | Rust |

```bash
tummler --list-formats
tummler --check-deps
```

A pandoc PDF needs LaTeX. `pdf → pptx` is not supported.

## Completions

Homebrew installs bash/zsh/fish completions and the man page (`man tummler`).

```bash
tummler --completions bash | sudo tee /etc/bash_completion.d/tummler
tummler --completions zsh  > ~/.zfunc/_tummler
tummler --completions fish > ~/.config/fish/completions/tummler.fish
tummler --man | groff -man -Tascii | less
```

## How it works

```
tummler ─┬─ images       →  image crate; svg via resvg; avif/heic via ffmpeg
         ├─ archives     →  zip / tar / gz / bz2 / xz / zstd (zip↔tar streamed)
         ├─ audio/video  →  ffmpeg
         ├─ spreadsheets →  calamine + rust_xlsxwriter (no LibreOffice)
         ├─ slides       →  soffice if already installed; pptx ↔ ppsx is a copy
         └─ documents    →  pandoc; PDF → png via poppler; PDF → docx via soffice or pdftotext
```

Image and archive batches run in parallel (`-j`). ffmpeg/pandoc/poppler are checked once, then the pool starts. Writes go through a staging file, then an atomic rename.

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
