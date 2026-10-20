use std::path::{Path, PathBuf};

/// Нормализация расширения для классификации: lowercase, без точки, алиасы.
pub fn normalize_ext(ext: &str) -> String {
    let ext = ext.trim().trim_start_matches('.').to_lowercase();
    match ext.as_str() {
        "jpeg" | "jpe" | "jfif" => "jpg".to_string(),
        "tif" => "tiff".to_string(),
        "htm" => "html".to_string(),
        "markdown" => "md".to_string(),
        "tgz" => "tar.gz".to_string(),
        "tbz" | "tbz2" => "tar.bz2".to_string(),
        "txz" => "tar.xz".to_string(),
        _ => ext,
    }
}

/// Расширение для имени выходного файла.
/// Сохраняем форму, которую указал пользователь: `-jpeg` → `.jpeg`, не `.jpg`.
/// Схлопываем только настоящие алиасы (`tgz` → `tar.gz`).
pub fn output_extension(ext: &str) -> String {
    let ext = ext.trim().trim_start_matches('.').to_lowercase();
    match ext.as_str() {
        "jpe" | "jfif" => "jpg".to_string(),
        "tif" => "tiff".to_string(),
        "htm" => "html".to_string(),
        "markdown" => "md".to_string(),
        "tgz" => "tar.gz".to_string(),
        "tbz" | "tbz2" => "tar.bz2".to_string(),
        "txz" => "tar.xz".to_string(),
        _ => ext,
    }
}

/// Получить расширение файла (нормализованное для классификации).
pub fn get_extension(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy().to_lowercase();
    for double in &["tar.gz", "tar.bz2", "tar.xz", "tar.zst"] {
        if file_name.ends_with(&format!(".{double}")) {
            return Some(normalize_ext(double));
        }
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
}

/// Временный путь рядом с финальным, с тем же расширением
/// (`photo.jpeg` → `.tummler-tmp-photo.jpeg`).
pub fn staging_path(output: &Path) -> PathBuf {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    parent.join(format!(".tummler-tmp-{name}"))
}

/// Группы форматов
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatKind {
    Image,
    Media,    // аудио/видео через ffmpeg
    Document, // через pandoc
    Archive,  // нативно
    Unknown,
}

pub fn classify_ext(ext: &str) -> FormatKind {
    let ext = normalize_ext(ext);
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif"
        | "webp" | "avif" | "pnm" | "pbm" | "pgm" | "ppm" | "tga" | "dds"
        | "hdr" | "farbfeld" | "ff" | "qoi" | "exr" | "openexr" => FormatKind::Image,

        "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg"
        | "3gp" | "m4v" | "ts" | "mts" | "m2ts" | "mxf" | "asf" | "rm" | "rmvb"
        | "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus" | "wma"
        | "aiff" | "aif" | "alac" | "ac3" | "dts" | "amr" | "au" | "ra" => FormatKind::Media,

        "pdf" | "docx" | "odt" | "html" | "htm" | "md" | "markdown" | "tex"
        | "rst" | "epub" | "pptx" | "ppt" | "csv" | "json" | "txt" | "rtf"
        | "org" | "adoc" | "djvu" | "fb2" => FormatKind::Document,

        "zip" | "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tar.xz" | "gz" | "bz2"
        | "xz" | "zst" | "tar.zst" => FormatKind::Archive,

        _ => FormatKind::Unknown,
    }
}

pub fn is_supported_output(ext: &str) -> bool {
    classify_ext(ext) != FormatKind::Unknown
}

/// Вычислить выходной путь.
///
/// - `-o` существующая директория / путь с `/` на конце / несколько входов → директория
/// - `-o` файл при одном входе → конкретный файл
/// - без `-o` → рядом с исходником, то же имя + новое расширение
pub fn resolve_output_path(
    input: &Path,
    target_ext: &str,
    output_arg: Option<&Path>,
    multiple_inputs: bool,
) -> Result<PathBuf, String> {
    let target_ext = output_extension(target_ext);
    let target_ext_with_dot = format!(".{target_ext}");

    if let Some(out) = output_arg {
        let is_dir_hint = out.to_string_lossy().ends_with('/')
            || out.to_string_lossy().ends_with('\\')
            || (out.exists() && out.is_dir())
            || multiple_inputs;

        if is_dir_hint {
            let stem = strip_double_ext_stem(input).unwrap_or_else(|| {
                input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output")
                    .to_string()
            });
            return Ok(out.join(format!("{stem}{target_ext_with_dot}")));
        }

        if let Some(out_ext) = get_extension(out) {
            if normalize_ext(&out_ext) == normalize_ext(&target_ext) {
                return Ok(out.to_path_buf());
            }
        }
        if out.extension().is_none() && !target_ext.contains('.') {
            let mut p = out.as_os_str().to_os_string();
            p.push(&target_ext_with_dot);
            return Ok(PathBuf::from(p));
        }
        let parent = out.parent();
        let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        if let Some(parent) = parent {
            if parent.as_os_str().is_empty() {
                return Ok(PathBuf::from(format!("{stem}{target_ext_with_dot}")));
            }
            return Ok(parent.join(format!("{stem}{target_ext_with_dot}")));
        }
        return Ok(out.to_path_buf());
    }

    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = strip_double_ext_stem(input).unwrap_or_else(|| {
        input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
            .to_string()
    });
    Ok(parent.join(format!("{stem}{target_ext_with_dot}")))
}

fn strip_double_ext_stem(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    let lower = file_name.to_lowercase();
    for double in &["tar.gz", "tar.bz2", "tar.xz", "tar.zst"] {
        let suffix = format!(".{double}");
        if lower.ends_with(&suffix) {
            let stem = &file_name[..file_name.len() - suffix.len()];
            return Some(stem.to_string());
        }
    }
    None
}

/// Собрать список файлов для конвертации из inputs.
pub fn collect_inputs(inputs: &[PathBuf], recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_file() {
            files.push(input.clone());
        } else if input.is_dir() {
            if recursive {
                for entry in walkdir::WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
            } else if let Ok(read_dir) = std::fs::read_dir(input) {
                for entry in read_dir.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        files.push(path);
                    }
                }
            }
        } else {
            files.push(input.clone());
        }
    }
    files
}

pub fn print_supported_formats() {
    use colored::Colorize;
    println!("{}", "Поддерживаемые форматы tummler:".bold());
    println!();

    println!("  {} {}", "🖼  Картинки".bold(), "(нативно, без зависимостей)".dimmed());
    println!(
        "     {}",
        "png, jpg/jpeg, gif, bmp, ico, tiff, webp, avif, pnm, tga, dds, hdr, farbfeld, qoi, exr"
            .cyan()
    );
    println!();

    println!("  {} {}", "🎬  Аудио/Видео".bold(), "(требует ffmpeg)".dimmed());
    println!(
        "     {}",
        "mp4, avi, mkv, mov, webm, flv, wmv, mpg, 3gp, m4v, ts, gif (из видео)".cyan()
    );
    println!(
        "     {}",
        "mp3, wav, flac, aac, ogg, m4a, opus, wma, aiff, alac".cyan()
    );
    println!();

    println!("  {} {}", "📄  Документы".bold(), "(требует pandoc)".dimmed());
    println!(
        "     {}",
        "pdf, docx, odt, html, md, tex, rst, epub, pptx, csv, json, txt, rtf".cyan()
    );
    println!();

    println!("  {} {}", "📦  Архивы".bold(), "(нативно)".dimmed());
    println!(
        "     {}",
        "zip, tar, tar.gz/tgz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst".cyan()
    );
    println!();

    println!("{}", "Примеры:".dimmed());
    println!("  {}  {}", "tummler photo.png -jpeg".dimmed(), "→ photo.jpeg".dimmed());
    println!("  {}  {}", "tummler video.mp4 -webm -q 80".dimmed(), "→ video.webm".dimmed());
    println!("  {}  {}", "tummler doc.docx -pdf".dimmed(), "→ doc.pdf".dimmed());
    println!("  {}  {}", "tummler archive.zip -tar.gz".dimmed(), "→ archive.tar.gz".dimmed());
    println!(
        "  {}  {}",
        "tummler ./photos -webp -r -o ./out".dimmed(),
        "→ батч + рекурсия".dimmed()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_ext("JPEG"), "jpg");
        assert_eq!(normalize_ext(".png"), "png");
        assert_eq!(normalize_ext("TGZ"), "tar.gz");
        assert_eq!(normalize_ext("jfif"), "jpg");
    }

    #[test]
    fn test_output_extension_preserves_jpeg() {
        assert_eq!(output_extension("jpeg"), "jpeg");
        assert_eq!(output_extension("JPEG"), "jpeg");
        assert_eq!(output_extension(".jpg"), "jpg");
        assert_eq!(output_extension("tgz"), "tar.gz");
        assert_eq!(output_extension("webp"), "webp");
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify_ext("png"), FormatKind::Image);
        assert_eq!(classify_ext("jpeg"), FormatKind::Image);
        assert_eq!(classify_ext("mp4"), FormatKind::Media);
        assert_eq!(classify_ext("pdf"), FormatKind::Document);
        assert_eq!(classify_ext("zip"), FormatKind::Archive);
        assert_eq!(classify_ext("xyz"), FormatKind::Unknown);
    }

    #[test]
    fn test_resolve_no_output() {
        let input = Path::new("/tmp/photo.png");
        let out = resolve_output_path(input, "jpg", None, false).unwrap();
        assert_eq!(out, Path::new("/tmp/photo.jpg"));
    }

    #[test]
    fn test_resolve_jpeg_keeps_jpeg() {
        let input = Path::new("/tmp/photo.png");
        let out = resolve_output_path(input, "jpeg", None, false).unwrap();
        assert_eq!(out, Path::new("/tmp/photo.jpeg"));
    }

    #[test]
    fn test_resolve_with_dir() {
        let input = Path::new("/tmp/a.png");
        let out = resolve_output_path(input, "webp", Some(Path::new("/out")), true).unwrap();
        assert_eq!(out, Path::new("/out/a.webp"));
    }

    #[test]
    fn test_resolve_tar_gz_stem() {
        let input = Path::new("/tmp/Archive.TAR.GZ");
        let out = resolve_output_path(input, "zip", None, false).unwrap();
        assert_eq!(out, Path::new("/tmp/Archive.zip"));
    }

    #[test]
    fn test_staging_path_keeps_extension() {
        let p = Path::new("/tmp/photo.jpeg");
        let s = staging_path(p);
        assert_eq!(s, Path::new("/tmp/.tummler-tmp-photo.jpeg"));
        assert_eq!(get_extension(&s).as_deref(), Some("jpg"));

        let p = Path::new("/tmp/archive.tar.gz");
        let s = staging_path(p);
        assert_eq!(s, Path::new("/tmp/.tummler-tmp-archive.tar.gz"));
        assert_eq!(get_extension(&s).as_deref(), Some("tar.gz"));
    }

    #[test]
    fn test_get_extension_double() {
        assert_eq!(
            get_extension(Path::new("a.tar.gz")).as_deref(),
            Some("tar.gz")
        );
        assert_eq!(get_extension(Path::new("a.PNG")).as_deref(), Some("png"));
    }
}
