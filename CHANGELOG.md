# Changelog

## 0.1.0 — 2026-10-20

- Формат позиционный: `tummler img.png jpg` (или `-f jpeg` / `-o out.webp`)
- Спиннер на одном файле, прогресс-бар на батче (TTY, не в `--quiet`)
- Параллельный батч: `-j/--jobs` (по умолчанию число CPU), prefetch ffmpeg/pandoc
- Буферы 256 KiB на JPEG, walkdir + BufReader на архивах, `panic = abort` в release
- CLI по POSIX/GNU/clig.dev: коды 0/1/2, stdout vs stderr, NO_COLOR, `--quiet`, `-n`, `--force`, `--`
- Формат из `-o`: `tummler in.png -o out.webp`
- Unix-фильтр: `cat in.png | tummler - -f jpeg > out.jpeg`
- Картинки, медиа (ffmpeg), документы (pandoc), архивы (zip/tar/gz/…)
- Автоустановка ffmpeg/pandoc в TTY; `--completions`
- Zip-slip защита, атомарная запись через staging-файл
- Релизы для macOS (Intel/ARM) и Linux (x86_64/ARM64)
