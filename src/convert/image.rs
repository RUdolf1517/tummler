use crate::error::TummlerError;
use crate::geom::Geometry;
use crate::util::normalize_ext;
use image::ImageDecoder;
use std::io::{BufWriter, Write};
use std::path::Path;

const BUF: usize = 256 * 1024;

fn map_img(e: impl ToString) -> TummlerError {
    TummlerError::ImageError(e.to_string())
}

fn map_io(e: impl ToString) -> TummlerError {
    TummlerError::IoError(e.to_string())
}

fn load_image(input: &Path) -> Result<image::DynamicImage, TummlerError> {
    let reader = image::ImageReader::open(input)
        .map_err(map_img)?
        .with_guessed_format()
        .map_err(map_img)?;
    let mut decoder = reader.into_decoder().map_err(map_img)?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder).map_err(map_img)?;
    if orientation != image::metadata::Orientation::NoTransforms {
        img.apply_orientation(orientation);
    }
    Ok(img)
}

fn into_rgb8(img: image::DynamicImage) -> image::RgbImage {
    match img {
        image::DynamicImage::ImageRgb8(buf) => buf,
        other => other.to_rgb8(),
    }
}

/// Image conversion via the `image` crate
pub fn convert_image(
    input: &Path,
    output: &Path,
    quality: Option<u8>,
    geometry: &Geometry,
) -> Result<(), TummlerError> {
    let output_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
        .unwrap_or_default();

    let img = apply_geometry(load_image(input)?, geometry);

    match output_ext.as_str() {
        "jpg" | "jpeg" => {
            let q = quality.unwrap_or(90).clamp(1, 100);
            let file = std::fs::File::create(output).map_err(map_io)?;
            let mut writer = BufWriter::with_capacity(BUF, file);
            let rgb = into_rgb8(img);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, q);
            rgb.write_with_encoder(encoder).map_err(map_img)?;
            writer.flush().map_err(map_io)?;
        }
        "avif" => {
            let q = quality.unwrap_or(80).clamp(1, 100);
            let file = std::fs::File::create(output).map_err(map_io)?;
            let mut writer = BufWriter::with_capacity(BUF, file);
            let rgba = img.to_rgba8();
            // speed 6: rav1e default 4 is slow; 1 thread so a `-j` batch doesn't oversubscribe.
            let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut writer, 6, q)
                .with_num_threads(Some(1));
            rgba.write_with_encoder(encoder).map_err(map_img)?;
            writer.flush().map_err(map_io)?;
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

fn apply_geometry(img: image::DynamicImage, geometry: &Geometry) -> image::DynamicImage {
    if geometry.is_noop() {
        return img;
    }
    let (w, h) = (img.width(), img.height());
    let (nw, nh) = geometry.target_size(w, h);
    if nw == w && nh == h {
        img
    } else {
        img.resize(nw, nh, image::imageops::FilterType::Lanczos3)
    }
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
        convert_image(&input, &output, Some(80), &Geometry::default()).unwrap();
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
        convert_image(&input, &output, None, &Geometry::default()).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 4);
    }

    #[test]
    fn resize_half() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        let output = dir.path().join("a.bmp");
        image::RgbImage::new(8, 8).save(&input).unwrap();
        let g = Geometry {
            max: None,
            resize: Some(crate::geom::ResizeSpec::Percent(50.0)),
        };
        convert_image(&input, &output, None, &g).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 4);
        assert_eq!(decoded.height(), 4);
    }
}
