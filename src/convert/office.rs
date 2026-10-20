use crate::error::TummlerError;
use crate::util::{get_extension, normalize_ext};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

const IO_BUF: usize = 256 * 1024;

type PptxZip = ZipWriter<BufWriter<File>>;

/// PowerPoint decks we convert natively (pptx → pdf via office2pdf, pdf → pptx via hayro).
pub fn is_slide_ext(ext: &str) -> bool {
    matches!(
        normalize_ext(ext).as_str(),
        "pptx" | "ppt" | "ppsx" | "pps" | "odp"
    )
}

/// `pptx → pdf` or `pdf → pptx`.
pub fn is_office_pair(src_ext: &str, dst_ext: &str) -> bool {
    let src = normalize_ext(src_ext);
    let dst = normalize_ext(dst_ext);
    (is_slide_ext(&src) && dst == "pdf") || (src == "pdf" && dst == "pptx")
}

/// pptx → pdf (office2pdf / Typst) and pdf → pptx (hayro raster + OOXML pack).
/// `_no_auto_install` / `_assume_yes` kept so the call site stays the same; nothing to install.
pub fn convert_office(
    input: &Path,
    output: &Path,
    target_ext: &str,
    _no_auto_install: bool,
    _assume_yes: bool,
) -> Result<(), TummlerError> {
    let src = normalize_ext(&get_extension(input).unwrap_or_default());
    let dst = normalize_ext(target_ext);
    if is_slide_ext(&src) && dst == "pdf" {
        pptx_to_pdf(input, output)
    } else if src == "pdf" && dst == "pptx" {
        pdf_to_pptx(input, output)
    } else {
        Err(TummlerError::UnsupportedConversion(src, dst))
    }
}

fn pptx_to_pdf(input: &Path, output: &Path) -> Result<(), TummlerError> {
    let src = normalize_ext(&get_extension(input).unwrap_or_default());
    if src != "pptx" && src != "ppsx" {
        return Err(TummlerError::OfficeError(format!(
            "native PDF export supports .pptx (not .{src})"
        )));
    }
    let result = if src == "ppsx" {
        let bytes = std::fs::read(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
        office2pdf::convert_bytes(
            &bytes,
            office2pdf::config::Format::Pptx,
            &office2pdf::config::ConvertOptions::default(),
        )
        .map_err(|e| TummlerError::OfficeError(e.to_string()))?
    } else {
        office2pdf::convert(input).map_err(|e| TummlerError::OfficeError(e.to_string()))?
    };
    if result.pdf.is_empty() {
        return Err(TummlerError::OfficeError(
            "office2pdf produced an empty PDF".into(),
        ));
    }
    std::fs::write(output, result.pdf).map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

fn pdf_to_pptx(input: &Path, output: &Path) -> Result<(), TummlerError> {
    let bytes = std::fs::read(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let pdf = Pdf::new(bytes).map_err(|e| TummlerError::OfficeError(e.to_string()))?;
    let pages = pdf.pages();
    let n = pages.len();
    if n == 0 {
        return Err(TummlerError::OfficeError("empty PDF".into()));
    }

    let cache = RenderCache::new();
    let interpreter = InterpreterSettings::default();
    let render_settings = RenderSettings::default();
    let pixmap_settings = PixmapSettings {
        x_scale: 2.0,
        y_scale: 2.0,
        bg_color: WHITE,
    };

    let raster = |idx: usize, page| -> Result<(u32, u32, Vec<u8>), TummlerError> {
        let rendered = render(
            page,
            &cache,
            &interpreter,
            &render_settings,
            &pixmap_settings,
        );
        let w = u32::from(rendered.width());
        let h = u32::from(rendered.height());
        let png = rendered
            .into_png()
            .map_err(|e| TummlerError::OfficeError(format!("page {}: {e}", idx + 1)))?;
        Ok((w, h, png))
    };

    let mut iter = pages.iter().enumerate();
    let Some((_, first)) = iter.next() else {
        return Err(TummlerError::OfficeError("empty PDF".into()));
    };
    let (w0, h0, png0) = raster(0, first)?;
    let cx = px_to_emu(w0);
    let cy = px_to_emu(h0);

    let mut zip = start_pptx(output, n, cx, cy)?;
    append_slide(&mut zip, 1, w0, h0, &png0, cx, cy)?;
    drop(png0);

    for (idx, page) in iter {
        let (w, h, png) = raster(idx, page)?;
        append_slide(&mut zip, idx + 1, w, h, &png, cx, cy)?;
    }

    zip.finish()
        .map_err(|e| TummlerError::OfficeError(e.to_string()))?;
    Ok(())
}

/// 144 dpi (2× 72) → EMU.
fn px_to_emu(px: u32) -> i64 {
    i64::from(px) * 914_400 / 144
}

fn xml_opts() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn png_opts() -> SimpleFileOptions {
    // PNG is already compressed; storing avoids a second deflate pass.
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

fn zip_err(e: impl ToString) -> TummlerError {
    TummlerError::OfficeError(e.to_string())
}

fn start_pptx(output: &Path, n: usize, cx: i64, cy: i64) -> Result<PptxZip, TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut zip = ZipWriter::new(BufWriter::with_capacity(IO_BUF, file));
    let xml = xml_opts();

    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
"#,
    );
    for i in 1..=n {
        types.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{i}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n"
        ));
    }
    types.push_str("</Types>");
    zip.start_file("[Content_Types].xml", xml)
        .map_err(zip_err)?;
    zip.write_all(types.as_bytes())?;

    zip.start_file("_rels/.rels", xml).map_err(zip_err)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#,
    )?;

    let mut pres = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldIdLst>
"#,
    );
    let mut pres_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#,
    );
    for i in 1..=n {
        let id = 255 + i as u32;
        pres.push_str(&format!("<p:sldId id=\"{id}\" r:id=\"rId{i}\"/>\n"));
        pres_rels.push_str(&format!(
            "<Relationship Id=\"rId{i}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{i}.xml\"/>\n"
        ));
    }
    pres.push_str(&format!(
        "</p:sldIdLst><p:sldSz cx=\"{cx}\" cy=\"{cy}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>"
    ));
    pres_rels.push_str("</Relationships>");

    zip.start_file("ppt/presentation.xml", xml)
        .map_err(zip_err)?;
    zip.write_all(pres.as_bytes())?;
    zip.start_file("ppt/_rels/presentation.xml.rels", xml)
        .map_err(zip_err)?;
    zip.write_all(pres_rels.as_bytes())?;
    Ok(zip)
}

fn append_slide(
    zip: &mut PptxZip,
    n: usize,
    w: u32,
    h: u32,
    png: &[u8],
    cx: i64,
    cy: i64,
) -> Result<(), TummlerError> {
    let scx = px_to_emu(w);
    let scy = px_to_emu(h);
    let slide = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/><a:chOff x="0" y="0"/><a:chExt cx="{cx}" cy="{cy}"/></a:xfrm></p:grpSpPr>
<p:pic>
<p:nvPicPr><p:cNvPr id="2" name="Picture {n}"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
<p:blipFill><a:blip r:embed="rId1"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{scx}" cy="{scy}"/></a:xfrm>
<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
</p:pic>
</p:spTree></p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#
    );
    zip.start_file(format!("ppt/slides/slide{n}.xml"), xml_opts())
        .map_err(zip_err)?;
    zip.write_all(slide.as_bytes())?;
    zip.start_file(format!("ppt/slides/_rels/slide{n}.xml.rels"), xml_opts())
        .map_err(zip_err)?;
    zip.write_all(
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{n}.png"/>
</Relationships>"#
        )
        .as_bytes(),
    )?;
    zip.start_file(format!("ppt/media/image{n}.png"), png_opts())
        .map_err(zip_err)?;
    zip.write_all(png)?;
    Ok(())
}

fn write_image_pptx(images: &[(u32, u32, Vec<u8>)], output: &Path) -> Result<(), TummlerError> {
    if images.is_empty() {
        return Err(TummlerError::OfficeError("empty PDF".into()));
    }
    let (sw, sh, _) = &images[0];
    let cx = px_to_emu(*sw);
    let cy = px_to_emu(*sh);
    let mut zip = start_pptx(output, images.len(), cx, cy)?;
    for (i, (w, h, png)) in images.iter().enumerate() {
        append_slide(&mut zip, i + 1, *w, *h, png, cx, cy)?;
    }
    zip.finish().map_err(zip_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_exts() {
        assert!(is_slide_ext("PPTX"));
        assert!(is_slide_ext("ppt"));
        assert!(is_slide_ext("odp"));
        assert!(!is_slide_ext("docx"));
        assert!(!is_slide_ext("pdf"));
    }

    #[test]
    fn pairs() {
        assert!(is_office_pair("pptx", "pdf"));
        assert!(is_office_pair("PPT", "PDF"));
        assert!(is_office_pair("pdf", "pptx"));
        assert!(!is_office_pair("pdf", "odp"));
        assert!(!is_office_pair("docx", "pdf"));
        assert!(!is_office_pair("pptx", "txt"));
        assert!(!is_office_pair("pptx", "pptx"));
    }

    #[test]
    fn packs_one_png_slide() {
        let png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("x.pptx");
        write_image_pptx(&[(8, 6, png)], &out).unwrap();
        assert!(out.metadata().unwrap().len() > 100);
    }
}
