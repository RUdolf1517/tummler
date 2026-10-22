use crate::deps;
use crate::error::TummlerError;
use crate::interrupt;
use std::path::Path;
use std::process::Command;

pub(crate) fn pandoc_strip_args(strip_meta: bool) -> &'static [&'static str] {
    if strip_meta {
        &["-M", "title=", "-M", "author=", "-M", "date="]
    } else {
        &[]
    }
}

/// Document conversion via pandoc
pub fn convert_document(
    input: &Path,
    output: &Path,
    no_auto_install: bool,
    assume_yes: bool,
    strip_meta: bool,
) -> Result<(), TummlerError> {
    deps::ensure_available("pandoc", no_auto_install, assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("pandoc".into(), e))?;

    let mut cmd = Command::new("pandoc");
    cmd.arg(input).arg("-o").arg(output);
    for a in pandoc_strip_args(strip_meta) {
        cmd.arg(a);
    }

    let result = interrupt::run(&mut cmd)
        .map_err(|e| TummlerError::DocumentError(e.to_string()))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);

        let out_ext = output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let hint = if out_ext == "pdf"
            && (stderr.contains("pdflatex") || stderr.contains("xelatex") || stderr.contains("lualatex"))
        {
            "\nhint: PDF needs LaTeX (e.g. brew install --cask mactex / sudo apt install texlive-latex-recommended texlive-fonts-recommended)"
        } else {
            ""
        };

        return Err(TummlerError::DocumentError(format!(
            "pandoc failed (exit {:?}):\n{}{}{}",
            result.status.code(),
            stderr,
            hint,
            if stdout.is_empty() {
                String::new()
            } else {
                format!("\n{stdout}")
            }
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandoc_strip_args_empty_title_author_date() {
        assert!(pandoc_strip_args(false).is_empty());
        assert_eq!(
            pandoc_strip_args(true),
            ["-M", "title=", "-M", "author=", "-M", "date="].as_slice()
        );
    }
}
