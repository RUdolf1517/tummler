pub mod archive;
pub mod document;
pub mod image;
pub mod media;
pub mod soffice;
pub mod spreadsheet;

use crate::deps;
use crate::error::TummlerError;
use crate::geom::Geometry;
use crate::util::{
    classify_ext, get_extension, is_heif, is_stdio, normalize_ext, staging_path, FormatKind,
};
use colored::Colorize;
use std::path::Path;

pub struct ConvertOpts<'a> {
    pub quality: Option<u8>,
    pub overwrite: bool,
    pub ffmpeg_args: Option<&'a str>,
    pub no_auto_install: bool,
    pub assume_yes: bool,
    pub verbose: bool,
    pub geometry: &'a Geometry,
    pub sheet: Option<&'a str>,
    /// ffmpeg `-threads`. 0 = let ffmpeg decide (single job).
    pub ffmpeg_threads: usize,
}

/// Check external dependencies once before a parallel batch.
pub fn prefetch_deps(
    inputs: &[std::path::PathBuf],
    target_ext: &str,
    quality: Option<u8>,
    no_auto: bool,
    assume_yes: bool,
) -> Result<(), TummlerError> {
    let target_kind = classify_ext(target_ext);
    let target = normalize_ext(target_ext);
    // Native webp is lossless only; `-q` goes through ffmpeg (libwebp).
    let lossy_webp = target == "webp" && quality.is_some();
    let mut need_ffmpeg =
        target_kind == FormatKind::Media || is_heif(target_ext) || lossy_webp;
    let need_pandoc = target_kind == FormatKind::Document
        && inputs.iter().any(|p| {
            let ext = get_extension(p).unwrap_or_default();
            matches!(
                classify_ext(&ext),
                FormatKind::Document | FormatKind::Unknown
            ) && !soffice::needs_soffice(&ext, target_ext)
                && !soffice::is_ooxml_copy(&ext, target_ext)
        });
    let need_soffice = inputs
        .iter()
        .any(|p| soffice::needs_soffice(&get_extension(p).unwrap_or_default(), target_ext));

    if target_kind == FormatKind::Image {
        need_ffmpeg = need_ffmpeg
            || inputs.iter().any(|p| {
                is_stdio(p)
                    || is_heif(&get_extension(p).unwrap_or_default())
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
    if need_soffice {
        deps::require_soffice()
            .map_err(|e| TummlerError::DependencyMissing("soffice".into(), e))?;
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
    opts: &ConvertOpts<'_>,
) -> Result<(), TummlerError> {
    let target_ext = normalize_ext(target_ext);
    let target_kind = classify_ext(&target_ext);

    let source_ext = get_extension(input).unwrap_or_default();
    let source_kind = if source_ext.is_empty() {
        FormatKind::Unknown
    } else {
        classify_ext(&source_ext)
    };

    if opts.verbose {
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

    if output.exists() && !opts.overwrite {
        return Err(TummlerError::Other(format!(
            "file already exists: {} (use --overwrite / --force)",
            output.display()
        )));
    }

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|e| TummlerError::IoError(e.to_string()))?;
    }

    let staging = staging_path(output);
    if staging.exists() {
        let _ = std::fs::remove_file(&staging);
    }

    let via_ffmpeg = || {
        media::convert_media(
            input,
            &staging,
            opts.quality,
            opts.ffmpeg_args,
            opts.no_auto_install,
            opts.assume_yes,
            opts.geometry,
            opts.ffmpeg_threads,
        )
    };

    let result = if soffice::is_ooxml_copy(&source_ext, &target_ext) {
        soffice::copy_ooxml_deck(input, &staging)
    } else if soffice::needs_soffice(&source_ext, &target_ext) {
        soffice::convert_office(input, &staging, &target_ext)
    } else if normalize_ext(&source_ext) == "pdf" && target_ext == "pptx" {
        Err(TummlerError::UnsupportedConversion(
            source_ext,
            target_ext.clone(),
        ))
    } else if source_kind == FormatKind::Spreadsheet {
        if spreadsheet::is_tabular_ext(&target_ext) {
            spreadsheet::convert_spreadsheet(input, &staging, &target_ext, opts.sheet)
        } else {
            Err(TummlerError::UnsupportedConversion(
                source_ext,
                target_ext.clone(),
            ))
        }
    } else {
        match target_kind {
            FormatKind::Image => {
                if is_heif(&source_ext)
                    || is_heif(&target_ext)
                    || (target_ext == "webp" && opts.quality.is_some())
                {
                    via_ffmpeg()
                } else {
                    match source_kind {
                        FormatKind::Media | FormatKind::Document => via_ffmpeg(),
                        FormatKind::Archive | FormatKind::Spreadsheet => {
                            Err(TummlerError::UnsupportedConversion(
                                source_ext,
                                target_ext.clone(),
                            ))
                        }
                        FormatKind::Unknown => {
                            match image::convert_image(input, &staging, opts.quality, opts.geometry)
                            {
                                Ok(()) => Ok(()),
                                Err(_) => via_ffmpeg(),
                            }
                        }
                        FormatKind::Image => {
                            image::convert_image(input, &staging, opts.quality, opts.geometry)
                        }
                    }
                }
            }
            FormatKind::Media => via_ffmpeg(),
            FormatKind::Document => {
                document::convert_document(input, &staging, opts.no_auto_install, opts.assume_yes)
            }
            FormatKind::Archive => archive::convert_archive(input, &staging),
            FormatKind::Spreadsheet | FormatKind::Unknown => {
                Err(TummlerError::UnsupportedFormat(target_ext.clone()))
            }
        }
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
