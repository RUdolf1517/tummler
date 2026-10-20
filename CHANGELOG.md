# Changelog

## 0.1.0 — 2026-10-20

- Первый релиз
- Картинки (image crate): png, jpg, gif, bmp, ico, tiff, webp, avif, pnm, tga, dds, hdr, farbfeld, qoi, exr
- Аудио/видео (ffmpeg): mp4, avi, mkv, mov, webm, mp3, wav, flac, aac, ogg, m4a, opus и др.
- Документы (pandoc): pdf, docx, odt, html, md, tex, rst, epub, pptx
- Архивы (нативно): zip, tar, tar.gz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst
- Синтаксис `tummler file.png -jpeg` + `--format` (выход `file.jpeg`, не `.jpg`)
- Батч, рекурсия, качество, overwrite, dry-run, прогресс-бар
- Автоустановка ffmpeg/pandoc с подтверждением в TTY (brew/apt/dnf/pacman/zypper)
- Сжатие обычных файлов: `tummler huge.log -gz`
- Атомарная запись через staging-файл (ошибка не затирает существующий выход)
- `--completions` для bash/zsh/fish/powershell/elvish
- `--list-formats` / `--check-deps` работают без входного файла
- Защита от zip-slip при распаковке
- Релизы для macOS (Intel/ARM) и Linux (x86_64/ARM64)
