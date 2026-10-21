use crate::error::TummlerError;
use crate::geom::Geometry;
use crate::util::{get_extension, is_svg, normalize_ext};
use image::ImageDecoder;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;
use std::sync::OnceLock;

const BUF: usize = 256 * 1024;

fn map_img(e: impl ToString) -> TummlerError {
    TummlerError::ImageError(e.to_string())
}

fn map_io(e: impl ToString) -> TummlerError {
    TummlerError::IoError(e.to_string())
}

fn is_jpeg_input(path: &Path) -> bool {
    get_extension(path)
        .map(|e| normalize_ext(&e))
        .is_some_and(|e| e == "jpg")
}

fn orientation_swaps_axes(o: image::metadata::Orientation) -> bool {
    use image::metadata::Orientation::*;
    matches!(
        o,
        Rotate90 | Rotate270 | Rotate90FlipH | Rotate270FlipH
    )
}

fn apply_orientation(
    mut img: image::DynamicImage,
    orientation: image::metadata::Orientation,
) -> image::DynamicImage {
    if orientation != image::metadata::Orientation::NoTransforms {
        img.apply_orientation(orientation);
    }
    img
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
    let img = image::DynamicImage::from_decoder(decoder).map_err(map_img)?;
    Ok(apply_orientation(img, orientation))
}

/// JPEG IDCT 1/2, 1/4, 1/8 when the target is at least 2× smaller on both coded axes.
/// `None` = fall back to a full decode (scale would not shrink, or unsupported JPEG).
fn load_jpeg_scaled(
    input: &Path,
    geometry: &Geometry,
) -> Result<Option<image::DynamicImage>, TummlerError> {
    let bytes = std::fs::read(input).map_err(map_io)?;
    let (orientation, cw, ch) = {
        let mut header = image::codecs::jpeg::JpegDecoder::new(Cursor::new(bytes.as_slice()))
            .map_err(map_img)?;
        let orientation = header
            .orientation()
            .unwrap_or(image::metadata::Orientation::NoTransforms);
        let (cw, ch) = header.dimensions();
        (orientation, cw, ch)
    };

    let swap = orientation_swaps_axes(orientation);
    let (ow, oh) = if swap { (ch, cw) } else { (cw, ch) };
    let (tw, th) = geometry.target_size(ow, oh);
    let (req_w, req_h) = if swap { (th, tw) } else { (tw, th) };
    // jpeg-decoder's reduced IDCT is 1/2, 1/4, 1/8. Skip unless both axes shrink ≥2×
    // so we never decode smaller than the (oriented) target on either axis.
    if req_w > cw / 2 || req_h > ch / 2 || req_w > u16::MAX as u32 || req_h > u16::MAX as u32 {
        return Ok(None);
    }

    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes.as_slice()));
    let (sw, sh) = decoder
        .scale(req_w as u16, req_h as u16)
        .map_err(map_img)?;
    if u32::from(sw) >= cw && u32::from(sh) >= ch {
        return Ok(None);
    }
    let pixels = decoder.decode().map_err(map_img)?;
    let info = decoder
        .info()
        .ok_or_else(|| TummlerError::ImageError("jpeg decode".into()))?;
    let img = jpeg_pixels_to_image(info, pixels)?;
    Ok(Some(apply_orientation(img, orientation)))
}

fn jpeg_pixels_to_image(
    info: jpeg_decoder::ImageInfo,
    pixels: Vec<u8>,
) -> Result<image::DynamicImage, TummlerError> {
    let w = u32::from(info.width);
    let h = u32::from(info.height);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            let buf = image::RgbImage::from_raw(w, h, pixels)
                .ok_or_else(|| TummlerError::ImageError("jpeg rgb size".into()))?;
            Ok(image::DynamicImage::ImageRgb8(buf))
        }
        jpeg_decoder::PixelFormat::L8 => {
            let buf = image::GrayImage::from_raw(w, h, pixels)
                .ok_or_else(|| TummlerError::ImageError("jpeg gray size".into()))?;
            Ok(image::DynamicImage::ImageLuma8(buf))
        }
        _ => Err(TummlerError::ImageError("jpeg scaled format".into())),
    }
}

fn into_rgb8(img: image::DynamicImage) -> image::RgbImage {
    match img {
        image::DynamicImage::ImageRgb8(buf) => buf,
        other => other.to_rgb8(),
    }
}

fn svg_options() -> &'static resvg::usvg::Options<'static> {
    static OPTS: OnceLock<resvg::usvg::Options<'static>> = OnceLock::new();
    OPTS.get_or_init(Default::default)
}

fn load_svg(input: &Path, geometry: &Geometry) -> Result<image::DynamicImage, TummlerError> {
    let data = std::fs::read(input).map_err(map_io)?;
    let tree = resvg::usvg::Tree::from_data(&data, svg_options()).map_err(map_img)?;
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
    let rgba = image::RgbaImage::from_raw(nw, nh, pixmap.take())
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
        let loaded = if !geometry.is_noop() && is_jpeg_input(input) {
            match load_jpeg_scaled(input, geometry) {
                Ok(Some(img)) => img,
                _ => load_image(input)?,
            }
        } else {
            load_image(input)?
        };
        apply_geometry(loaded, geometry)
    };

    match output_ext.as_str() {
        "jpg" => {
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
            let file = std::fs::File::create(output).map_err(map_io)?;
            let mut writer = BufWriter::with_capacity(BUF, file);
            img.write_to(&mut writer, fmt).map_err(map_img)?;
            writer.flush().map_err(map_io)?;
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
        // Triangle is much cheaper on the downscale hot path; Lanczos for upscale.
        let filter = if (nw as u64) * (nh as u64) < (w as u64) * (h as u64) {
            image::imageops::FilterType::Triangle
        } else {
            image::imageops::FilterType::Lanczos3
        };
        img.resize(nw, nh, filter)
    }
}

fn image_format_from_ext(ext: &str) -> Option<image::ImageFormat> {
    Some(match ext {
        "png" => image::ImageFormat::Png,
        "gif" => image::ImageFormat::Gif,
        "bmp" => image::ImageFormat::Bmp,
        "ico" => image::ImageFormat::Ico,
        "tiff" => image::ImageFormat::Tiff,
        "webp" => image::ImageFormat::WebP,
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
    #[test]
    fn jpeg_scaled_decode_shrinks() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.jpg");
        image::RgbImage::new(64, 48).save(&input).unwrap();
        let g = Geometry {
            max: Some(16),
            resize: None,
        };
        let img = load_jpeg_scaled(&input, &g)
            .unwrap()
            .expect("DCT scale should apply at 64×48 → 16");
        assert!(img.width() <= 32 && img.height() <= 24);
        assert!(img.width() >= 8 && img.height() >= 6);
    }

    #[test]
    fn jpeg_max_downscale() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.jpg");
        let output = dir.path().join("a.png");
        image::RgbImage::new(64, 48).save(&input).unwrap();
        let g = Geometry {
            max: Some(16),
            resize: None,
        };
        convert_image(&input, &output, None, &g).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 16);
        assert_eq!(decoded.height(), 12);
    }

    #[test]
    fn jpeg_max_no_upscale() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.jpg");
        let output = dir.path().join("a.png");
        image::RgbImage::new(8, 8).save(&input).unwrap();
        let g = Geometry {
            max: Some(1920),
            resize: None,
        };
        convert_image(&input, &output, None, &g).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 8);
        assert_eq!(decoded.height(), 8);
    }

    #[test]
    fn jpeg_resize_percent() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.jpg");
        let output = dir.path().join("a.png");
        image::RgbImage::new(64, 32).save(&input).unwrap();
        let g = Geometry {
            max: None,
            resize: Some(crate::geom::ResizeSpec::Percent(50.0)),
        };
        convert_image(&input, &output, None, &g).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 32);
        assert_eq!(decoded.height(), 16);
    }

    #[test]
    fn svg_to_png() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.svg");
        std::fs::write(
            &input,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#ff0000"/></svg>"##,
        )
        .unwrap();
        let output = dir.path().join("a.png");
        convert_image(&input, &output, None, &Geometry::default()).unwrap();
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.width(), 8);
        assert_eq!(decoded.height(), 8);
    }
}
