use crate::deps;
use crate::error::TummlerError;
use crate::geom::Geometry;
use crate::interrupt;
use crate::util::{classify_ext, is_ffmpeg_image, is_pdf, normalize_ext, FormatKind};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{image, media, soffice, ConvertOpts};

/// Writer-ish targets we can fill from extracted PDF text (and soffice for a few).
pub(crate) fn is_pdf_doc_target(ext: &str) -> bool {
    matches!(
        normalize_ext(ext).as_str(),
        "docx" | "odt" | "rtf" | "txt" | "md" | "html" | "epub" | "rst" | "org" | "adoc"
    )
}

fn soffice_writer_target(ext: &str) -> bool {
    matches!(ext, "docx" | "odt" | "rtf")
}

/// PDF → png/jpg/… (first page) and PDF → docx/txt/… .
pub fn convert_pdf(
    input: &Path,
    output: &Path,
    target_ext: &str,
    opts: &ConvertOpts<'_>,
) -> Result<(), TummlerError> {
    let target = normalize_ext(target_ext);
    if is_pdf(&target) {
        std::fs::copy(input, output).map_err(|e| {
            TummlerError::IoError(format!("failed to write {}: {e}", output.display()))
        })?;
        return Ok(());
    }

    if classify_ext(&target) == FormatKind::Image {
        return pdf_to_image(input, output, &target, opts);
    }

    if is_pdf_doc_target(&target) {
        return pdf_to_document(input, output, &target, opts);
    }

    Err(TummlerError::UnsupportedConversion(
        "pdf".into(),
        target,
    ))
}

fn pdf_to_image(
    input: &Path,
    output: &Path,
    target: &str,
    opts: &ConvertOpts<'_>,
) -> Result<(), TummlerError> {
    let tmp = tempfile::tempdir().map_err(|e| TummlerError::IoError(e.to_string()))?;
    let (raster, already_sized) = raster_first_page(input, tmp.path(), opts)?;
    let noop = Geometry::default();
    let geometry = if already_sized {
        &noop
    } else {
        opts.geometry
    };
    if target == "png" && geometry.is_noop() {
        std::fs::copy(&raster, output).map_err(|e| {
            TummlerError::IoError(format!("failed to write {}: {e}", output.display()))
        })?;
        return Ok(());
    }
    if is_ffmpeg_image(target) || (target == "webp" && opts.quality.is_some()) {
        return media::convert_media(
            &raster,
            output,
            opts.quality,
            opts.ffmpeg_args,
            opts.no_auto_install,
            opts.assume_yes,
            geometry,
            opts.ffmpeg_threads,
        );
    }
    image::convert_image(&raster, output, opts.quality, geometry)
}

/// 150 dpi pixels for a PDF page in points (poppler's default raster).
fn pts_to_px(pts: f32) -> u32 {
    (pts * 150.0 / 72.0).round().max(1.0) as u32
}

fn parse_pdfinfo_page_pts(text: &str) -> Option<(f32, f32)> {
    let mut size = None;
    let mut rot = 0i32;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Page size:") {
            let mut parts = rest.split_whitespace();
            let w: f32 = parts.next()?.parse().ok()?;
            if parts.next()? != "x" {
                return None;
            }
            let h: f32 = parts.next()?.parse().ok()?;
            size = Some((w, h));
        } else if let Some(rest) = line.strip_prefix("Page rot:") {
            rot = rest.trim().parse().unwrap_or(0);
        }
    }
    let (w, h) = size?;
    if rot.rem_euclid(180) == 90 {
        Some((h, w))
    } else {
        Some((w, h))
    }
}

fn pdf_page_pts(input: &Path) -> Option<(f32, f32)> {
    let out = Command::new("pdfinfo").arg(input).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_pdfinfo_page_pts(&String::from_utf8_lossy(&out.stdout))
}

/// pdftoppm size flags + whether the PNG is already at `geometry.target_size`.
fn pdftoppm_fit(geometry: &Geometry, page_pts: Option<(f32, f32)>) -> (Vec<String>, bool) {
    if let Some((pw, ph)) = page_pts {
        let w = pts_to_px(pw);
        let h = pts_to_px(ph);
        let (tw, th) = geometry.target_size(w, h);
        if tw == w && th == h {
            return (vec!["-r".into(), "150".into()], true);
        }
        return (
            vec!["-scale-to".into(), tw.max(th).to_string()],
            true,
        );
    }
    match (geometry.resize.is_none(), geometry.max) {
        (true, None) => (vec!["-r".into(), "150".into()], true),
        (true, Some(max)) => (vec!["-scale-to".into(), max.to_string()], true),
        _ => (vec!["-r".into(), "150".into()], false),
    }
}

fn raster_first_page(
    input: &Path,
    tmp: &Path,
    opts: &ConvertOpts<'_>,
) -> Result<(PathBuf, bool), TummlerError> {
    deps::ensure_available("pdftoppm", opts.no_auto_install, opts.assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("pdftoppm".into(), e))?;

    let (scale, already_sized) = pdftoppm_fit(opts.geometry, pdf_page_pts(input));
    let prefix = tmp.join("page");
    let mut cmd = Command::new("pdftoppm");
    cmd.args(["-png", "-f", "1", "-l", "1"]);
    for a in &scale {
        cmd.arg(a);
    }
    cmd.arg(input).arg(&prefix);

    let result = interrupt::run(&mut cmd)
        .map_err(|e| TummlerError::PdfError(format!("pdftoppm failed to start: {e}")))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(TummlerError::PdfError(format!(
            "pdftoppm failed (exit {:?}):\n{stderr}{stdout}",
            result.status.code()
        )));
    }

    if let Some(png) = first_png(tmp) {
        return Ok((png, already_sized));
    }
    Err(TummlerError::PdfError(
        "pdftoppm produced no PNG for page 1".into(),
    ))
}

fn first_png(dir: &Path) -> Option<PathBuf> {
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        })
        .collect();
    pngs.sort();
    pngs.into_iter().next()
}

fn pdf_to_document(
    input: &Path,
    output: &Path,
    target: &str,
    opts: &ConvertOpts<'_>,
) -> Result<(), TummlerError> {
    if soffice_writer_target(target) && deps::soffice_bin().is_some() {
        match soffice::convert_pdf_import(input, output, target) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if opts.verbose {
                    eprintln!("  soffice PDF import failed, falling back to text: {e}");
                }
            }
        }
    }

    if target == "txt" {
        return pdftotext(input, output, opts);
    }

    let tmp = tempfile::tempdir().map_err(|e| TummlerError::IoError(e.to_string()))?;
    let txt = tmp.path().join("page.txt");
    pdftotext(input, &txt, opts)?;
    pandoc_plain(&txt, output, opts)
}

fn pdftotext(input: &Path, output: &Path, opts: &ConvertOpts<'_>) -> Result<(), TummlerError> {
    deps::ensure_available("pdftotext", opts.no_auto_install, opts.assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("pdftotext".into(), e))?;

    let mut cmd = Command::new("pdftotext");
    cmd.args(["-layout", "-nopgbrk"]).arg(input).arg(output);
    let result = interrupt::run(&mut cmd)
        .map_err(|e| TummlerError::PdfError(format!("pdftotext failed to start: {e}")))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(TummlerError::PdfError(format!(
            "pdftotext failed (exit {:?}):\n{stderr}{stdout}",
            result.status.code()
        )));
    }
    Ok(())
}

fn pandoc_plain(
    input_txt: &Path,
    output: &Path,
    opts: &ConvertOpts<'_>,
) -> Result<(), TummlerError> {
    deps::ensure_available("pandoc", opts.no_auto_install, opts.assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("pandoc".into(), e))?;

    let to = output
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
        .unwrap_or_else(|| "docx".into());
    let mut cmd = Command::new("pandoc");
    cmd.args(["-f", "plain", "-t", &to, "-o"])
        .arg(output)
        .arg(input_txt);
    let result = interrupt::run(&mut cmd)
        .map_err(|e| TummlerError::DocumentError(e.to_string()))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(TummlerError::DocumentError(format!(
            "pandoc failed (exit {:?}):\n{stderr}{stdout}",
            result.status.code()
        )));
    }
    Ok(())
}

/// One-page Helvetica PDF. Offsets are precomputed (poppler is picky).
#[cfg(test)]
pub(crate) fn sample_pdf(_text: &str) -> Vec<u8> {
    b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n4 0 obj\n<< /Length 45 >>\nstream\nBT /F1 24 Tf 72 720 Td (Hello tummler) Tj ET\nendstream\nendobj\n5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\nxref\n0 6\n0000000000 65535 f \n0000000015 00000 n \n0000000064 00000 n \n0000000121 00000 n \n0000000247 00000 n \n0000000341 00000 n \ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n411\n%%EOF\n".to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts<'a>(geometry: &'a Geometry, overwrite: bool) -> ConvertOpts<'a> {
        ConvertOpts {
            quality: None,
            overwrite,
            ffmpeg_args: None,
            no_auto_install: true,
            assume_yes: false,
            verbose: false,
            geometry,
            sheet: None,
            ffmpeg_threads: 0,
        }
    }

    fn write_sample(dir: &Path) -> PathBuf {
        let input = dir.join("hello.pdf");
        std::fs::write(&input, sample_pdf("Hello tummler")).unwrap();
        input
    }

    #[test]
    fn pdf_doc_targets() {
        assert!(is_pdf_doc_target("docx"));
        assert!(is_pdf_doc_target("txt"));
        assert!(!is_pdf_doc_target("pptx"));
        assert!(!is_pdf_doc_target("png"));
        assert!(!is_pdf_doc_target("xlsx"));
    }

    #[test]
    fn sample_pdf_header() {
        let bytes = sample_pdf("Hi");
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert_eq!(bytes.len(), 594);
        assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
        assert!(is_pdf("PDF"));
        assert!(is_pdf(".pdf"));
        assert!(!is_pdf("png"));
    }

    #[test]
    fn pdf_to_pptx_is_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.pptx");
        let g = Geometry::default();
        let err = convert_pdf(&input, &out, "pptx", &opts(&g, true)).unwrap_err();
        match err {
            TummlerError::UnsupportedConversion(src, dst) => {
                assert_eq!(src, "pdf");
                assert_eq!(dst, "pptx");
            }
            other => panic!("expected UnsupportedConversion, got {other}"),
        }
        assert!(!out.exists());
    }

    #[test]
    fn pdf_to_png_first_page() {
        if !deps::is_available("pdftoppm") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.png");
        let g = Geometry::default();
        convert_pdf(&input, &out, "png", &opts(&g, true))
            .unwrap_or_else(|e| panic!("pdf→png: {e}"));
        assert!(out.exists());
        let decoded = ::image::open(&out).unwrap();
        assert!(decoded.width() >= 100);
        assert!(decoded.height() >= 100);
    }

    #[test]
    fn pdfinfo_page_pts() {
        let letter = "Pages:          1\nPage size:      612 x 792 pts (letter)\nPage rot:       0\n";
        assert_eq!(parse_pdfinfo_page_pts(letter), Some((612.0, 792.0)));
        let a4 = "Page size:      595.28 x 841.89 pts (A4)\nPage rot:       90\n";
        let (w, h) = parse_pdfinfo_page_pts(a4).unwrap();
        assert!((w - 841.89).abs() < 0.01);
        assert!((h - 595.28).abs() < 0.01);
        assert!(parse_pdfinfo_page_pts("Pages: 1\n").is_none());
    }

    #[test]
    fn pdftoppm_fit_max_and_resize() {
        let letter = Some((612.0, 792.0));
        let (args, sized) = pdftoppm_fit(&Geometry::default(), letter);
        assert_eq!(args, ["-r", "150"]);
        assert!(sized);

        let g = Geometry {
            max: Some(200),
            resize: None,
        };
        let (args, sized) = pdftoppm_fit(&g, letter);
        assert_eq!(args[0], "-scale-to");
        let long: u32 = args[1].parse().unwrap();
        assert_eq!(long, 200);
        assert!(sized);

        let g = Geometry {
            max: None,
            resize: Some(crate::geom::ResizeSpec::Width(400)),
        };
        let (args, sized) = pdftoppm_fit(&g, letter);
        assert_eq!(args[0], "-scale-to");
        let long: u32 = args[1].parse().unwrap();
        assert!(long >= 400);
        assert!(sized);

        let g = Geometry {
            max: Some(1920),
            resize: None,
        };
        let (args, sized) = pdftoppm_fit(&g, None);
        assert_eq!(args, ["-scale-to", "1920"]);
        assert!(sized);
    }

    #[test]
    fn pdf_to_png_respects_max() {
        if !deps::is_available("pdftoppm") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.png");
        let g = Geometry {
            max: Some(200),
            resize: None,
        };
        convert_pdf(&input, &out, "png", &opts(&g, true))
            .unwrap_or_else(|e| panic!("pdf→png --max: {e}"));
        let decoded = ::image::open(&out).unwrap();
        assert!(decoded.width() <= 200);
        assert!(decoded.height() <= 200);
        assert!(decoded.width() >= 50);
    }

    #[test]
    fn pdf_to_png_respects_resize_width() {
        if !deps::is_available("pdftoppm") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.png");
        let g = Geometry {
            max: None,
            resize: Some(crate::geom::ResizeSpec::Width(400)),
        };
        convert_pdf(&input, &out, "png", &opts(&g, true))
            .unwrap_or_else(|e| panic!("pdf→png --resize: {e}"));
        let decoded = ::image::open(&out).unwrap();
        // pdftoppm -scale-to long-side can be ±1–2px off the exact width.
        let w = decoded.width();
        assert!(
            (390..=410).contains(&w),
            "expected width ~400, got {w}×{}",
            decoded.height()
        );
        assert!(decoded.height() >= 400);
    }

    #[test]
    fn pdf_to_txt_extracts_text() {
        if !deps::is_available("pdftotext") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.txt");
        let g = Geometry::default();
        convert_pdf(&input, &out, "txt", &opts(&g, true))
            .unwrap_or_else(|e| panic!("pdf→txt: {e}"));
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(
            text.contains("Hello"),
            "expected extracted text to contain Hello, got {text:?}"
        );
    }

    fn pandoc_writes_docx() -> bool {
        if !deps::is_available("pandoc") {
            return false;
        }
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return false,
        };
        let txt = dir.path().join("a.txt");
        let out = dir.path().join("a.docx");
        if std::fs::write(&txt, "hi").is_err() {
            return false;
        }
        Command::new("pandoc")
            .args(["-f", "plain", "-t", "docx", "-o"])
            .arg(&out)
            .arg(&txt)
            .status()
            .map(|s| s.success() && out.is_file())
            .unwrap_or(false)
    }

    #[test]
    fn pdf_to_docx_via_text_or_soffice() {
        let has_soffice = deps::soffice_bin().is_some();
        if !has_soffice && !pandoc_writes_docx() {
            return;
        }
        if !has_soffice && !deps::is_available("pdftotext") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.docx");
        let g = Geometry::default();
        convert_pdf(&input, &out, "docx", &opts(&g, true))
            .unwrap_or_else(|e| panic!("pdf→docx: {e}"));
        assert!(out.exists());
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"PK"), "docx should be a zip package");
        assert!(bytes.len() > 32);
    }

    #[test]
    fn convert_file_pdf_to_pptx_is_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.pptx");
        let g = Geometry::default();
        let err = crate::convert::convert_file(&input, &out, "pptx", &opts(&g, true)).unwrap_err();
        match err {
            TummlerError::UnsupportedConversion(src, dst) => {
                assert_eq!(normalize_ext(&src), "pdf");
                assert_eq!(dst, "pptx");
            }
            other => panic!("expected UnsupportedConversion, got {other}"),
        }
    }

    #[test]
    fn convert_file_pdf_to_png() {
        if !deps::is_available("pdftoppm") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.png");
        let g = Geometry::default();
        crate::convert::convert_file(&input, &out, "png", &opts(&g, true))
            .unwrap_or_else(|e| panic!("convert_file pdf→png: {e}"));
        let decoded = ::image::open(&out).unwrap();
        assert!(decoded.width() >= 100);
    }

    #[test]
    fn pdf_to_png_missing_poppler_is_a_hard_error() {
        if deps::is_available("pdftoppm") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.png");
        let g = Geometry::default();
        let err = convert_pdf(&input, &out, "png", &opts(&g, true)).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("pdftoppm") || msg.contains("poppler"),
            "unexpected error: {msg}"
        );
        assert!(!msg.contains("Run `"));
    }
}
