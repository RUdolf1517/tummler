# tummler 🌀

Универсальный конвертер файлов — один CLI для всего. Картинки, аудио/видео, документы и архивы.

```bash
tummler photo.png -jpeg              # photo.png → photo.jpeg
tummler photo.png -webp -q 80        # с качеством
tummler video.mp4 -gif                # через ffmpeg
tummler doc.docx -pdf                 # через pandoc
tummler archive.zip -tar.gz           # перепаковка
tummler ./photos -webp -r -o ./out    # батч + рекурсия
tummler a.png b.jpg -avif -o ./out    # несколько файлов
```

Поддерживает **macOS** и **Linux** (x86_64 и ARM).

## Установка

### Через cargo (рекомендуется)

```bash
cargo install tummler
```

Требуется Rust 1.70+.

### Готовые бинарники

Скачай последний релиз со страницы [Releases](https://github.com/RUdolf1517/tummler/releases):

| Платформа | Файл |
|-----------|------|
| macOS Apple Silicon | `tummler-aarch64-apple-darwin` |
| macOS Intel | `tummler-x86_64-apple-darwin` |
| Linux x86_64 | `tummler-x86_64-unknown-linux-gnu` |
| Linux ARM64 | `tummler-aarch64-unknown-linux-gnu` |

```bash
chmod +x tummler-*
sudo mv tummler-* /usr/local/bin/tummler
tummler --help
```

### Из исходников

На Linux понадобятся заголовки сжатия:

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

## Использование

```
tummler <INPUT>... -<EXT> [ОПЦИИ]

Аргументы:
  <INPUT>...        Входные файлы или директории (один или несколько)

Опции:
  -f, --format <EXT>        Целевой формат (альтернатива `-jpeg` синтаксису)
  -o, --output <PATH>       Выходной файл или директория
  -q, --quality <1-100>     Качество для lossy-форматов (jpeg, webp, видео)
      --overwrite           Перезаписывать существующие файлы
  -r, --recursive           Рекурсивный обход директорий
  -v, --verbose             Подробный вывод
      --dry-run             Показать что будет сделано без конвертации
      --list-formats        Список всех поддерживаемых форматов
      --check-deps          Проверить внешние зависимости
      --no-auto-install     Не предлагать автоустановку, только подсказка
      --ffmpeg-args <ARGS>  Доп. аргументы для ffmpeg: "-crf 23 -preset fast"
      --completions <SHELL> Скрипт автодополнения: bash, zsh, fish, powershell, elvish
  -h, --help                Справка
  -V, --version             Версия
```

### Синтаксис `-расширение`

Главная фишка — короткий синтаксис `tummler файл -jpeg` без лишних флагов:

```bash
tummler image.png -jpeg          # → image.jpeg рядом с исходником
tummler image.png -webp -o out/  # → out/image.webp
tummler video.mp4 -mp3            # извлечь аудио
tummler archive.zip -tar.gz       # перепаковка
```

Работает с любым расширением: `-jpeg`, `-webp`, `-pdf`, `-tar.gz` и т.д.
`-jpeg` пишет `.jpeg`, `-jpg` пишет `.jpg`. Можно и классически:

```bash
tummler image.png --format jpeg
tummler image.png -f webp
```

### Батч-конвертация

```bash
# Несколько файлов в директорию
tummler a.png b.jpg c.bmp -webp -o ./converted/

# Вся директория
tummler ./photos -avif --overwrite

# Рекурсивно со вложенными папками
tummler ./photos -webp -r -o ./out

# Dry-run — посмотреть что будет
tummler ./photos -jpeg -r --dry-run

# С прогресс-баром и итогами
tummler ./huge-folder -webp -r
# █▓▒░  [00:00:12] [████████████░░░░░░░░] 142/300 photo123.png → photo123.webp
```

### Качество

```bash
tummler photo.png -jpeg -q 90    # высокое качество jpeg (дефолт 90)
tummler photo.png -webp -q 80    # webp
tummler video.mp4 -webm -q 70    # видео (crf)
tummler video.mp4 -mp3 -q 85     # аудио (битрейт 256k)
```

### Перезапись

```bash
tummler img.png -jpeg              # ошибка если img.jpeg уже есть
tummler img.png -jpeg --overwrite # перезаписать
```

## Поддерживаемые форматы

| Категория | Форматы | Зависимости |
|-----------|---------|-------------|
| 🖼 Картинки | `png, jpg/jpeg, gif, bmp, ico, tiff, webp, avif, pnm, tga, dds, hdr, farbfeld, qoi, exr` | нет (чистый Rust) |
| 🎬 Аудио/Видео | `mp4, avi, mkv, mov, webm, flv, wmv, mpg, 3gp, m4v, ts, mp3, wav, flac, aac, ogg, m4a, opus, wma, aiff` | `ffmpeg` |
| 📄 Документы | `pdf, docx, odt, html, md, tex, rst, epub, pptx, csv, json, txt, rtf` | `pandoc` (+ LaTeX для PDF) |
| 📦 Архивы | `zip, tar, tar.gz/tgz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst` | нет (чистый Rust) |

Посмотреть в терминале:

```bash
tummler --list-formats
```

## Зависимости

Картинки и архивы работают без внешних программ. Для остального нужен:

- **ffmpeg** — аудио/видео (`brew install ffmpeg` / `sudo apt install ffmpeg`)
- **pandoc** — документы (`brew install pandoc` / `sudo apt install pandoc`)
  - Для `→ pdf` нужен LaTeX: `brew install --cask mactex` / `sudo apt install texlive-latex-recommended`

### Автоустановка

Если зависимость не найдена, tummler предложит установить её автоматически:

```
⚠ ffmpeg не найден в PATH.
Установите ffmpeg:
  macOS:          brew install ffmpeg
  Debian/Ubuntu:  sudo apt install ffmpeg

Запустить `brew install ffmpeg` для установки ffmpeg? [Y/n]: Y
→ Запускаю: brew install ffmpeg
✓ ffmpeg успешно установлен!
```

Отключить автоустановку:

```bash
tummler video.mp4 -gif --no-auto-install
# или
tummler --check-deps   # только проверить
```

Автоустановка спрашивает подтверждение только в интерактивном терминале (TTY). В скриптах и CI tummler не будет запускать `sudo apt` сам.

## Автодополнение

```bash
# bash
tummler --completions bash | sudo tee /etc/bash_completion.d/tummler

# zsh
mkdir -p ~/.zfunc
tummler --completions zsh > ~/.zfunc/_tummler
# в ~/.zshrc: fpath+=~/.zfunc && autoload -Uz compinit && compinit

# fish
tummler --completions fish > ~/.config/fish/completions/tummler.fish
```

Ручная проверка:

```bash
tummler --check-deps
# Зависимости tummler:
#   ffmpeg        ✓ установлен  — Аудио/видео конвертация
#   ffprobe       ○ не найден (опционально)  — Анализ медиафайлов
#   pandoc        ✓ установлен  — Конвертация документов
#   Нативные:     картинки (image crate), архивы (zip/tar)
```

## Примеры

```bash
# Картинки
tummler screenshot.png -jpeg -q 85 -o compressed.jpg
tummler logo.png -ico
tummler ./wallpapers -avif -r -o ./avif

# Видео/аудио
tummler input.mp4 -webm
tummler input.avi -mp4 --ffmpeg-args "-crf 23 -preset fast"
tummler podcast.wav -mp3 -q 90
tummler video.mp4 -gif                   # видео → gif

# Документы
tummler report.docx -pdf
tummler slides.pptx -pdf
tummler README.md -html -o site.html
tummler manual.md -epub

# Архивы
tummler backup.zip -tar.gz
tummler data.tar.bz2 -zip
tummler huge.log -gz

# Комбинированные
tummler . -webp -r --overwrite -q 80     # все картинки в текущей директории
tummler a.png b.jpg c.tiff -png -o ./pngs --overwrite
```

## Как это работает

```
tummler ─┬─ Картинки  →  image crate (нативно, без внешних зависимостей)
         ├─ Архивы    →  zip/tar/flate2/bzip2/xz2/zstd (нативно)
         ├─ Аудио/Видео → ffmpeg (подпроцесс)
         └─ Документы →  pandoc (подпроцесс)
```

Архитектура "оркестратор" — как у [MCVT](https://dev.to/arshalaromal/i-got-tired-of-googling-ffmpeg-flags-so-i-built-a-universal-cli-converter-in-rust-45l9) и [image-converter-rs](https://github.com/JohnDotOwl/image-converter-rs): используем лучшие инструменты для каждой категории, а tummler даёт единый интерфейс.

## Разработка

```bash
cargo run -- photo.png -jpeg --dry-run
cargo run -- --list-formats
cargo run -- --check-deps
cargo test
cargo build --release
```

## Лицензия

MIT — см. [LICENSE](LICENSE).
