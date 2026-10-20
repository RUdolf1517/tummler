pub mod archive;
pub mod document;
pub mod image;
pub mod media;

use crate::deps;
use crate::error::TummlerError;
use crate::util::{classify_ext, get_extension, is_stdio, normalize_ext, staging_path, FormatKind};
use colored::Colorize;
use std::path::Path;

/// Check external dependencies once before a parallel batch.
pub fn prefetch_deps(
    inputs: &[std::path::PathBuf],
    target_ext: &str,
    no_auto: bool,
    assume_yes: bool,
) -> Result<(), TummlerError> {
    let target_kind = classify_ext(target_ext);
    let mut need_ffmpeg = target_kind == FormatKind::Media;
    let need_pandoc = target_kind == FormatKind::Document;

    if target_kind == FormatKind::Image {
        need_ffmpeg = inputs.iter().any(|p| {
            is_stdio(p)
                || matches!(
                    classify_ext(&get_extension(p).unwrap_or_default()),
                    FormatKind::Media | FormatKind::Document
                )
        });
    }

    if need_ffmpeg {
        deps::ensure_available("ffmpeg", no_auto, assume_yes)
            .map_err(|e| TummlerError::DependencyMissing("ffmpeg".into(), e))?;
    }
    if need_pandoc {
        deps::ensure_available("pandoc", no_auto, assume_yes)
            .map_err(|e| TummlerError::DependencyMissing("pandoc".into(), e))?;
    }
    Ok(())
}

/// Main conversion dispatcher.
/// Writes to a temp file next to the target and renames atomically
/// so a failure cannot clobber an existing output.
pub fn convert_file(
    input: &Path,
    output: &Path,
    target_ext: &str,
    quality: Option<u8>,
    overwrite: bool,
    ffmpeg_args: Option<&str>,
    no_auto_install: bool,
    assume_yes: bool,
    verbose: bool,
) -> Result<(), TummlerError> {
    let target_ext = normalize_ext(target_ext);
    let target_kind = classify_ext(&target_ext);

    let source_ext = get_extension(input).unwrap_or_default();
    let source_kind = if source_ext.is_empty() {
        FormatKind::Unknown
    } else {
        classify_ext(&source_ext)
    };

    if verbose {
        eprintln!(
            "  {} {} ({}:{:?}) → {} ({}:{:?})",
            "→".dimmed(),
            input.display(),
            source_ext,
            source_kind,
            output.display(),
            target_ext,
            target_kind
        );
    }

    if output.exists() && !overwrite {
        return Err(TummlerError::Other(format!(
            "file already exists: {} (use --overwrite / --force)",
            output.display()
        )));
    }

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| TummlerError::IoError(e.to_string()))?;
        }
    }

    let staging = staging_path(output);
    if staging.exists() {
        let _ = std::fs::remove_file(&staging);
    }

    let result = match target_kind {
        FormatKind::Image => match source_kind {
            FormatKind::Media | FormatKind::Document => {
                media::convert_media(
                    input,
                    &staging,
                    quality,
                    ffmpeg_args,
                    no_auto_install,
                    assume_yes,
                )
            }
            FormatKind::Archive => Err(TummlerError::UnsupportedConversion(
                source_ext,
                target_ext.clone(),
            )),
            FormatKind::Unknown => match image::convert_image(input, &staging, quality) {
                Ok(()) => Ok(()),
                Err(_) => media::convert_media(
                    input,
                    &staging,
                    quality,
                    ffmpeg_args,
                    no_auto_install,
                    assume_yes,
                ),
            },
            FormatKind::Image => image::convert_image(input, &staging, quality),
        },
        FormatKind::Media => media::convert_media(
            input,
            &staging,
            quality,
            ffmpeg_args,
            no_auto_install,
            assume_yes,
        ),
        FormatKind::Document => {
            document::convert_document(input, &staging, no_auto_install, assume_yes)
        }
        FormatKind::Archive => archive::convert_archive(input, &staging),
        FormatKind::Unknown => Err(TummlerError::UnsupportedFormat(target_ext.clone())),
    };

    match result {
        Ok(()) => {
            if let Err(e) = std::fs::rename(&staging, output) {
                std::fs::copy(&staging, output).map_err(|e2| {
                    let _ = std::fs::remove_file(&staging);
                    TummlerError::IoError(format!("failed to save {}: {e}; {e2}", output.display()))
                })?;
                let _ = std::fs::remove_file(&staging);
            }
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&staging);
            Err(e)
        }
    }
}
