# tummler 🌀

Универсальный конвертер файлов — один CLI для всего. Картинки, аудио/видео, документы и архивы.

```bash
tummler photo.png jpg                # photo.png → photo.jpg
tummler photo.png jpeg -q 80         # → photo.jpeg, с качеством
tummler video.mp4 gif                # через ffmpeg
tummler doc.docx pdf                 # через pandoc
tummler archive.zip tar.gz           # перепаковка
tummler ./photos webp -r -o ./out    # батч + рекурсия
tummler a.png b.jpg avif -o ./out    # несколько файлов
```

Поддерживает **macOS** и **Linux** (x86_64 и ARM).

## Установка

Одной командой:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

Пока релиза нет — из этой ветки:

```bash
cargo install --git https://github.com/RUdolf1517/tummler --branch arena/ce98465f-tummler
```

Требуется Rust 1.74+ (для сборки из исходников). Скрипт ставит бинарник в `~/.local/bin`.

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
tummler [OPTIONS] <INPUT>... [FORMAT]

Аргументы:
  <INPUT>...                Файлы или директории. «-» = stdin
  [FORMAT]                  Целевой формат последним: jpg, webp, mp4, pdf, tar.gz…

Опции:
  -f, --format <EXT>        Формат явно (вместо позиционного)
  -o, --output <PATH>       Выходной файл или директория. «-» = stdout
  -q, --quality <1-100>     Качество lossy-форматов
      --overwrite, --force  Перезаписать существующие файлы
  -r, --recursive           Рекурсивный обход директорий
  -j, --jobs <N>            Параллельные задачи (0 = число CPU)
  -v, --verbose             Подробный лог на stderr
      --quiet               Только ошибки (для скриптов)
  -n, --dry-run             Показать план, ничего не писать
      --color <WHEN>        auto | always | never
      --no-color            То же, что --color never
      --list-formats        Список форматов
      --check-deps          Проверить ffmpeg/pandoc
      --no-auto-install     Не ставить зависимости
  -y, --yes                 Ставить зависимости без вопроса
      --ffmpeg-args <ARGS>  Доп. аргументы ffmpeg
      --completions <SHELL> bash | zsh | fish | powershell | elvish
  -h, --help                Справка
  -V, --version             Версия
```

Основной синтаксис — формат последним аргументом:

```bash
tummler image.png jpg                    # → image.jpg
tummler image.png jpeg                   # → image.jpeg
tummler image.png webp -o out/
tummler image.png -o image.webp          # формат из расширения -o
tummler -- -file.png jpeg                # файл, имя которого начинается с -
cat in.png | tummler - jpeg > out.jpeg
```

`-f` / `--format` тоже работают: `tummler image.png -f webp`.

### Батч-конвертация

```bash
# Несколько файлов в директорию (параллельно)
tummler a.png b.jpg c.bmp webp -o ./converted/

# Вся директория
tummler ./photos avif --overwrite

# Рекурсивно, 8 потоков
tummler ./photos webp -r -o ./out -j 8

# Dry-run — план на stdout (можно пайпить)
tummler ./photos jpeg -r -n
```

В TTY tummler показывает спиннер на одном файле и прогресс-бар на батче. В пайпах и с `--quiet` анимация выключена.

### Качество

```bash
tummler photo.png jpeg -q 90    # высокое качество jpeg (дефолт 90)
tummler photo.png webp -q 80
tummler video.mp4 webm -q 70    # видео (crf)
tummler video.mp4 mp3 -q 85     # аудио (битрейт 256k)
```

### Перезапись

```bash
tummler img.png jpeg              # ошибка если img.jpeg уже есть
tummler img.png jpeg --overwrite  # перезаписать
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

Если зависимость не найдена, tummler предложит установить её автоматически в TTY. В скриптах: `--yes` чтобы поставить, `--no-auto-install` чтобы никогда не ставить.

```bash
tummler video.mp4 gif --no-auto-install
tummler --check-deps
```

## Соглашения CLI

Ориентир — POSIX, GNU и [clig.dev](https://clig.dev/):

| Правило | Как у tummler |
|---------|----------------|
| stdout = данные, stderr = сообщения | прогресс и спиннер на stderr, только в TTY |
| Коды выхода | `0` успех, `1` ошибка конвертации, `2` неверные аргументы |
| Цвет | `NO_COLOR`, `FORCE_COLOR`, `TERM=dumb`, `--color`, `--no-color` |
| `--` конец опций | `tummler -- -file.png jpeg` |
| stdin/stdout | `tummler - jpeg` читает stdin, без `-o` пишет stdout |
| Короткие флаги | склеиваются: `-rvn` |
| `--quiet` | только ошибки; `-q` занят качеством |

Ошибки в GNU-стиле: `tummler: файл не найден: foo.png`.

## Автодополнение

```bash
# bash
tummler --completions bash | sudo tee /etc/bash_completion.d/tummler

# zsh
mkdir -p ~/.zfunc
tummler --completions zsh > ~/.zfunc/_tummler

# fish
tummler --completions fish > ~/.config/fish/completions/tummler.fish
```

## Примеры

```bash
# Картинки
tummler screenshot.png jpeg -q 85 -o compressed.jpg
tummler logo.png ico
tummler ./wallpapers avif -r -o ./avif -j 8

# Видео/аудио
tummler input.mp4 webm
tummler input.avi mp4 --ffmpeg-args "-crf 23 -preset fast"
tummler podcast.wav mp3 -q 90
tummler video.mp4 gif

# Документы
tummler report.docx pdf
tummler README.md html -o site.html

# Архивы
tummler backup.zip tar.gz
tummler huge.log gz

# Батч
tummler . webp -r --overwrite -q 80
tummler a.png b.jpg c.tiff png -o ./pngs --overwrite
```

## Как это работает

```
tummler ─┬─ Картинки  →  image crate (нативно)
         ├─ Архивы    →  zip/tar/flate2/bzip2/xz2/zstd (нативно)
         ├─ Аудио/Видео → ffmpeg (подпроцесс)
         └─ Документы →  pandoc (подпроцесс)
```

Батч картинок и архивов идёт параллельно (`-j`). Зависимости ffmpeg/pandoc проверяются один раз до старта пула.

## Разработка

```bash
cargo run -- photo.png jpg --dry-run
cargo run -- --list-formats
cargo test
cargo build --release
```

## Лицензия

MIT — см. [LICENSE](LICENSE).
