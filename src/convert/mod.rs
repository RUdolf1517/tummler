pub mod archive;
pub mod document;
pub mod image;
pub mod media;

use crate::error::TummlerError;
use crate::util::{classify_ext, get_extension, normalize_ext, staging_path, FormatKind};
use colored::Colorize;
use std::path::Path;

/// Главный диспетчер конвертации.
/// Пишет во временный файл рядом с целью и атомарно переименовывает,
/// чтобы при ошибке не затереть уже существующий выход.
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
            "Файл уже существует: {} (используйте --overwrite / --force)",
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
            _ => image::convert_image(input, &staging, quality),
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
                    TummlerError::IoError(format!("не удалось сохранить {}: {e}; {e2}", output.display()))
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
