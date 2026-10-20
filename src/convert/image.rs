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

    let img = image::open(input).map_err(|e| TummlerError::ImageError(e.to_string()))?;

    match output_ext.as_str() {
        "jpg" | "jpeg" => {
            let q = quality.unwrap_or(90).clamp(1, 100);
            let file = std::fs::File::create(output)
                .map_err(|e| TummlerError::IoError(e.to_string()))?;
            let writer = std::io::BufWriter::new(file);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, q);
            img.write_with_encoder(encoder)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
        }
        _ => {
            // PNG/GIF/WebP/AVIF и остальные — image crate определяет формат по расширению.
            // WebP в image 0.25 — lossless, `-q` на него не влияет.
            img.save(output)
                .map_err(|e| TummlerError::ImageError(format!("Не удалось сохранить {output_ext}: {e}")))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_to_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        let output = dir.path().join("a.jpeg");
        image::RgbImage::new(8, 8).save(&input).unwrap();
        convert_image(&input, &output, Some(80)).unwrap();
        assert!(output.exists());
        assert!(output.metadata().unwrap().len() > 0);
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 8);
        assert_eq!(decoded.height(), 8);
    }

    #[test]
    fn png_to_bmp_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        let output = dir.path().join("a.bmp");
        image::RgbImage::new(4, 4).save(&input).unwrap();
        convert_image(&input, &output, None).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 4);
    }
}
