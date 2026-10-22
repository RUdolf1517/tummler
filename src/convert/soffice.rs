use crate::deps;
use crate::error::TummlerError;
use crate::interrupt;
use crate::util::normalize_ext;
use std::path::{Path, PathBuf};
use std::process::Command;

fn is_lo_slide(ext: &str) -> bool {
    matches!(ext, "odp" | "ppt" | "pps")
}

fn is_ooxml_slide(ext: &str) -> bool {
    matches!(ext, "pptx" | "ppsx")
}

fn is_impress(ext: &str) -> bool {
    is_lo_slide(ext) || is_ooxml_slide(ext)
}

fn is_writer(ext: &str) -> bool {
    matches!(ext, "odt" | "docx" | "doc" | "rtf")
}

/// `pptx` ↔ `ppsx`: same OOXML package, no LibreOffice.
pub fn is_ooxml_copy(src: &str, dst: &str) -> bool {
    let src = normalize_ext(src);
    let dst = normalize_ext(dst);
    is_ooxml_slide(&src) && is_ooxml_slide(&dst) && src != dst
}

pub fn copy_ooxml_deck(input: &Path, output: &Path) -> Result<(), TummlerError> {
    std::fs::copy(input, output).map_err(|e| {
        TummlerError::IoError(format!("failed to write {}: {e}", output.display()))
    })?;
    Ok(())
}

/// Already-installed soffice: Impress ↔ pdf / pptx / odp / ppt.
/// Never auto-installed. `pptx` ↔ `ppsx` is a copy, not this.
pub fn needs_soffice(src: &str, dst: &str) -> bool {
    let src = normalize_ext(src);
    let dst = normalize_ext(dst);
    if src == dst || is_ooxml_copy(&src, &dst) {
        return false;
    }
    is_impress(&src) && (is_impress(&dst) || dst == "pdf")
}

/// Writer → pdf, or PDF → docx/odt/rtf: soffice if already on PATH.
pub fn prefers_soffice(src: &str, dst: &str) -> bool {
    let src = normalize_ext(src);
    let dst = normalize_ext(dst);
    (is_writer(&src) && dst == "pdf")
        || (src == "pdf" && matches!(dst.as_str(), "docx" | "odt" | "rtf"))
}

pub fn convert_office(
    input: &Path,
    output: &Path,
    target_ext: &str,
) -> Result<(), TummlerError> {
    convert_office_filter(input, output, target_ext, None)
}

/// PDF → Writer via `--infilter=writer_pdf_import` (layout). Never auto-installs soffice.
pub fn convert_pdf_import(
    input: &Path,
    output: &Path,
    target_ext: &str,
) -> Result<(), TummlerError> {
    convert_office_filter(input, output, target_ext, Some("writer_pdf_import"))
}

fn convert_office_filter(
    input: &Path,
    output: &Path,
    target_ext: &str,
    infilter: Option<&str>,
) -> Result<(), TummlerError> {
    let dst = normalize_ext(target_ext);
    let Some(filter) = lo_filter(&dst) else {
        return Err(TummlerError::UnsupportedConversion(
            crate::util::get_extension(input).unwrap_or_default(),
            dst,
        ));
    };
    let soffice = deps::require_soffice()
        .map_err(|e| TummlerError::DependencyMissing("soffice".into(), e))?;

    let tmp = tempfile::tempdir().map_err(|e| TummlerError::IoError(e.to_string()))?;
    let profile = tmp.path().join("lo-profile");
    let outdir = tmp.path().join("out");
    std::fs::create_dir_all(&profile).map_err(|e| TummlerError::IoError(e.to_string()))?;
    std::fs::create_dir_all(&outdir).map_err(|e| TummlerError::IoError(e.to_string()))?;

    let mut cmd = Command::new(&soffice);
    cmd.args([
        "--headless",
        "--norestore",
        "--nolockcheck",
        "--nofirststartwizard",
    ])
    .arg(format!("-env:UserInstallation={}", file_url(&profile)));
    if let Some(f) = infilter {
        cmd.arg(format!("--infilter={f}"));
    }
    cmd.args(["--convert-to", filter, "--outdir"])
        .arg(&outdir)
        .arg(input);

    let result = interrupt::run(&mut cmd)?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(TummlerError::Other(crate::hint::format_engine_fail(
            "LibreOffice",
            result.status.code(),
            &stderr,
            &stdout,
        )));
    }

    let produced = take_lo_output(&outdir, &dst)?;
    std::fs::copy(&produced, output).map_err(|e| {
        TummlerError::IoError(format!("failed to write {}: {e}", output.display()))
    })?;
    Ok(())
}

fn lo_filter(ext: &str) -> Option<&'static str> {
    match ext {
        "pptx" => Some("pptx"),
        "ppsx" => Some("ppsx"),
        "ppt" => Some("ppt"),
        "pps" => Some("pps"),
        "odp" => Some("odp"),
        "pdf" => Some("pdf"),
        "docx" => Some("docx"),
        "odt" => Some("odt"),
        "rtf" => Some("rtf"),
        _ => None,
    }
}

fn file_url(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy();
    let mut url = String::from("file://");
    for c in s.chars() {
        match c {
            ' ' => url.push_str("%20"),
            '%' => url.push_str("%25"),
            _ => url.push(c),
        }
    }
    url
}

fn take_lo_output(outdir: &Path, target_ext: &str) -> Result<PathBuf, TummlerError> {
    let mut found = Vec::new();
    let rd = std::fs::read_dir(outdir).map_err(|e| TummlerError::IoError(e.to_string()))?;
    for ent in rd {
        let p = ent.map_err(|e| TummlerError::IoError(e.to_string()))?.path();
        if p.extension()
            .and_then(|e| e.to_str())
            .map(normalize_ext)
            .as_deref()
            == Some(target_ext)
        {
            found.push(p);
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(TummlerError::Other(
            "LibreOffice produced no output file".into(),
        )),
        _ => Err(TummlerError::Other(
            "LibreOffice produced multiple output files".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soffice_pairs() {
        assert!(needs_soffice("odp", "pptx"));
        assert!(needs_soffice("ppt", "pptx"));
        assert!(needs_soffice("pps", "pdf"));
        assert!(needs_soffice("pptx", "odp"));
        assert!(needs_soffice("odp", "pdf"));
        assert!(needs_soffice("pptx", "pdf"));
        assert!(needs_soffice("ppsx", "pdf"));
        assert!(!needs_soffice("docx", "pdf"));
        assert!(!needs_soffice("odt", "pdf"));
        assert!(!needs_soffice("pptx", "ppsx"));
        assert!(!needs_soffice("odt", "pptx"));
        assert!(!needs_soffice("docx", "odt"));
        assert!(!needs_soffice("md", "pdf"));
        assert!(!needs_soffice("pdf", "pptx"));
        assert!(!needs_soffice("pdf", "docx"));
        assert!(!needs_soffice("pptx", "pptx"));
        assert!(prefers_soffice("docx", "pdf"));
        assert!(prefers_soffice("odt", "pdf"));
        assert!(prefers_soffice("doc", "pdf"));
        assert!(prefers_soffice("rtf", "pdf"));
        assert!(prefers_soffice("pdf", "docx"));
        assert!(prefers_soffice("pdf", "odt"));
        assert!(prefers_soffice("pdf", "rtf"));
        assert!(!prefers_soffice("pdf", "pptx"));
        assert!(!prefers_soffice("pdf", "png"));
        assert!(!prefers_soffice("odt", "pptx"));
        assert!(!prefers_soffice("md", "pdf"));
    }

    #[test]
    fn pptx_ppsx_is_a_copy() {
        assert!(is_ooxml_copy("pptx", "ppsx"));
        assert!(is_ooxml_copy("ppsx", "pptx"));
        assert!(!is_ooxml_copy("pptx", "pptx"));
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("deck.pptx");
        let output = dir.path().join("deck.ppsx");
        std::fs::write(&input, b"PK fake-ooxml").unwrap();
        copy_ooxml_deck(&input, &output).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"PK fake-ooxml");
    }

    #[test]
    fn missing_soffice_is_a_hard_error_not_an_install() {
        if deps::soffice_bin().is_some() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("deck.pptx");
        std::fs::write(&input, b"not a real pptx").unwrap();
        let out = dir.path().join("deck.pdf");
        let err = convert_office(&input, &out, "pdf").unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("libreoffice") || msg.contains("soffice"));
        assert!(!msg.contains("Run `"));
        assert!(!msg.contains("brew install"));
    }
}
