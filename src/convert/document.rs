use crate::deps;
use crate::error::TummlerError;
use std::path::Path;
use std::process::Command;

/// Конвертация документов через pandoc
pub fn convert_document(
    input: &Path,
    output: &Path,
    no_auto_install: bool,
) -> Result<(), TummlerError> {
    deps::ensure_available("pandoc", no_auto_install)
        .map_err(|e| TummlerError::DependencyMissing("pandoc".into(), e))?;

    let mut cmd = Command::new("pandoc");
    cmd.arg(input).arg("-o").arg(output);

    let result = cmd
        .output()
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
            "\nПодсказка: для PDF нужен LaTeX (например: brew install --cask mactex / sudo apt install texlive-latex-recommended texlive-fonts-recommended)"
        } else {
            ""
        };

        return Err(TummlerError::DocumentError(format!(
            "pandoc завершился с ошибкой (код {:?}):\n{}{}{}",
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
