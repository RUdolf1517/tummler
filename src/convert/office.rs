use crate::error::TummlerError;
use crate::util::{get_extension, normalize_ext};
use std::path::Path;

/// OOXML decks we convert natively (pptx/ppsx → pdf via office2pdf).
pub fn is_slide_ext(ext: &str) -> bool {
    matches!(normalize_ext(ext).as_str(), "pptx" | "ppsx")
}

/// `pptx/ppsx → pdf`, or `pptx ↔ ppsx` (same OPC package).
pub fn is_office_pair(src_ext: &str, dst_ext: &str) -> bool {
    let src = normalize_ext(src_ext);
    let dst = normalize_ext(dst_ext);
    if is_slide_ext(&src) && dst == "pdf" {
        return true;
    }
    is_slide_ext(&src) && is_slide_ext(&dst) && src != dst
}

/// pptx/ppsx → pdf (office2pdf / Typst). pptx ↔ ppsx is a copy.
/// `_no_auto_install` / `_assume_yes` kept so the call site stays the same; nothing to install.
pub fn convert_office(
    input: &Path,
    output: &Path,
    target_ext: &str,
    _no_auto_install: bool,
    _assume_yes: bool,
) -> Result<(), TummlerError> {
    let src = normalize_ext(&get_extension(input).unwrap_or_default());
    let dst = normalize_ext(target_ext);
    if is_slide_ext(&src) && dst == "pdf" {
        pptx_to_pdf(input, output)
    } else if is_slide_ext(&src) && is_slide_ext(&dst) && src != dst {
        std::fs::copy(input, output).map_err(|e| {
            TummlerError::IoError(format!("failed to write {}: {e}", output.display()))
        })?;
        Ok(())
    } else {
        Err(TummlerError::UnsupportedConversion(src, dst))
    }
}

fn pptx_to_pdf(input: &Path, output: &Path) -> Result<(), TummlerError> {
    let src = normalize_ext(&get_extension(input).unwrap_or_default());
    if !is_slide_ext(&src) {
        return Err(TummlerError::OfficeError(format!(
            "native PDF export supports .pptx/.ppsx (not .{src})"
        )));
    }
    let bytes = std::fs::read(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let result = office2pdf::convert_bytes(
        &bytes,
        office2pdf::config::Format::Pptx,
        &office2pdf::config::ConvertOptions::default(),
    )
    .map_err(|e| TummlerError::OfficeError(e.to_string()))?;
    if result.pdf.is_empty() {
        return Err(TummlerError::OfficeError(
            "office2pdf produced an empty PDF".into(),
        ));
    }
    std::fs::write(output, result.pdf).map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_exts() {
        assert!(is_slide_ext("PPTX"));
        assert!(is_slide_ext("ppsx"));
        assert!(!is_slide_ext("ppt"));
        assert!(!is_slide_ext("odp"));
        assert!(!is_slide_ext("odt"));
        assert!(!is_slide_ext("pdf"));
    }

    #[test]
    fn pairs() {
        assert!(is_office_pair("pptx", "pdf"));
        assert!(is_office_pair("ppsx", "pdf"));
        assert!(is_office_pair("pptx", "ppsx"));
        assert!(is_office_pair("ppsx", "pptx"));
        assert!(!is_office_pair("pptx", "pptx"));
        assert!(!is_office_pair("pdf", "pptx"));
        assert!(!is_office_pair("PPT", "PDF"));
        assert!(!is_office_pair("odp", "pptx"));
        assert!(!is_office_pair("odt", "pptx"));
        assert!(!is_office_pair("pptx", "txt"));
    }

    #[test]
    fn pptx_ppsx_is_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("deck.pptx");
        let output = dir.path().join("deck.ppsx");
        std::fs::write(&input, b"PK fake-ooxml").unwrap();
        convert_office(&input, &output, "ppsx", true, true).unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"PK fake-ooxml");
    }
}
