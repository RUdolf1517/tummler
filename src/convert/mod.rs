pub mod archive;
pub mod document;
pub mod image;
pub mod media;

use crate::error::TummlerError;
use crate::util::{classify_ext, normalize_ext, FormatKind, get_extension};
use std::path::Path;

/// Главный диспетчер конвертации
/// Определяет тип исходника и цели, роутит в нужный модуль
pub fn convert_file(
    input: &Path,
    output: &Path,
    target_ext: &str,
    quality: Option<u8>,
    overwrite: bool,
    ffmpeg_args: Option<&str>,
    no_auto_install: bool,
    verbose: bool,
) -> Result<(), TummlerError> {
    let target_ext = normalize_ext(target_ext);
    let target_kind = classify_ext(&target_ext);

    // Определяем исходный формат (по расширению)
    // Для картинок — можем также попробовать открыть файл, но пока по расширению
    let source_ext = get_extension(input).unwrap_or_default();
    let source_kind = if source_ext.is_empty() {
        // Пытаемся угадать по mime или содержимому
        // Для простоты — если не удалось, считаем что исходник может быть любым,
        // главное что целевой формат известен
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

    // Проверка overwrite
    if output.exists() && !overwrite {
        return Err(TummlerError::Other(format!(
            "Файл уже существует: {} (используйте --overwrite)",
            output.display()
        )));
    }

    // Создаем родительские директории
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| TummlerError::IoError(e.to_string()))?;
        }
    }

    // Роутинг по целевому формату (важнее всего куда конвертируем)
    // Но также учитываем что некоторые кросс-типы невозможны
    match target_kind {
        FormatKind::Image => {
            // Картинка → картинка: нативно
            // Видео → картинка (извлечь кадр): через ffmpeg — тоже поддержим
            if source_kind == FormatKind::Media {
                // извлекаем кадр через ffmpeg
                media::convert_media(input, output, quality, ffmpeg_args, no_auto_install)
            } else {
                image::convert_image(input, output, quality)
            }
        }
        FormatKind::Media => {
            media::convert_media(input, output, quality, ffmpeg_args, no_auto_install)
        }
        FormatKind::Document => {
            document::convert_document(input, output, no_auto_install)
        }
        FormatKind::Archive => {
            archive::convert_archive(input, output)
        }
        FormatKind::Unknown => {
            Err(TummlerError::UnsupportedFormat(target_ext.clone()))
        }
    }
}

// Для colored dimmed в verbose
use colored::Colorize;
