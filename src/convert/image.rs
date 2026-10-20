use crate::error::TummlerError;
use crate::util::normalize_ext;
use std::io::{BufWriter, Write};
use std::path::Path;

const BUF: usize = 256 * 1024;

/// Image conversion via the `image` crate
pub fn convert_image(input: &Path, output: &Path, quality: Option<u8>) -> Result<(), TummlerError> {
    let output_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
        .unwrap_or_default();

    let reader = image::ImageReader::open(input)
        .map_err(|e| TummlerError::ImageError(e.to_string()))?
        .with_guessed_format()
        .map_err(|e| TummlerError::ImageError(e.to_string()))?;
    let img = reader
        .decode()
        .map_err(|e| TummlerError::ImageError(e.to_string()))?;

    match output_ext.as_str() {
        "jpg" | "jpeg" => {
            let q = quality.unwrap_or(90).clamp(1, 100);
            let file = std::fs::File::create(output)
                .map_err(|e| TummlerError::IoError(e.to_string()))?;
            let mut writer = BufWriter::with_capacity(BUF, file);
            let rgb = img.to_rgb8();
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, q);
            rgb.write_with_encoder(encoder)
                .map_err(|e| TummlerError::ImageError(e.to_string()))?;
            writer.flush().map_err(|e| TummlerError::IoError(e.to_string()))?;
        }
        _ => {
            let fmt = image_format_from_ext(&output_ext).ok_or_else(|| {
                TummlerError::ImageError(format!("unsupported output format {output_ext}"))
            })?;
            img.save_with_format(output, fmt)
                .map_err(|e| TummlerError::ImageError(format!("failed to save {output_ext}: {e}")))?;
        }
    }

    Ok(())
}

fn image_format_from_ext(ext: &str) -> Option<image::ImageFormat> {
    Some(match ext {
        "png" => image::ImageFormat::Png,
        "gif" => image::ImageFormat::Gif,
        "bmp" => image::ImageFormat::Bmp,
        "ico" => image::ImageFormat::Ico,
        "tiff" | "tif" => image::ImageFormat::Tiff,
        "webp" => image::ImageFormat::WebP,
        "avif" => image::ImageFormat::Avif,
        "pnm" | "pbm" | "pgm" | "ppm" => image::ImageFormat::Pnm,
        "tga" => image::ImageFormat::Tga,
        "hdr" => image::ImageFormat::Hdr,
        "qoi" => image::ImageFormat::Qoi,
        "ff" | "farbfeld" => image::ImageFormat::Farbfeld,
        "dds" => image::ImageFormat::Dds,
        "exr" | "openexr" => image::ImageFormat::OpenExr,
        _ => return None,
    })
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
