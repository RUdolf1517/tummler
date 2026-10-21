use crate::deps;
use crate::error::TummlerError;
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
    let raster = raster_first_page(input, tmp.path(), opts)?;
    if target == "png" && opts.geometry.is_noop() {
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
            opts.geometry,
            opts.ffmpeg_threads,
        );
    }
    image::convert_image(&raster, output, opts.quality, opts.geometry)
}

fn raster_first_page(
    input: &Path,
    tmp: &Path,
    opts: &ConvertOpts<'_>,
) -> Result<PathBuf, TummlerError> {
    deps::ensure_available("pdftoppm", opts.no_auto_install, opts.assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("pdftoppm".into(), e))?;

    let prefix = tmp.join("page");
    let mut cmd = Command::new("pdftoppm");
    cmd.args(["-png", "-f", "1", "-l", "1", "-singlefile", "-r", "150"])
        .arg(input)
        .arg(&prefix);

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

    let produced = prefix.with_extension("png");
    if produced.is_file() {
        return Ok(produced);
    }
    for name in ["page-1.png", "page-01.png"] {
        let alt = tmp.join(name);
        if alt.is_file() {
            return Ok(alt);
        }
    }
    Err(TummlerError::PdfError(
        "pdftoppm produced no PNG for page 1".into(),
    ))
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
    cmd.args(["-layout", "-nopgbrk", "-enc", "UTF-8"])
        .arg(input)
        .arg(output);
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
    // Same binary as document conversion; keep the ensure in one place.
    deps::ensure_available("pandoc", opts.no_auto_install, opts.assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("pandoc".into(), e))?;

    let mut cmd = Command::new("pandoc");
    cmd.args(["-f", "plain"]).arg(input_txt).arg("-o").arg(output);
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

/// One-page PDF with Helvetica text. Used in tests.
#[cfg(test)]
pub(crate) fn sample_pdf(text: &str) -> Vec<u8> {
    let escaped = text
        .replace('\\', r"\\")
        .replace('(', r"\(")
        .replace(')', r"\)");
    let content = format!("BT /F1 24 Tf 72 720 Td ({escaped}) Tj ET\n");
    let mut body = Vec::new();
    body.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = vec![0u32];
    let mut push = |bytes: &[u8]| {
        offsets.push(body.len() as u32);
        body.extend_from_slice(bytes);
    };
    push(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    push(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    push(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");
    let stream = format!(
        "4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
        content.len()
    );
    push(stream.as_bytes());
    push(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref_at = body.len();
    let n = offsets.len();
    body.extend_from_slice(format!("xref\n0 {n}\n").as_bytes());
    body.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        body.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    body.extend_from_slice(
        format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes(),
    );
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Geometry;

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
        convert_pdf(&input, &out, "png", &opts(&g, true)).unwrap();
        assert!(out.exists());
        let decoded = ::image::open(&out).unwrap();
        assert!(decoded.width() >= 100);
        assert!(decoded.height() >= 100);
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
        convert_pdf(&input, &out, "png", &opts(&g, true)).unwrap();
        let decoded = ::image::open(&out).unwrap();
        assert!(decoded.width() <= 200);
        assert!(decoded.height() <= 200);
        assert!(decoded.width() >= 50);
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
        convert_pdf(&input, &out, "txt", &opts(&g, true)).unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(
            text.contains("Hello"),
            "expected extracted text to contain Hello, got {text:?}"
        );
    }

    #[test]
    fn pdf_to_docx_via_text_or_soffice() {
        let has_soffice = deps::soffice_bin().is_some();
        let has_text = deps::is_available("pdftotext") && deps::is_available("pandoc");
        if !has_soffice && !has_text {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = write_sample(dir.path());
        let out = dir.path().join("hello.docx");
        let g = Geometry::default();
        convert_pdf(&input, &out, "docx", &opts(&g, true)).unwrap();
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
        crate::convert::convert_file(&input, &out, "png", &opts(&g, true)).unwrap();
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
