use crate::error::TummlerError;
use crate::geom::Geometry;
use crate::util::{get_extension, is_svg, normalize_ext};
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

fn load_svg(input: &Path, geometry: &Geometry) -> Result<image::DynamicImage, TummlerError> {
    let data = std::fs::read(input).map_err(map_io)?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default())
        .map_err(map_img)?;
    let size = tree.size();
    let ow = size.width().max(1.0);
    let oh = size.height().max(1.0);
    let (nw, nh) = geometry.target_size(ow.round().max(1.0) as u32, oh.round().max(1.0) as u32);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(nw, nh).ok_or_else(|| {
        TummlerError::ImageError("svg raster size is invalid".into())
    })?;
    let transform =
        resvg::tiny_skia::Transform::from_scale(nw as f32 / ow, nh as f32 / oh);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let rgba = image::RgbaImage::from_raw(nw, nh, pixmap.data().to_vec())
        .ok_or_else(|| TummlerError::ImageError("svg raster failed".into()))?;
    Ok(image::DynamicImage::ImageRgba8(rgba))
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

    let img = if is_svg(&get_extension(input).unwrap_or_default()) {
        load_svg(input, geometry)?
    } else {
        apply_geometry(load_image(input)?, geometry)
    };

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

    #[test]
    fn svg_to_png() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.svg");
        std::fs::write(
            &input,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#ff0000"/></svg>"#,
        )
        .unwrap();
        let output = dir.path().join("a.png");
        convert_image(&input, &output, None, &Geometry::default()).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 8);
        assert_eq!(decoded.height(), 8);
    }
}
