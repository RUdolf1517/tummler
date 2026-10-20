# Changelog

## Unreleased

- Spreadsheets: `tummler table.xlsx csv` / `txt` (xlsx, xlsm, xlsb, xls, ods via calamine)
- `--sheet NAME` (first sheet by default); `--all-sheets` / `--sheet '*'` → `table-Sheet1.csv`
- Slides: `tummler slides.pptx pdf` (office2pdf) and `tummler slides.pdf pptx` (hayro), no LibreOffice
- Rust 1.99.1 / edition 2024; pdf→pptx streams pages and stores PNGs uncompressed in the zip

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
