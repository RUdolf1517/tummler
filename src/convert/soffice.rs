use crate::deps;
use crate::error::TummlerError;
use crate::interrupt;
use crate::util::normalize_ext;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Impress formats that need LibreOffice (no native converter).
pub fn is_lo_slide(ext: &str) -> bool {
    matches!(normalize_ext(ext).as_str(), "odp" | "ppt" | "pps")
}

fn is_ooxml_slide(ext: &str) -> bool {
    matches!(normalize_ext(ext).as_str(), "pptx" | "ppsx")
}

fn is_impress(ext: &str) -> bool {
    is_lo_slide(ext) || is_ooxml_slide(ext)
}

/// Use already-installed soffice: `.odp`/`.ppt`/`.pps` ↔ pptx/pdf/odp.
/// Never `pptx/ppsx → pdf` (native) or `pptx ↔ ppsx` (copy).
pub fn needs_soffice(src: &str, dst: &str) -> bool {
    let src = normalize_ext(src);
    let dst = normalize_ext(dst);
    if is_lo_slide(&src) {
        return is_impress(&dst) || dst == "pdf";
    }
    is_ooxml_slide(&src) && is_lo_slide(&dst)
}

pub fn convert_impress(
    input: &Path,
    output: &Path,
    target_ext: &str,
) -> Result<(), TummlerError> {
    let dst = normalize_ext(target_ext);
    let Some(filter) = lo_filter(&dst) else {
        return Err(TummlerError::UnsupportedConversion(
            crate::util::get_extension(input).unwrap_or_default(),
            dst,
        ));
    };
    let soffice = deps::require_soffice().map_err(|e| {
        TummlerError::DependencyMissing("soffice".into(), e)
    })?;

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
    .arg(format!("-env:UserInstallation={}", file_url(&profile)))
    .args(["--convert-to", filter, "--outdir"])
    .arg(&outdir)
    .arg(input);

    let result = interrupt::run(&mut cmd).map_err(|e| {
        TummlerError::Other(format!("LibreOffice failed to start: {e}"))
    })?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(TummlerError::Other(format!(
            "LibreOffice failed (exit {:?}):\n{stderr}\n{stdout}",
            result.status.code()
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
        _ => None,
    }
}

fn file_url(path: &Path) -> String {
    let abs = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
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
        assert!(!needs_soffice("pptx", "pdf"));
        assert!(!needs_soffice("pptx", "ppsx"));
        assert!(!needs_soffice("odt", "pptx"));
        assert!(!needs_soffice("pdf", "pptx"));
        assert!(!needs_soffice("pptx", "pptx"));
    }

    #[test]
    fn missing_soffice_is_a_hard_error_not_an_install() {
        if deps::soffice_bin().is_some() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("deck.odp");
        std::fs::write(&input, b"not a real odp").unwrap();
        let out = dir.path().join("deck.pptx");
        let err = convert_impress(&input, &out, "pptx").unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("libreoffice") || msg.contains("soffice"));
        assert!(!msg.contains("Run `"));
        assert!(!msg.contains("brew install"));
    }
}
