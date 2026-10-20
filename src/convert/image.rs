use crate::error::TummlerError;
use crate::util::normalize_ext;
use std::path::Path;

/// Конвертация изображений через crate `image`
pub fn convert_image(input: &Path, output: &Path, quality: Option<u8>) -> Result<(), TummlerError> {
    let output_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
        .unwrap_or_default();

    // Для .tar.gz и т.п. — это не картинка, но сюда не должны попасть
    // Проверяем что output_ext — картинка
    let img = image::open(input).map_err(|e| TummlerError::ImageError(e.to_string()))?;

    // Определяем формат вывода
    // image crate определяет по расширению, но для контроля качества нужно явно
    match output_ext.as_str() {
        "jpg" | "jpeg" => {
            // JPEG с качеством
            let q = quality.unwrap_or(90).clamp(1, 100);
            // Сохраняем через JpegEncoder с качеством
            let file = std::fs::File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
            let mut writer = std::io::BufWriter::new(file);
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, q);
            encoder
                .encode_image(&img)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "png" => {
            // PNG — lossless, качество игнорируется (но можно использовать compression)
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "gif" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "bmp" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "ico" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "tiff" | "tif" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "webp" => {
            // WebP — image crate поддерживает через webp feature
            // Качество для WebP тоже можно, но image 0.25 не дает прямого контроля качества WebP через save
            // Используем save, качество игнорируется в пользу дефолта (можно улучшить кастомным энкодером)
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "avif" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "pnm" | "pbm" | "pgm" | "ppm" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "tga" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "hdr" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "qoi" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "farbfeld" | "ff" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "dds" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        "exr" | "openexr" => {
            img.save(output)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        _ => {
            // Попытка сохранить — image crate может угадать по расширению
            img.save(output)
                .map_err(|e| TummlerError::ImageError(format!("Неподдерживаемый формат вывода {output_ext}: {e}")))?;
        }
    }

    Ok(())
}
