use super::{ConvertOpts, image, media};
use crate::{error::TummlerError, geom::Geometry, util};
use flate2::{Compression, write::ZlibEncoder};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use std::{io::Write, path::Path};

pub fn supports(ext: &str) -> bool {
    matches!(
        util::normalize_ext(ext).as_str(),
        "jpg" | "png" | "heic" | "heif"
    )
}

pub fn convert(
    inputs: &[impl AsRef<Path>],
    output: &Path,
    opts: &ConvertOpts<'_>,
) -> Result<(), TummlerError> {
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let pages = Ref::new(2);
    let mut refs = Ref::new(3);
    let mut page_ids = Vec::with_capacity(inputs.len());
    let mut decoded = None;
    pdf.catalog(catalog).pages(pages);

    for input in inputs {
        let input = input.as_ref();
        let ext = util::get_extension(input).unwrap_or_default();
        let source = if util::is_ffmpeg_image(&ext) {
            let directory = match &decoded {
                Some(directory) => directory,
                None => decoded.insert(tempfile::tempdir()?),
            };
            let path = directory.path().join("image.png");
            media::convert_media(
                input,
                &path,
                None,
                None,
                opts.no_auto_install,
                opts.assume_yes,
                &Geometry::default(),
                opts.ffmpeg_threads,
                true,
            )?;
            path
        } else {
            input.to_path_buf()
        };
        let image = image::load_image(&source)?;
        let rgb = if image.has_alpha() {
            let rgba = image.to_rgba8();
            ::image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
                let [r, g, b, a] = rgba[(x, y)].0;
                let alpha = u16::from(a);
                ::image::Rgb([r, g, b].map(|channel| {
                    ((u16::from(channel) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
                }))
            })
        } else {
            image.into_rgb8()
        };
        let (width, height) = rgb.dimensions();
        let (data, filter) = if util::normalize_ext(&ext) == "png" && opts.quality.is_none() {
            let mut compressed = ZlibEncoder::new(Vec::new(), Compression::fast());
            compressed.write_all(rgb.as_raw())?;
            (compressed.finish()?, Filter::FlateDecode)
        } else {
            let mut compressed = Vec::new();
            ::image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut compressed,
                opts.quality.unwrap_or(90),
            )
            .encode_image(&rgb)
            .map_err(|e| TummlerError::ImageError(e.to_string()))?;
            (compressed, Filter::DctDecode)
        };

        // Treat the source as 96 DPI and scale oversized pages to PDF's 200-inch limit.
        let scale = 0.75_f32.min(14_400.0 / width.max(height) as f32);
        let (w, h) = (width as f32 * scale, height as f32 * scale);
        let page = refs.bump();
        let image = refs.bump();
        let content_id = refs.bump();
        page_ids.push(page);
        let name = Name(b"Image");
        let mut page = pdf.page(page);
        page.parent(pages)
            .media_box(Rect::new(0.0, 0.0, w, h))
            .contents(content_id);
        page.resources().x_objects().pair(name, image);
        page.finish();

        let mut object = pdf.image_xobject(image, &data);
        object
            .width(width as i32)
            .height(height as i32)
            .color_space()
            .device_rgb();
        object.bits_per_component(8).filter(filter);
        object.finish();

        let mut content = Content::new();
        content.transform([w, 0.0, 0.0, h, 0.0, 0.0]).x_object(name);
        pdf.stream(content_id, &content.finish());
    }
    pdf.pages(pages)
        .kids(page_ids.iter().copied())
        .count(page_ids.len() as i32);
    std::fs::write(output, pdf.finish())?;
    Ok(())
}
