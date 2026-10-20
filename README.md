# tummler 🌀

Универсальный конвертер файлов — один CLI для всего. Картинки, аудио/видео, документы и архивы.

```bash
tummler photo.png -f jpeg            # photo.png → photo.jpeg
tummler photo.png -f webp -q 80      # с качеством
tummler video.mp4 -f gif             # через ffmpeg
tummler doc.docx -f pdf              # через pandoc
tummler archive.zip -f tar.gz        # перепаковка
tummler ./photos -f webp -r -o ./out # батч + рекурсия
tummler a.png b.jpg -f avif -o ./out # несколько файлов
```

Поддерживает **macOS** и **Linux** (x86_64 и ARM).

## Установка

### Через cargo (рекомендуется)

```bash
cargo install tummler
```

Требуется Rust 1.74+.

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
tummler [OPTIONS] [INPUT]...

Аргументы:
  [INPUT]...                Файлы или директории. «-» = stdin

Опции:
  -f, --format <EXT>        Целевой формат
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

Формат можно не указывать, если он есть в `-o`: `tummler in.png -o out.webp`.

`-f jpeg` пишет `.jpeg`, `-f jpg` пишет `.jpg`.

```bash
tummler image.png --format jpeg
tummler image.png -f webp
tummler image.png -o image.webp          # формат из расширения -o
tummler -- -file.png -f jpeg             # файл, имя которого начинается с -
cat in.png | tummler - -f jpeg > out.jpeg
```

### Батч-конвертация

```bash
# Несколько файлов в директорию (параллельно)
tummler a.png b.jpg c.bmp -f webp -o ./converted/

# Вся директория
tummler ./photos -f avif --overwrite

# Рекурсивно, 8 потоков
tummler ./photos -f webp -r -o ./out -j 8

# Dry-run — план на stdout (можно пайпить)
tummler ./photos -f jpeg -r -n
```

В TTY tummler показывает спиннер на одном файле и прогресс-бар на батче. В пайпах и с `--quiet` анимация выключена.

### Качество

```bash
tummler photo.png -f jpeg -q 90    # высокое качество jpeg (дефолт 90)
tummler photo.png -f webp -q 80
tummler video.mp4 -f webm -q 70    # видео (crf)
tummler video.mp4 -f mp3 -q 85     # аудио (битрейт 256k)
```

### Перезапись

```bash
tummler img.png -f jpeg              # ошибка если img.jpeg уже есть
tummler img.png -f jpeg --overwrite  # перезаписать
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
tummler video.mp4 -f gif --no-auto-install
tummler --check-deps
```

## Соглашения CLI

Ориентир — POSIX, GNU и [clig.dev](https://clig.dev/):

| Правило | Как у tummler |
|---------|----------------|
| stdout = данные, stderr = сообщения | прогресс и спиннер на stderr, только в TTY |
| Коды выхода | `0` успех, `1` ошибка конвертации, `2` неверные аргументы |
| Цвет | `NO_COLOR`, `FORCE_COLOR`, `TERM=dumb`, `--color`, `--no-color` |
| `--` конец опций | `tummler -- -file.png -f jpeg` |
| stdin/stdout | `tummler - -f jpeg` читает stdin, без `-o` пишет stdout |
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
tummler screenshot.png -f jpeg -q 85 -o compressed.jpg
tummler logo.png -f ico
tummler ./wallpapers -f avif -r -o ./avif -j 8

# Видео/аудио
tummler input.mp4 -f webm
tummler input.avi -f mp4 --ffmpeg-args "-crf 23 -preset fast"
tummler podcast.wav -f mp3 -q 90
tummler video.mp4 -f gif

# Документы
tummler report.docx -f pdf
tummler README.md -f html -o site.html

# Архивы
tummler backup.zip -f tar.gz
tummler huge.log -f gz

# Батч
tummler . -f webp -r --overwrite -q 80
tummler a.png b.jpg c.tiff -f png -o ./pngs --overwrite
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
cargo run -- photo.png -f jpeg --dry-run
cargo run -- --list-formats
cargo test
cargo build --release
```

## Лицензия

MIT — см. [LICENSE](LICENSE).
