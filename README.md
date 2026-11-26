<div align="center">

# tummler

**טומלער** — *the one who stirs things up*

**One CLI that mixes any file into any format.**

[![ci](https://github.com/RUdolf1517/tummler/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/RUdolf1517/tummler/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/RUdolf1517/tummler?style=flat-square&color=0b3d91)](https://github.com/RUdolf1517/tummler/releases/latest)
[![license](https://img.shields.io/badge/license-MIT-0b3d91?style=flat-square)](LICENSE)

[Install](#install) · [Usage](#usage) · [Formats](#formats) · [Engines](#engines) · [Name](#the-name)

</div>

---

Format last: `tummler photo.png jpg`. Native Rust for images, spreadsheets and archives; ffmpeg, pandoc and poppler when the file needs them. LibreOffice (`soffice`) only if it is already on PATH — never asked for, never installed.

```bash
$ tummler photo.png jpg
$ tummler ./photos webp -r --skip-newer -o ./out
$ tummler report.pdf png --page 2
$ tummler table.xlsx csv --sheet 2
$ tummler backup.zip tar.gz

12 files, 48 MB → 11 MB (-77%)
```

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler && brew install RUdolf1517/tummler/tummler
cargo install tummler --locked                                                     # Rust 1.99+
```

Linux (x86_64 and ARM), macOS (Apple silicon and Intel), Windows x64 (not ARM). Homebrew 6+ [tap trust](https://docs.brew.sh/Tap-Trust): the fully qualified name trusts only this formula. Short name afterwards: `brew trust --formula RUdolf1517/tummler/tummler && brew install tummler`.

<details>
<summary>Debian, Fedora, Windows, from source</summary>

```bash
# Debian / Ubuntu — x86_64
curl -fsSL -o tummler_amd64.deb https://github.com/RUdolf1517/tummler/releases/latest/download/tummler_amd64.deb
sudo apt install ./tummler_amd64.deb

# Debian / Ubuntu — ARM64
curl -fsSL -o tummler_arm64.deb https://github.com/RUdolf1517/tummler/releases/latest/download/tummler_arm64.deb
sudo apt install ./tummler_arm64.deb

# Fedora — x86_64
curl -fsSL -o tummler.x86_64.rpm https://github.com/RUdolf1517/tummler/releases/latest/download/tummler.x86_64.rpm
sudo dnf install ./tummler.x86_64.rpm

# Fedora — ARM64
curl -fsSL -o tummler.aarch64.rpm https://github.com/RUdolf1517/tummler/releases/latest/download/tummler.aarch64.rpm
sudo dnf install ./tummler.aarch64.rpm
```

Windows x64 — until `RUdolf1517.tummler` is in the winget community source, install from this repo (winget downloads the release zip):

```powershell
git clone https://github.com/RUdolf1517/tummler
cd tummler
winget settings --enable LocalManifestFiles
winget install --manifest .\packaging\winget\0.4.0
```

`LocalManifestFiles` is once, as Administrator. After microsoft/winget-pkgs accepts the package: `winget install -e --id RUdolf1517.tummler`. Zip only: [Releases](https://github.com/RUdolf1517/tummler/releases/latest) → `tummler-x86_64-pc-windows-msvc.zip`, put `tummler.exe` on PATH.

From source:

```bash
# macOS:        brew install pkg-config xz
# Debian/Ubuntu: sudo apt install pkg-config liblzma-dev libbz2-dev
git clone https://github.com/RUdolf1517/tummler
cd tummler
cargo build --release
```

Homebrew and the Linux packages install `man tummler` and bash/zsh/fish completions.

</details>

### Dependencies

Images, archives, spreadsheets: nothing extra. ffmpeg, pandoc and poppler are offered in a TTY (`tummler --check-deps`, or the first conversion that needs them). `-y` installs without asking. `--no-auto-install` skips the offer. Linux: apt/dnf before linuxbrew. Windows: winget.

```bash
brew install ffmpeg pandoc poppler                         # macOS
sudo apt install ffmpeg pandoc poppler-utils               # Debian/Ubuntu
sudo dnf install ffmpeg pandoc poppler-utils               # Fedora
winget install -e --id Gyan.FFmpeg                         # Windows x64
winget install -e --id JohnMacFarlane.Pandoc
winget install -e --id oschwartz10612.Poppler
tummler --check-deps
```

| Tool | Used for |
| --- | --- |
| **ffmpeg** | video, HEIC, AVIF, lossy webp |
| **pandoc** | documents (a pandoc PDF needs LaTeX) |
| **poppler** | PDF → images/raster slides/text (`pdfinfo`, `pdftoppm`, `pdftotext`) |
| **soffice** | editable slide conversions and print-layout export for XLSX/ODS/Word/ODT. **Never auto-installed** |
| **OCRmyPDF** | searchable PDF via `--ocr`; install it with Tesseract language data for `--ocr-lang` |

## Usage

```bash
tummler image.png jpg                 # → image.jpg
tummler image.png jpeg                # keeps .jpeg
tummler image.png -o image.webp       # format from -o
tummler -- -file.png jpeg             # name starts with -
cat in.png | tummler - jpeg > out.jpeg
tummler ./photos webp -r -o ./out
tummler ./photos webp -r -o ./out --preserve-structure
tummler photo.png webp --suffix _opt  # → photo_opt.webp
```

`-r` does not follow symlinks. `--include` / `--exclude`: `*` does not cross `/`; `**` does. TTY: spinner for one file, progress bar for a batch. `--quiet` and pipes stay silent.

| Exit | Meaning |
| --- | --- |
| `0` | Success |
| `1` | A conversion failed |
| `2` | Bad arguments |
| `130` | Ctrl-C (kills ffmpeg/pandoc/pdftoppm) |

stdout is data. stderr is chatter. `NO_COLOR` / `TERM=dumb` respected.

<details>
<summary>Images, video, documents, sheets, slides</summary>

```bash
tummler photo.png jpeg -q 90
tummler photo.png jpg --max 1920          # longest side, never upscales
tummler photo.png jpg --resize 800x
tummler photo.png webp -q 80              # lossy (ffmpeg)
tummler icon.svg png
tummler video.mkv mp4                     # remux (-c copy) if codecs fit
tummler video.mkv mp4 --strip-meta
tummler report.docx pdf                   # pandoc; soffice if already on PATH
tummler report.pdf png                    # all pages, one image
tummler report.pdf png --page 2
tummler table.xlsx csv --sheet 2          # 1-based; a sheet named "2" wins
tummler table.csv xlsx                    # no LibreOffice
tummler slides.pptx pdf                   # needs soffice already installed
tummler slides.pdf pptx                   # editable import with LibreOffice Impress
tummler slides.pdf pptx --raster          # one image per slide via Poppler and pandoc
tummler report.pdf jpg --all-pages -o ./pages/
tummler photo.jpg scan.png photo.heic pdf --merge -o scans.pdf
tummler budget.xlsx pdf                   # uses LibreOffice Calc print settings
tummler budget.ods pdf
tummler scan.pdf pdf --ocr --ocr-lang eng+rus
tummler slides.pptx ppsx                  # copy, no soffice
```

JPEG `--max` / `--resize` decodes at 1/2, 1/4 or 1/8 (DCT) when that still covers the target. PDF → png/jpg: poppler, raster at `--max` / `--resize`. PDF → docx: soffice layout if present, else `pdftotext` + pandoc (text only). PDF → PPTX uses LibreOffice Impress and creates one slide per page. Layout can vary with the source PDF and available fonts. Scanned pages stay images.

`--all-pages` names files `report-1.jpg`, `report-2.jpg`, and so on, with zero-padding for longer PDFs. `--merge` puts each image on its own PDF page in the input order. JPG and PNG work in Rust, and HEIC/HEIF needs FFmpeg. `--ocr` adds searchable text to scanned pages and skips pages with an existing text layer. With no output path, it writes `scan-ocr.pdf`. The default OCR language is English.

</details>

<details>
<summary>Flags</summary>

```
tummler [OPTIONS] <INPUT>... [FORMAT]

  -f, --format <EXT>        format explicitly
  -o, --output <PATH>       file or directory. "-" = stdout
  -q, --quality <1-100>     lossy quality
      --overwrite, --force  replace existing output files (does not delete the input)
  -r, --recursive           walk directories (does not follow symlinks)
      --preserve-structure  keep subdirs under -o (needs -r). Default flattens
      --include / --exclude <GLOB>
      --skip-newer          skip if output is up to date
      --preserve-mtime      copy input mtime onto the output (like cp -p)
      --prefix / --suffix   glue onto the output stem
      --max <PX>            longest side (images/video)
      --resize <SPEC>       50% | 800x | x600 | 800x600
      --page <N>            PDF page for image output (1-based; default: all pages in one image)
      --all-pages           separate PNG or JPG file for each PDF page
      --merge               combine JPG/PNG/HEIC images into one PDF
      --ocr                 add a searchable text layer to a scanned PDF
      --ocr-lang <LANGS>    OCR languages, e.g. eng+rus (default: eng)
      --raster              PDF → PPTX with one page image per slide
      --strip-meta          drop EXIF/GPS/author
      --timeout <SECS>      kill one file after SECS
      --sheet <NAME|N>      spreadsheet sheet
      --all-sheets          every sheet → table-Sheet1.csv
  -j, --jobs <N>            parallel jobs (0 = CPUs)
  -n, --dry-run             print the plan
      --check-deps          TTY offers to install ffmpeg/pandoc/poppler
      --completions <SHELL> bash | zsh | fish | powershell | elvish
      --man                 groff to stdout
```

</details>

## Formats

| | Converts | Engine |
|---|---|---|
| **Images** | png jpg gif bmp ico tiff webp · svg→raster · avif/heic · jpg/png/heic → pdf | Rust · resvg · ffmpeg |
| **Audio / video** | mp4 mkv mov webm gif · mp3 wav flac aac ogg m4a opus … | ffmpeg |
| **Documents** | doc/docx/odt/rtf → pdf · md/html/tex/ipynb/… · pdf → png/jpg/docx · scanned PDF → searchable PDF | pandoc · poppler · OCRmyPDF · optional soffice |
| **Spreadsheets** | xlsx/xls/ods → csv txt · csv/xls/ods → xlsx · xlsx/ods → pdf | calamine · rust_xlsxwriter · soffice for PDF |
| **Slides** | pdf → pptx (editable or raster) · pptx/ppsx/odp/ppt → pdf and between decks · pptx ↔ ppsx copy · odt/docx/md → pptx | optional soffice · pandoc |
| **Archives** | zip tar tar.gz tar.bz2 tar.xz tar.zst gz bz2 xz zst | Rust |

```bash
tummler --list-formats
tummler --check-deps
```

## Engines

```
tummler ─┬─ images       →  image crate; svg via resvg; avif/heic via ffmpeg
         ├─ archives     →  zip / tar / gz / bz2 / xz / zstd (zip↔tar streamed)
         ├─ audio/video  →  ffmpeg
         ├─ spreadsheets →  calamine + rust_xlsxwriter (no LibreOffice)
         ├─ slides       →  soffice if already installed; pptx ↔ ppsx is a copy
         └─ documents    →  pandoc; PDF → png via poppler; PDF → docx via soffice or pdftotext
```

Writes go through a staging file, then an atomic rename. Image and archive batches run in parallel (`-j`). ffmpeg/pandoc/poppler are checked once, then the pool starts.

## The name

Yiddish **tummler** (טומלער): *the one who stirs things up.* In the Catskills resorts the tummler mixed the guests. It also echoes **tumbler**, the mixing glass. This CLI mixes files the same way.

```bash
tummler -i
```

Written by AI. Directed by [RUdolf1517](https://github.com/RUdolf1517) ·
[rudolfzinovev@protonmail.com](mailto:rudolfzinovev@protonmail.com)

## License

[MIT](LICENSE)
