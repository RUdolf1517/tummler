use std::path::{Path, PathBuf};

/// Normalize an extension for classification: lowercase, no dot, aliases.
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

/// Extension used in the output filename.
/// Keep the form the user asked for: `-f jpeg` → `.jpeg`, not `.jpg`.
/// Collapse only true aliases (`tgz` → `tar.gz`).
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

/// Unix filter: path "-" means stdin/stdout.
pub fn is_stdio(path: &Path) -> bool {
    path.as_os_str() == "-"
}

/// Glob match (`*`, `?`, `**`), case-insensitive. `*` does not cross `/`.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.replace('\\', "/").to_lowercase();
    let t = text.replace('\\', "/").to_lowercase();
    glob_match_bytes(p.as_bytes(), t.as_bytes())
}

fn glob_match_bytes(pat: &[u8], text: &[u8]) -> bool {
    if pat.starts_with(b"**") {
        let rest = if pat.len() > 2 && pat[2] == b'/' {
            &pat[3..]
        } else {
            &pat[2..]
        };
        if rest.is_empty() {
            return true;
        }
        if glob_match_bytes(rest, text) {
            return true;
        }
        for i in 0..text.len() {
            if glob_match_bytes(rest, &text[i + 1..]) {
                return true;
            }
        }
        return false;
    }
    if pat.is_empty() {
        return text.is_empty();
    }
    match pat[0] {
        b'*' => {
            if glob_match_bytes(&pat[1..], text) {
                return true;
            }
            if !text.is_empty() && text[0] != b'/' {
                return glob_match_bytes(pat, &text[1..]);
            }
            false
        }
        b'?' => {
            if text.is_empty() || text[0] == b'/' {
                false
            } else {
                glob_match_bytes(&pat[1..], &text[1..])
            }
        }
        c => text.first() == Some(&c) && glob_match_bytes(&pat[1..], &text[1..]),
    }
}

/// Match GLOB against the file name or the full path.
pub fn path_matches_glob(path: &Path, pattern: &str) -> bool {
    let pat = pattern.trim();
    if pat.is_empty() {
        return false;
    }
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let full = path.to_string_lossy().replace('\\', "/");
    glob_match(pat, name) || glob_match(pat, &full)
}

pub fn passes_filters(path: &Path, include: &[String], exclude: &[String]) -> bool {
    if is_stdio(path) {
        return true;
    }
    if !include.is_empty() && !include.iter().any(|g| path_matches_glob(path, g)) {
        return false;
    }
    if exclude.iter().any(|g| path_matches_glob(path, g)) {
        return false;
    }
    true
}

pub fn is_staging_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|n| n.starts_with(".tummler-tmp-"))
}

pub fn output_up_to_date(input: &Path, output: &Path) -> bool {
    let Ok(out_m) = output.metadata() else {
        return false;
    };
    let Ok(in_m) = input.metadata() else {
        return false;
    };
    let Ok(out_t) = out_m.modified() else {
        return false;
    };
    let Ok(in_t) = in_m.modified() else {
        return false;
    };
    out_t >= in_t
}

pub fn file_size(path: &Path) -> u64 {
    path.metadata().map(|m| m.len()).unwrap_or(0)
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else if v >= 10.0 {
        format!("{v:.0} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

pub fn format_summary(
    ok: usize,
    skipped: usize,
    failed: usize,
    bytes_in: u64,
    bytes_out: u64,
) -> String {
    if ok == 0 {
        let mut parts = Vec::new();
        if skipped > 0 {
            parts.push(format!(
                "{} skipped",
                skipped
            ));
        }
        if failed > 0 {
            parts.push(format!("{failed} failed"));
        }
        if parts.is_empty() {
            return "nothing to do".into();
        }
        return parts.join(", ");
    }
    let mut parts = vec![if ok == 1 {
        "1 file".to_string()
    } else {
        format!("{ok} files")
    }];
    if skipped > 0 {
        parts.push(format!("{skipped} skipped"));
    }
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    let mut s = parts.join(", ");
    if bytes_in > 0 {
        let delta = if bytes_out < bytes_in {
            let pct = (1.0 - bytes_out as f64 / bytes_in as f64) * 100.0;
            format!(" (-{pct:.0}%)")
        } else if bytes_out > bytes_in {
            let pct = (bytes_out as f64 / bytes_in as f64 - 1.0) * 100.0;
            format!(" (+{pct:.0}%)")
        } else {
            " (0%)".into()
        };
        s.push_str(&format!(
            ", {} → {}{delta}",
            human_bytes(bytes_in),
            human_bytes(bytes_out)
        ));
    }
    s
}

/// Extension from magic bytes (stdin with no filename).
pub fn sniff_suffix(header: &[u8]) -> &'static str {
    if header.len() >= 8 && header.starts_with(&[0x89, b'P', b'N', b'G']) {
        return ".png";
    }
    if header.len() >= 3 && header[0] == 0xFF && header[1] == 0xD8 && header[2] == 0xFF {
        return ".jpg";
    }
    if header.starts_with(b"GIF8") {
        return ".gif";
    }
    if header.len() >= 12 && header.starts_with(b"RIFF") {
        return match &header[8..12] {
            b"WEBP" => ".webp",
            b"WAVE" => ".wav",
            b"AVI " => ".avi",
            _ => "",
        };
    }
    if header.starts_with(b"BM") {
        return ".bmp";
    }
    if header.starts_with(b"%PDF") {
        return ".pdf";
    }
    if header.starts_with(b"PK") {
        return ".zip";
    }
    if header.len() >= 2 && header[0] == 0x1f && header[1] == 0x8b {
        return ".gz";
    }
    if header.len() >= 12 && &header[4..8] == b"ftyp" {
        let brand = &header[8..12];
        if matches!(
            brand,
            b"heic" | b"heif" | b"mif1" | b"msf1" | b"heix" | b"hevc"
        ) {
            return ".heic";
        }
        return ".mp4";
    }
    if header.len() >= 8 && header.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return ".xls";
    }
    if header.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return ".webm";
    }
    if header.starts_with(b"fLaC") {
        return ".flac";
    }
    if header.starts_with(b"OggS") {
        return ".ogg";
    }
    if header.starts_with(b"ID3") {
        return ".mp3";
    }
    ""
}

/// If the last positional argument is a format name (`jpg`, `webp`, `tar.gz`),
/// take it out of the input list.
///
/// `tummler img.png jpg` → inputs `[img.png]`, format `jpg`.
/// `photo.jpg` is not treated as a format (it is a filename).
pub fn take_positional_format(inputs: &mut Vec<PathBuf>) -> Option<String> {
    let last = inputs.last()?;
    if is_stdio(last) {
        return None;
    }
    let raw = last.to_str()?;
    if raw.contains('/') || raw.contains('\\') {
        return None;
    }
    let token = raw.trim().trim_start_matches('.').to_lowercase();
    if token.is_empty() {
        return None;
    }
    if token.contains('.') && !matches!(token.as_str(), "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst")
    {
        return None;
    }
    if !is_supported_output(&token) {
        return None;
    }
    inputs.pop();
    Some(token)
}

/// Extension as in the filename (`photo.jpeg` → `jpeg`), for `-o`.
pub fn requested_ext_from_path(path: &Path) -> Option<String> {
    if is_stdio(path) {
        return None;
    }
    let file_name = path.file_name()?.to_string_lossy().to_lowercase();
    for double in &["tar.gz", "tar.bz2", "tar.xz", "tar.zst"] {
        if file_name.ends_with(&format!(".{double}")) {
            return Some((*double).to_string());
        }
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
}

/// File extension (normalized for classification).
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

/// Staging path next to the final file, with the same extension
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

/// Format groups
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatKind {
    Image,
    Media,        // audio/video via ffmpeg
    Document,     // via pandoc
    Spreadsheet,  // xlsx/xls/ods via calamine (input only)
    Archive,      // native
    Unknown,
}

pub fn classify_ext(ext: &str) -> FormatKind {
    let ext = normalize_ext(ext);
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif"
        | "webp" | "avif" | "pnm" | "pbm" | "pgm" | "ppm" | "tga" | "dds"
        | "hdr" | "farbfeld" | "ff" | "qoi" | "exr" | "openexr"
        | "heic" | "heif" => FormatKind::Image,

        "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg"
        | "3gp" | "m4v" | "ts" | "mts" | "m2ts" | "mxf" | "asf" | "rm" | "rmvb"
        | "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus" | "wma"
        | "aiff" | "aif" | "alac" | "ac3" | "dts" | "amr" | "au" | "ra" => FormatKind::Media,

        "pdf" | "docx" | "odt" | "html" | "htm" | "md" | "markdown" | "tex"
        | "rst" | "epub" | "pptx" | "ppt" | "ppsx" | "pps" | "odp"
        | "csv" | "json" | "txt" | "rtf"
        | "org" | "adoc" | "djvu" | "fb2" => FormatKind::Document,

        "xlsx" | "xlsm" | "xlsb" | "xls" | "ods" => FormatKind::Spreadsheet,

        "zip" | "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tar.xz" | "gz" | "bz2"
        | "xz" | "zst" | "tar.zst" => FormatKind::Archive,

        _ => FormatKind::Unknown,
    }
}

pub fn is_supported_output(ext: &str) -> bool {
    !matches!(
        classify_ext(ext),
        FormatKind::Unknown | FormatKind::Spreadsheet
    )
}

pub fn is_heif(ext: &str) -> bool {
    matches!(normalize_ext(ext).as_str(), "heic" | "heif")
}

/// Resolve the output path.
///
/// - `-o` existing directory / path ending with `/` / multiple inputs → directory
/// - `-o` file with a single input → that file
/// - no `-o` → next to the source, same stem + new extension
pub fn resolve_output_path(
    input: &Path,
    target_ext: &str,
    output_arg: Option<&Path>,
    multiple_inputs: bool,
) -> Result<PathBuf, String> {
    let target_ext = output_extension(target_ext);
    let target_ext_with_dot = format!(".{target_ext}");

    if let Some(out) = output_arg {
        if is_stdio(out) {
            return Ok(PathBuf::from("-"));
        }
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

/// Collect files to convert from inputs.
pub fn collect_inputs(inputs: &[PathBuf], recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for input in inputs {
        if is_stdio(input) || input.is_file() {
            files.push(input.clone());
        } else if input.is_dir() {
            if recursive {
                for entry in walkdir::WalkDir::new(input).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        let p = entry.path().to_path_buf();
                        if !is_staging_name(&p) {
                            files.push(p);
                        }
                    }
                }
            } else if let Ok(read_dir) = std::fs::read_dir(input) {
                for entry in read_dir.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() && !is_staging_name(&path) {
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

pub fn print_info() {
    use colored::Colorize;
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "{} {} — {}",
        "tummler".bold(),
        format!("v{version}").dimmed(),
        "טומלער".italic()
    );
    println!();
    println!("Yiddish tummler (טומלער): \"the one who stirs things up.\"");
    println!("In the Catskills resorts the tummler mixed the guests —");
    println!("introduced them, got the room going, kept the night alive.");
    println!("Also a nod to tumbler, the mixing glass.");
    println!();
    println!("This CLI does the same to files: mix any format into any other.");
    println!("Images, audio, video, documents, spreadsheets, archives. Native Rust where");
    println!("it can; ffmpeg and pandoc where it must. Format last:");
    println!();
    println!("  {}", "tummler photo.png jpg".cyan());
    println!("  {}", "tummler table.xlsx csv".cyan());
    println!("  {}", "tummler slides.pptx pdf".cyan());
    println!("  {}", "tummler ./photos webp -r --skip-newer".cyan());
    println!();
    println!("{}", "This project was written by AI.".dimmed());
    println!();
    println!("  {:8}  RUdolf1517", "author".dimmed());
    println!(
        "  {:8}  https://github.com/RUdolf1517/tummler",
        "github".dimmed()
    );
    println!(
        "  {:8}  rudolfzinovev@protonmail.com",
        "mail".dimmed()
    );
    println!();
}

pub fn print_supported_formats() {
    use colored::Colorize;
    println!("{}", "Supported tummler formats:".bold());
    println!();

    println!("  {} {}", "🖼  Images".bold(), "(native, no dependencies)".dimmed());
    println!(
        "     {}",
        "png, jpg/jpeg, gif, bmp, ico, tiff, webp, avif, heic/heif (ffmpeg), pnm, tga, dds, hdr, farbfeld, qoi, exr"
            .cyan()
    );
    println!();

    println!("  {} {}", "🎬  Audio/Video".bold(), "(requires ffmpeg)".dimmed());
    println!(
        "     {}",
        "mp4, avi, mkv, mov, webm, flv, wmv, mpg, 3gp, m4v, ts, gif (from video)".cyan()
    );
    println!(
        "     {}",
        "mp3, wav, flac, aac, ogg, m4a, opus, wma, aiff, alac".cyan()
    );
    println!();

    println!("  {} {}", "📄  Documents".bold(), "(requires pandoc)".dimmed());
    println!(
        "     {}",
        "pdf, docx, odt, html, md, tex, rst, epub, csv, json, txt, rtf".cyan()
    );
    println!();

    println!(
        "  {} {}",
        "🎞  Slides".bold(),
        "(native, no LibreOffice)".dimmed()
    );
    println!(
        "     {}",
        "pptx → pdf,  pdf → pptx".cyan()
    );
    println!();

    println!(
        "  {} {}",
        "📊  Spreadsheets".bold(),
        "(native, xlsx/xls/ods → csv/txt)".dimmed()
    );
    println!(
        "     {}",
        "xlsx, xlsm, xlsb, xls, ods  →  csv, txt".cyan()
    );
    println!();

    println!("  {} {}", "📦  Archives".bold(), "(native)".dimmed());
    println!(
        "     {}",
        "zip, tar, tar.gz/tgz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst".cyan()
    );
    println!();

    println!("{}", "Examples:".dimmed());
    println!("  {}  {}", "tummler photo.png jpeg".dimmed(), "→ photo.jpeg".dimmed());
    println!("  {}  {}", "tummler video.mp4 webm -q 80".dimmed(), "→ video.webm".dimmed());
    println!("  {}  {}", "tummler doc.docx pdf".dimmed(), "→ doc.pdf".dimmed());
    println!("  {}  {}", "tummler table.xlsx csv".dimmed(), "→ table.csv".dimmed());
    println!("  {}  {}", "tummler slides.pptx pdf".dimmed(), "→ slides.pdf".dimmed());
    println!("  {}  {}", "tummler slides.pdf pptx".dimmed(), "→ slides.pptx".dimmed());
    println!("  {}  {}", "tummler archive.zip tar.gz".dimmed(), "→ archive.tar.gz".dimmed());
    println!(
        "  {}  {}",
        "tummler ./photos webp -r --include '*.png' --skip-newer".dimmed(),
        "→ batch + filter".dimmed()
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
        assert_eq!(classify_ext("heic"), FormatKind::Image);
        assert_eq!(classify_ext("jpeg"), FormatKind::Image);
        assert_eq!(classify_ext("mp4"), FormatKind::Media);
        assert_eq!(classify_ext("pdf"), FormatKind::Document);
        assert_eq!(classify_ext("pptx"), FormatKind::Document);
        assert_eq!(classify_ext("odp"), FormatKind::Document);
        assert_eq!(classify_ext("xlsx"), FormatKind::Spreadsheet);
        assert_eq!(classify_ext("xls"), FormatKind::Spreadsheet);
        assert_eq!(classify_ext("ods"), FormatKind::Spreadsheet);
        assert_eq!(classify_ext("zip"), FormatKind::Archive);
        assert_eq!(classify_ext("xyz"), FormatKind::Unknown);
        assert!(!is_supported_output("xlsx"));
        assert!(is_supported_output("csv"));
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

    #[test]
    fn test_stdio_and_requested_ext() {
        assert!(is_stdio(Path::new("-")));
        assert!(!is_stdio(Path::new("file.png")));
        assert_eq!(
            requested_ext_from_path(Path::new("/tmp/photo.jpeg")).as_deref(),
            Some("jpeg")
        );
        assert_eq!(
            requested_ext_from_path(Path::new("a.tar.gz")).as_deref(),
            Some("tar.gz")
        );
        assert_eq!(requested_ext_from_path(Path::new("-")), None);
    }

    #[test]
    fn test_resolve_stdio_output() {
        let out = resolve_output_path(
            Path::new("a.png"),
            "jpeg",
            Some(Path::new("-")),
            false,
        )
        .unwrap();
        assert_eq!(out, Path::new("-"));
    }

    #[test]
    fn test_take_positional_format() {
        let mut inputs = vec![PathBuf::from("img.png"), PathBuf::from("jpg")];
        assert_eq!(take_positional_format(&mut inputs).as_deref(), Some("jpg"));
        assert_eq!(inputs, vec![PathBuf::from("img.png")]);

        let mut inputs = vec![PathBuf::from("a.png"), PathBuf::from("b.png"), PathBuf::from("webp")];
        assert_eq!(take_positional_format(&mut inputs).as_deref(), Some("webp"));
        assert_eq!(inputs.len(), 2);

        let mut inputs = vec![PathBuf::from("archive.zip"), PathBuf::from("tar.gz")];
        assert_eq!(take_positional_format(&mut inputs).as_deref(), Some("tar.gz"));

        let mut inputs = vec![PathBuf::from("photo.jpg")];
        assert_eq!(take_positional_format(&mut inputs), None);
        assert_eq!(inputs, vec![PathBuf::from("photo.jpg")]);

        let mut inputs = vec![PathBuf::from("-")];
        assert_eq!(take_positional_format(&mut inputs), None);
    }

    #[test]
    fn test_sniff_suffix() {
        assert_eq!(sniff_suffix(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]), ".png");
        assert_eq!(sniff_suffix(&[0xFF, 0xD8, 0xFF, 0xE0]), ".jpg");
        assert_eq!(sniff_suffix(b"GIF89a"), ".gif");
        assert_eq!(sniff_suffix(b"%PDF-1.4"), ".pdf");
        assert_eq!(sniff_suffix(b"????ftypisom"), ".mp4");
        assert_eq!(sniff_suffix(b"????ftypheic"), ".heic");
        assert_eq!(sniff_suffix(b"xxxx"), "");
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.png", "a.png"));
        assert!(glob_match("*.png", "A.PNG"));
        assert!(!glob_match("*.png", "a.jpg"));
        assert!(!glob_match("*.png", "dir/a.png"));
        assert!(glob_match("*_raw*", "img_raw_1.png"));
        assert!(glob_match("photos/*", "photos/a.png"));
        assert!(glob_match("**/*.png", "a/b/c.png"));
        assert!(path_matches_glob(Path::new("dir/a.png"), "*.png"));
        assert!(!passes_filters(
            Path::new("a.jpg"),
            &["*.png".into()],
            &[]
        ));
        assert!(!passes_filters(
            Path::new("img_raw.png"),
            &[],
            &["*_raw*".into()]
        ));
    }

    #[test]
    fn test_human_bytes_and_summary() {
        assert_eq!(human_bytes(500), "500 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        let s = format_summary(12, 0, 0, 48 * 1024 * 1024, 11 * 1024 * 1024);
        assert!(s.contains("12 files"));
        assert!(s.contains("48 MB"));
        assert!(s.contains("11 MB"));
        assert!(s.contains("-77%"));
        assert_eq!(format_summary(0, 3, 0, 0, 0), "3 skipped");
    }

    #[test]
    fn test_output_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"y").unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&b)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert!(output_up_to_date(&a, &b));
        assert!(!output_up_to_date(&b, &a));
        assert!(!output_up_to_date(&a, Path::new("/no/such/file")));
    }
}
