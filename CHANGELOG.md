# Changelog

## Unreleased

## 0.2.0 — 2026-10-21

- `zip` 9.0 / `calamine` 0.37 (after the zip 8.x yank; unifies with rust_xlsxwriter 0.100)
- Allowlist: `m4b` `ogv` `vob` `caf` (ffmpeg), `ipynb` (pandoc)
- SVG → png/jpg/… via resvg (svg is input-only)
- `zip` ↔ `tar`/`tar.gz`/… streamed (no temp unpack); zip-slip / tar-slip entries skipped
- `tar` ↔ `tar.gz`/`tar.bz2`/`tar.xz`/`tar.zst` recompresses the payload (no unpack)
- Zip files packed as `0o644` (dirs `0o755`)
- Stdin `PK…`: sniff OOXML/ODF (`docx`/`xlsx`/`pptx`/`odt`/`ods`/`odp`) instead of always `.zip`
- `ftyp avif` sniffed as AVIF, not MP4
- ffmpeg `-c copy` failure deletes the staging file before re-encode
- Image `--max`/`--resize` downscale uses Triangle (Lanczos stays for upscale)
- AVIF via ffmpeg (dropped rav1e from the `image` crate)
- Spreadsheets: `tummler table.xlsx csv` / `txt` (xlsx, xlsm, xlsb, xls, ods via calamine)
- Spreadsheets: `tummler table.csv xlsx` and `xls`/`ods`/`xlsm`/`xlsb` → xlsx (rust_xlsxwriter, no LibreOffice)
- Video: container change without `-q`/`--max`/`--resize` tries ffmpeg `-c copy`, then re-encodes if needed
- `--sheet NAME` (first sheet by default); `--all-sheets` / `--sheet '*'` → `table-Sheet1.csv`
- Dropped office2pdf/Typst. Slides (`pptx`/`odp`/`ppt` → pdf and between decks) use soffice only if already on PATH
- Word/ODT (`doc`/`docx`/`odt`/`rtf` → pdf) via pandoc; soffice is used only if already on PATH (never required)
- `pptx` ↔ `ppsx` is a copy (no soffice). `pdf → pptx` is not supported. `odt`/`docx`/`md` → `pptx` still pandoc
- Rust 1.99.1 / edition 2024
- Video `-q` maps to CRF 18–38; x264 preset `medium`/`veryfast` (never `slow`); webm CRF uses `-b:v 0`
- Auto `-j` for media capped at 2; each ffmpeg gets `cpu/jobs` threads
- LibreOffice is never a crate dep and never auto-installed
- `-q` on AVIF; lossy WebP via ffmpeg; JPEG skips an extra RGB copy; EXIF orientation applied
- Pack file→zip/tar without a temp copy; spreadsheet writes CSV/TXT without a second buffer

## 0.1.0 — 2026-10-20

- `-i` / `--info`: about, Yiddish name, author, GitHub, mail
- Batch summary: `12 files, 48 MB → 11 MB (-77%)`
- `--include` / `--exclude` globs; `--skip-newer` for folders
- `--max PX` / `--resize 50%|800x|x600|800x600` for images and video
- HEIC/HEIF via ffmpeg; Ctrl-C kills ffmpeg/pandoc and exits 130
- Positional format: `tummler img.png jpg` (or `-f jpeg` / `-o out.webp`)
- Fixes: `mut cli` (build), stdin+files, in-place `--overwrite`, stdin sniff, `-preset` only for mp4/mkv
- Install: one-command `install.sh`
- Spinner for a single file, progress bar for batches (TTY, not with `--quiet`)
- Parallel batch: `-j/--jobs` (CPU count by default), prefetch ffmpeg/pandoc
- 256 KiB JPEG buffers, walkdir + BufReader for archives, `panic = abort` in release
- POSIX/GNU/clig.dev CLI: exit 0/1/2, stdout vs stderr, NO_COLOR, `--quiet`, `-n`, `--force`, `--`
- Format from `-o`: `tummler in.png -o out.webp`
- Unix filter: `cat in.png | tummler - jpeg > out.jpeg`
- Images, media (ffmpeg), documents (pandoc), archives (zip/tar/gz/…)
- Auto-install ffmpeg/pandoc in a TTY; `--completions`
- Zip-slip protection, atomic writes via a staging file
- Releases for macOS (Intel/ARM) and Linux (x86_64/ARM64)
