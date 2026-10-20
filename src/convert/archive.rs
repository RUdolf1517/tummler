use crate::error::TummlerError;
use crate::util::{get_extension, normalize_ext};
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

fn is_archive_ext(ext: &str) -> bool {
    matches!(
        ext,
        "zip" | "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst" | "gz" | "bz2" | "xz" | "zst"
    )
}

fn is_tar_family(ext: &str) -> bool {
    matches!(
        ext,
        "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst"
    )
}

/// Archive conversion (repack) and compression of plain files.
///
/// Supports: zip, tar, tar.gz/tgz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst
/// - zip ↔ tar.* and tar ↔ tar.*: stream entries / recompress (no unpack)
/// - other archive pairs: unpack to a temp dir (slip-safe) and pack again
/// - Plain file → gz/zip/tar.*: compress / pack a single file
pub fn convert_archive(input: &Path, output: &Path) -> Result<(), TummlerError> {
    let input_ext = normalize_ext(&get_extension(input).unwrap_or_default());
    let output_ext = normalize_ext(&get_extension(output).unwrap_or_default());

    if !is_archive_ext(&output_ext) {
        return Err(TummlerError::ArchiveError(format!(
            "unsupported target archive format: {output_ext}"
        )));
    }

    if !is_archive_ext(&input_ext) {
        return pack_plain_file(input, output, &output_ext);
    }

    if input_ext == output_ext {
        return Err(TummlerError::Other(format!(
            "source and target formats are the same: {input_ext}"
        )));
    }

    if input_ext == "zip" && is_tar_family(&output_ext) {
        return zip_to_tar(input, output, &output_ext);
    }
    if is_tar_family(&input_ext) && output_ext == "zip" {
        return tar_to_zip(input, output, &input_ext);
    }
    if is_tar_family(&input_ext) && is_tar_family(&output_ext) {
        return recompress_tar(input, &input_ext, output, &output_ext);
    }

    // Temp directory for unpacking
    let temp_dir = tempfile::tempdir().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let temp_path = temp_dir.path();

    // 1. Unpack
    unpack_archive(input, &input_ext, temp_path)?;

    // 2. Pack
    pack_archive(temp_path, output, &output_ext)?;

    Ok(())
}

fn unpack_archive(input: &Path, ext: &str, dest: &Path) -> Result<(), TummlerError> {
    match ext {
        "zip" => unpack_zip(input, dest),
        "tar" => unpack_tar(input, dest),
        "tar.gz" => unpack_tar_gz(input, dest),
        "tar.bz2" => unpack_tar_bz2(input, dest),
        "tar.xz" => unpack_tar_xz(input, dest),
        "tar.zst" => unpack_tar_zst(input, dest),
        "gz" => unpack_gz(input, dest),
        "bz2" => unpack_bz2(input, dest),
        "xz" => unpack_xz(input, dest),
        "zst" => unpack_zst(input, dest),
        _ => Err(TummlerError::ArchiveError(format!(
            "unsupported source archive format: {ext}"
        ))),
    }
}

fn pack_plain_file(input: &Path, output: &Path, ext: &str) -> Result<(), TummlerError> {
    match ext {
        "gz" | "bz2" | "xz" | "zst" => compress_file(input, output, ext),
        "zip" => pack_one_zip(input, output),
        "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst" => pack_one_tar(input, output, ext),
        _ => Err(TummlerError::ArchiveError(format!(
            "unsupported target archive format: {ext}"
        ))),
    }
}

fn entry_name(input: &Path) -> std::borrow::Cow<'_, str> {
    input
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or(std::borrow::Cow::Borrowed("file"))
}

fn pack_one_zip(input: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut zip = zip::ZipWriter::new(io::BufWriter::with_capacity(64 * 1024, file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file(entry_name(input).as_ref(), options)
        .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let mut f = io::BufReader::with_capacity(
        64 * 1024,
        File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?,
    );
    io::copy(&mut f, &mut zip).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut inner = zip
        .finish()
        .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    inner.flush().map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

fn pack_one_tar(input: &Path, output: &Path, ext: &str) -> Result<(), TummlerError> {
    let name = entry_name(input);
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    match ext {
        "tar" => {
            let mut builder = tar::Builder::new(file);
            builder
                .append_path_with_name(input, name.as_ref())
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            builder
                .finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.gz" => {
            let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut builder = tar::Builder::new(gz);
            builder
                .append_path_with_name(input, name.as_ref())
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let gz = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            gz.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.bz2" => {
            let bz = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
            let mut builder = tar::Builder::new(bz);
            builder
                .append_path_with_name(input, name.as_ref())
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let bz = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            bz.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.xz" => {
            let xz = xz2::write::XzEncoder::new(file, 6);
            let mut builder = tar::Builder::new(xz);
            builder
                .append_path_with_name(input, name.as_ref())
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let xz = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            xz.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.zst" => {
            let enc = zstd::stream::write::Encoder::new(file, 3)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let mut builder = tar::Builder::new(enc);
            builder
                .append_path_with_name(input, name.as_ref())
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let enc = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            enc.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        _ => {
            return Err(TummlerError::ArchiveError(format!(
                "unsupported target archive format: {ext}"
            )));
        }
    }
    Ok(())
}

fn zip_to_tar(input: &Path, output: &Path, ext: &str) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let out = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    match ext {
        "tar" => {
            let mut builder = tar::Builder::new(out);
            append_zip_to_tar(&mut builder, &mut zip)?;
            builder
                .finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.gz" => {
            let gz = flate2::write::GzEncoder::new(out, flate2::Compression::default());
            let mut builder = tar::Builder::new(gz);
            append_zip_to_tar(&mut builder, &mut zip)?;
            let gz = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            gz.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.bz2" => {
            let bz = bzip2::write::BzEncoder::new(out, bzip2::Compression::default());
            let mut builder = tar::Builder::new(bz);
            append_zip_to_tar(&mut builder, &mut zip)?;
            let bz = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            bz.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.xz" => {
            let xz = xz2::write::XzEncoder::new(out, 6);
            let mut builder = tar::Builder::new(xz);
            append_zip_to_tar(&mut builder, &mut zip)?;
            let xz = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            xz.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.zst" => {
            let enc = zstd::stream::write::Encoder::new(out, 3)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let mut builder = tar::Builder::new(enc);
            append_zip_to_tar(&mut builder, &mut zip)?;
            let enc = builder
                .into_inner()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            enc.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        _ => {
            return Err(TummlerError::ArchiveError(format!(
                "unsupported target archive format: {ext}"
            )));
        }
    }
    Ok(())
}

fn append_zip_to_tar<W: Write, R: io::Read + Seek>(
    builder: &mut tar::Builder<W>,
    zip: &mut zip::ZipArchive<R>,
) -> Result<(), TummlerError> {
    for i in 0..zip.len() {
        let mut zf = zip
            .by_index(i)
            .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        let Some(enclosed) = zf.enclosed_name() else {
            continue;
        };
        let name = enclosed.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }
        let mut header = tar::Header::new_gnu();
        header
            .set_path(&name)
            .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        let is_dir = zf.is_dir() || name.ends_with('/');
        if is_dir {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mode(zf.unix_mode().unwrap_or(0o755));
            header.set_cksum();
            builder
                .append(&header, io::empty())
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        } else {
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(zf.size());
            header.set_mode(zf.unix_mode().unwrap_or(0o644));
            header.set_cksum();
            builder
                .append(&header, &mut zf)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
    }
    Ok(())
}

/// Skip `..`, absolute paths, Windows prefixes. Same idea as zip `enclosed_name`.
fn safe_rel_path(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::Normal(p) => out.push(p),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// PK zip vs OOXML/ODF — stdin sniff only sees `PK`.
pub fn peek_zip_kind(path: &Path) -> &'static str {
    let Ok(file) = File::open(path) else {
        return ".zip";
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return ".zip";
    };
    let mut mime = String::new();
    if let Ok(mut f) = zip.by_name("mimetype") {
        let _ = f.read_to_string(&mut mime);
    }
    if mime.contains("opendocument.text") {
        return ".odt";
    }
    if mime.contains("opendocument.spreadsheet") {
        return ".ods";
    }
    if mime.contains("opendocument.presentation") {
        return ".odp";
    }
    for i in 0..zip.len() {
        let Ok(e) = zip.by_index(i) else {
            continue;
        };
        let Ok(name) = e.name() else {
            continue;
        };
        let n = name.replace('\\', "/").to_ascii_lowercase();
        if n.starts_with("word/") {
            return ".docx";
        }
        if n.starts_with("xl/") {
            return ".xlsx";
        }
        if n.starts_with("ppt/") {
            return ".pptx";
        }
    }
    ".zip"
}

fn tar_to_zip(input: &Path, output: &Path, in_ext: &str) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    match in_ext {
        "tar" => tar_reader_to_zip(tar::Archive::new(file), output),
        "tar.gz" => tar_reader_to_zip(tar::Archive::new(flate2::read::GzDecoder::new(file)), output),
        "tar.bz2" => tar_reader_to_zip(tar::Archive::new(bzip2::read::BzDecoder::new(file)), output),
        "tar.xz" => tar_reader_to_zip(tar::Archive::new(xz2::read::XzDecoder::new(file)), output),
        "tar.zst" => {
            let dec = zstd::stream::read::Decoder::new(file)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            tar_reader_to_zip(tar::Archive::new(dec), output)
        }
        _ => Err(TummlerError::ArchiveError(format!(
            "unsupported source archive format: {in_ext}"
        ))),
    }
}

fn tar_reader_to_zip<R: Read>(
    mut archive: tar::Archive<R>,
    output: &Path,
) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut zip = zip::ZipWriter::new(io::BufWriter::with_capacity(64 * 1024, file));
    let file_opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let dir_opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);

    for ent in archive
        .entries()
        .map_err(|e| TummlerError::ArchiveError(e.to_string()))?
    {
        let mut ent = ent.map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        let name = match ent.path() {
            Ok(p) => safe_rel_path(&p).map(|rel| rel.to_string_lossy().replace('\\', "/")),
            Err(_) => None,
        };
        let Some(name) = name.filter(|n| !n.is_empty()) else {
            io::copy(&mut ent, &mut io::sink()).map_err(|e| TummlerError::IoError(e.to_string()))?;
            continue;
        };
        let kind = ent.header().entry_type();
        if kind.is_dir() {
            zip.add_directory(&name, dir_opts)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        } else if kind.is_file() {
            zip.start_file(&name, file_opts)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            io::copy(&mut ent, &mut zip).map_err(|e| TummlerError::IoError(e.to_string()))?;
        } else {
            io::copy(&mut ent, &mut io::sink()).map_err(|e| TummlerError::IoError(e.to_string()))?;
        }
    }
    let mut inner = zip
        .finish()
        .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    inner.flush().map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

/// `tar` ↔ `tar.gz` / `tar.bz2` / … is a compression wrap, not unpack+repack.
fn recompress_tar(
    input: &Path,
    in_ext: &str,
    output: &Path,
    out_ext: &str,
) -> Result<(), TummlerError> {
    let inf = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let outf = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    match in_ext {
        "tar" => encode_tar_stream(inf, outf, out_ext),
        "tar.gz" => encode_tar_stream(flate2::read::GzDecoder::new(inf), outf, out_ext),
        "tar.bz2" => encode_tar_stream(bzip2::read::BzDecoder::new(inf), outf, out_ext),
        "tar.xz" => encode_tar_stream(xz2::read::XzDecoder::new(inf), outf, out_ext),
        "tar.zst" => {
            let dec = zstd::stream::read::Decoder::new(inf)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            encode_tar_stream(dec, outf, out_ext)
        }
        _ => Err(TummlerError::ArchiveError(format!(
            "unsupported source archive format: {in_ext}"
        ))),
    }
}

fn encode_tar_stream<R: Read>(
    mut src: R,
    outf: File,
    out_ext: &str,
) -> Result<(), TummlerError> {
    match out_ext {
        "tar" => {
            let mut out = io::BufWriter::with_capacity(64 * 1024, outf);
            io::copy(&mut src, &mut out).map_err(|e| TummlerError::IoError(e.to_string()))?;
            out.flush().map_err(|e| TummlerError::IoError(e.to_string()))?;
        }
        "tar.gz" => {
            let mut enc =
                flate2::write::GzEncoder::new(outf, flate2::Compression::default());
            io::copy(&mut src, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.bz2" => {
            let mut enc = bzip2::write::BzEncoder::new(outf, bzip2::Compression::default());
            io::copy(&mut src, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.xz" => {
            let mut enc = xz2::write::XzEncoder::new(outf, 6);
            io::copy(&mut src, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "tar.zst" => {
            let mut enc = zstd::stream::write::Encoder::new(outf, 3)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            io::copy(&mut src, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish()
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        _ => {
            return Err(TummlerError::ArchiveError(format!(
                "unsupported target archive format: {out_ext}"
            )));
        }
    }
    Ok(())
}

fn compress_file(input: &Path, output: &Path, ext: &str) -> Result<(), TummlerError> {
    let mut input_file = io::BufReader::with_capacity(
        64 * 1024,
        File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?,
    );
    let output_file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    match ext {
        "gz" => {
            let mut enc = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
            io::copy(&mut input_file, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "bz2" => {
            let mut enc = bzip2::write::BzEncoder::new(output_file, bzip2::Compression::default());
            io::copy(&mut input_file, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "xz" => {
            let mut enc = xz2::write::XzEncoder::new(output_file, 6);
            io::copy(&mut input_file, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        "zst" => {
            let mut enc = zstd::stream::write::Encoder::new(output_file, 3)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            io::copy(&mut input_file, &mut enc).map_err(|e| TummlerError::IoError(e.to_string()))?;
            enc.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
        _ => {
            return Err(TummlerError::ArchiveError(format!(
                "unsupported compression codec: {ext}"
            )));
        }
    }
    Ok(())
}

fn single_file_in(src_dir: &Path, ext: &str) -> Result<PathBuf, TummlerError> {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_files_recursive(src_dir, &mut files);
    let files: Vec<_> = files.into_iter().filter(|p| p.is_file()).collect();
    if files.is_empty() {
        return Err(TummlerError::ArchiveError(format!(
            "no files to compress into .{ext}"
        )));
    }
    if files.len() > 1 {
        let hint = match ext {
            "gz" => "tar.gz",
            "bz2" => "tar.bz2",
            "xz" => "tar.xz",
            "zst" => "tar.zst",
            other => other,
        };
        return Err(TummlerError::ArchiveError(format!(
            "format .{ext} supports a single file. Use .{hint} for multiple files."
        )));
    }
    Ok(files.into_iter().next().unwrap())
}

fn pack_archive(src_dir: &Path, output: &Path, ext: &str) -> Result<(), TummlerError> {
    match ext {
        "zip" => pack_zip(src_dir, output),
        "tar" => pack_tar(src_dir, output),
        "tar.gz" => pack_tar_gz(src_dir, output),
        "tar.bz2" => pack_tar_bz2(src_dir, output),
        "tar.xz" => pack_tar_xz(src_dir, output),
        "tar.zst" => pack_tar_zst(src_dir, output),
        "gz" => pack_gz(src_dir, output),
        "bz2" => pack_bz2(src_dir, output),
        "xz" => pack_xz(src_dir, output),
        "zst" => pack_zst(src_dir, output),
        _ => Err(TummlerError::ArchiveError(format!(
            "unsupported target archive format: {ext}"
        ))),
    }
}

// ── Unpack ──────────────────────────────────────────────────────────────

fn unpack_zip(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        let Some(enclosed) = file.enclosed_name() else {
            continue;
        };
        let out_path = dest.join(enclosed);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| TummlerError::IoError(e.to_string()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| TummlerError::IoError(e.to_string()))?;
            }
            let mut out_file = File::create(&out_path).map_err(|e| TummlerError::IoError(e.to_string()))?;
            io::copy(&mut file, &mut out_file).map_err(|e| TummlerError::IoError(e.to_string()))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode));
            }
        }
    }
    Ok(())
}

fn unpack_tar_archive<R: Read>(
    mut archive: tar::Archive<R>,
    dest: &Path,
) -> Result<(), TummlerError> {
    // `unpack_in` skips `..` / root (zip-slip). Plain `unpack()` does not.
    for ent in archive
        .entries()
        .map_err(|e| TummlerError::ArchiveError(e.to_string()))?
    {
        let mut ent = ent.map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        ent.unpack_in(dest)
            .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    }
    Ok(())
}

fn unpack_tar(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    unpack_tar_archive(tar::Archive::new(file), dest)
}

fn unpack_tar_gz(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    unpack_tar_archive(tar::Archive::new(flate2::read::GzDecoder::new(file)), dest)
}

fn unpack_tar_bz2(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    unpack_tar_archive(tar::Archive::new(bzip2::read::BzDecoder::new(file)), dest)
}

fn unpack_tar_xz(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    unpack_tar_archive(tar::Archive::new(xz2::read::XzDecoder::new(file)), dest)
}

fn unpack_tar_zst(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    unpack_tar_archive(tar::Archive::new(decoder), dest)
}

fn unpack_gz(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut gz = flate2::read::GzDecoder::new(file);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_path = dest.join(stem);
    let mut out_file = File::create(&out_path).map_err(|e| TummlerError::IoError(e.to_string()))?;
    io::copy(&mut gz, &mut out_file).map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

fn unpack_bz2(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut bz = bzip2::read::BzDecoder::new(file);
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let out_path = dest.join(stem);
    let mut out_file = File::create(&out_path).map_err(|e| TummlerError::IoError(e.to_string()))?;
    io::copy(&mut bz, &mut out_file).map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

fn unpack_xz(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut xz = xz2::read::XzDecoder::new(file);
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let out_path = dest.join(stem);
    let mut out_file = File::create(&out_path).map_err(|e| TummlerError::IoError(e.to_string()))?;
    io::copy(&mut xz, &mut out_file).map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

fn unpack_zst(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut decoder = zstd::stream::read::Decoder::new(file).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let out_path = dest.join(stem);
    let mut out_file = File::create(&out_path).map_err(|e| TummlerError::IoError(e.to_string()))?;
    io::copy(&mut decoder, &mut out_file).map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

// ── Pack ────────────────────────────────────────────────────────────────

fn pack_zip(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut zip = zip::ZipWriter::new(file);
    let file_opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let dir_opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);

    let mut files: Vec<PathBuf> = Vec::new();
    collect_files_recursive(src_dir, &mut files);

    for path in &files {
        let name = path.strip_prefix(src_dir).unwrap_or(path);
        let name_str = name.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(&name_str, dir_opts).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        } else {
            zip.start_file(&name_str, file_opts).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let mut f = io::BufReader::with_capacity(
                64 * 1024,
                File::open(path).map_err(|e| TummlerError::IoError(e.to_string()))?,
            );
            io::copy(&mut f, &mut zip).map_err(|e| TummlerError::IoError(e.to_string()))?;
        }
    }

    zip.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn pack_tar(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut builder = tar::Builder::new(file);
    append_dir_to_tar(&mut builder, src_dir, src_dir)?;
    builder.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn pack_tar_gz(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(gz);
    append_dir_to_tar(&mut builder, src_dir, src_dir)?;
    let gz = builder.into_inner().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    gz.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn pack_tar_bz2(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let bz = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    let mut builder = tar::Builder::new(bz);
    append_dir_to_tar(&mut builder, src_dir, src_dir)?;
    let bz = builder.into_inner().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    bz.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn pack_tar_xz(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let xz = xz2::write::XzEncoder::new(file, 6);
    let mut builder = tar::Builder::new(xz);
    append_dir_to_tar(&mut builder, src_dir, src_dir)?;
    let xz = builder.into_inner().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    xz.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn pack_tar_zst(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let enc = zstd::stream::write::Encoder::new(file, 3).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let mut builder = tar::Builder::new(enc);
    append_dir_to_tar(&mut builder, src_dir, src_dir)?;
    let enc = builder.into_inner().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    enc.finish().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn pack_gz(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = single_file_in(src_dir, "gz")?;
    compress_file(&file, output, "gz")
}

fn pack_bz2(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = single_file_in(src_dir, "bz2")?;
    compress_file(&file, output, "bz2")
}

fn pack_xz(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = single_file_in(src_dir, "xz")?;
    compress_file(&file, output, "xz")
}

fn pack_zst(src_dir: &Path, output: &Path) -> Result<(), TummlerError> {
    let file = single_file_in(src_dir, "zst")?;
    compress_file(&file, output, "zst")
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path != dir {
            out.push(path.to_path_buf());
        }
    }
}

fn append_dir_to_tar<W: Write>(builder: &mut tar::Builder<W>, src_dir: &Path, base: &Path) -> Result<(), TummlerError> {
    let entries = std::fs::read_dir(src_dir).map_err(|e| TummlerError::IoError(e.to_string()))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(base).unwrap_or(&path);
        if path.is_dir() {
            builder
                .append_dir(name, &path)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            append_dir_to_tar(builder, &path, base)?;
        } else {
            builder
                .append_path_with_name(&path, name)
                .map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_plain_file_to_gz_and_back() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("hello.txt");
        let payload = b"hello world hello world hello world";
        std::fs::write(&input, payload).unwrap();
        let gz = dir.path().join("hello.txt.gz");
        convert_archive(&input, &gz).unwrap();
        assert!(gz.exists());
        assert!(gz.metadata().unwrap().len() > 0);

        let unpacked = tempfile::tempdir().unwrap();
        unpack_gz(&gz, unpacked.path()).unwrap();
        let restored = std::fs::read(unpacked.path().join("hello.txt")).unwrap();
        assert_eq!(restored, payload);
    }

    #[test]
    fn plain_file_to_zip() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("note.txt");
        std::fs::write(&input, b"zip me").unwrap();
        let zip_path = dir.path().join("note.zip");
        convert_archive(&input, &zip_path).unwrap();
        assert!(zip_path.exists());

        let unpacked = tempfile::tempdir().unwrap();
        unpack_zip(&zip_path, unpacked.path()).unwrap();
        let restored = std::fs::read(unpacked.path().join("note.txt")).unwrap();
        assert_eq!(restored, b"zip me");
    }

    #[test]
    fn plain_file_to_tar_gz() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("note.txt");
        std::fs::write(&input, b"tar me").unwrap();
        let out = dir.path().join("note.tar.gz");
        convert_archive(&input, &out).unwrap();
        let unpacked = tempfile::tempdir().unwrap();
        unpack_tar_gz(&out, unpacked.path()).unwrap();
        let restored = std::fs::read(unpacked.path().join("note.txt")).unwrap();
        assert_eq!(restored, b"tar me");
    }

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn zip_to_tar_gz_keeps_entries() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("a.zip");
        write_zip(
            &zip_path,
            &[("a.txt", b"aaa"), ("dir/b.txt", b"bbb")],
        );
        let out = dir.path().join("a.tar.gz");
        convert_archive(&zip_path, &out).unwrap();
        let unpacked = tempfile::tempdir().unwrap();
        unpack_tar_gz(&out, unpacked.path()).unwrap();
        assert_eq!(std::fs::read(unpacked.path().join("a.txt")).unwrap(), b"aaa");
        assert_eq!(
            std::fs::read(unpacked.path().join("dir/b.txt")).unwrap(),
            b"bbb"
        );
    }

    #[test]
    fn zip_slip_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        write_zip(&zip_path, &[("../evil.txt", b"nope"), ("ok.txt", b"ok")]);
        let out = dir.path().join("evil.tar");
        convert_archive(&zip_path, &out).unwrap();
        let unpacked = tempfile::tempdir().unwrap();
        unpack_tar(&out, unpacked.path()).unwrap();
        assert_eq!(std::fs::read(unpacked.path().join("ok.txt")).unwrap(), b"ok");
        assert!(!dir.path().join("evil.txt").exists());
        assert!(!unpacked.path().join("evil.txt").exists());
    }

    fn write_tar(path: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut builder = tar::Builder::new(file);
        for (name, data) in files {
            let mut header = tar::Header::new_gnu();
            if header.set_path(name).is_err() {
                let b = name.as_bytes();
                header.as_old_mut().name[..b.len()].copy_from_slice(b);
            }
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, *data).unwrap();
        }
        builder.finish().unwrap();
    }

    #[test]
    fn tar_gz_to_tar_is_a_wrap() {
        let dir = tempfile::tempdir().unwrap();
        let tar_path = dir.path().join("a.tar");
        write_tar(&tar_path, &[("a.txt", b"aaa"), ("b.txt", b"bbb")]);
        let gz = dir.path().join("a.tar.gz");
        convert_archive(&tar_path, &gz).unwrap();
        let out = dir.path().join("b.tar");
        convert_archive(&gz, &out).unwrap();
        let unpacked = tempfile::tempdir().unwrap();
        unpack_tar(&out, unpacked.path()).unwrap();
        assert_eq!(std::fs::read(unpacked.path().join("a.txt")).unwrap(), b"aaa");
        assert_eq!(std::fs::read(unpacked.path().join("b.txt")).unwrap(), b"bbb");
    }

    #[test]
    fn tar_to_zip_keeps_entries() {
        let dir = tempfile::tempdir().unwrap();
        let tar_path = dir.path().join("a.tar");
        write_tar(&tar_path, &[("a.txt", b"aaa"), ("dir/b.txt", b"bbb")]);
        let zip_path = dir.path().join("a.zip");
        convert_archive(&tar_path, &zip_path).unwrap();
        let unpacked = tempfile::tempdir().unwrap();
        unpack_zip(&zip_path, unpacked.path()).unwrap();
        assert_eq!(std::fs::read(unpacked.path().join("a.txt")).unwrap(), b"aaa");
        assert_eq!(
            std::fs::read(unpacked.path().join("dir/b.txt")).unwrap(),
            b"bbb"
        );
    }

    #[test]
    fn tar_slip_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let tar_path = dir.path().join("evil.tar");
        write_tar(
            &tar_path,
            &[("../evil.txt", b"nope"), ("ok.txt", b"ok")],
        );
        let unpacked = tempfile::tempdir().unwrap();
        unpack_tar(&tar_path, unpacked.path()).unwrap();
        assert_eq!(std::fs::read(unpacked.path().join("ok.txt")).unwrap(), b"ok");
        assert!(!dir.path().join("evil.txt").exists());
        assert!(!unpacked.path().join("evil.txt").exists());
        assert!(!unpacked.path().parent().unwrap().join("evil.txt").exists());
    }

    #[test]
    fn peek_xlsx_is_not_a_plain_zip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.xlsx");
        write_zip(
            &path,
            &[
                ("[Content_Types].xml", b"<Types/>"),
                ("xl/workbook.xml", b"<workbook/>"),
            ],
        );
        assert_eq!(peek_zip_kind(&path), ".xlsx");
        let zip = dir.path().join("a.zip");
        write_zip(&zip, &[("readme.txt", b"hi")]);
        assert_eq!(peek_zip_kind(&zip), ".zip");

        let odt = dir.path().join("t.odt");
        write_zip(
            &odt,
            &[(
                "mimetype",
                b"application/vnd.oasis.opendocument.text",
            )],
        );
        assert_eq!(peek_zip_kind(&odt), ".odt");

        let docx = dir.path().join("t.docx");
        write_zip(&docx, &[("word/document.xml", b"<w/>")]);
        assert_eq!(peek_zip_kind(&docx), ".docx");
    }

    #[test]
    fn safe_rel_rejects_parent_and_absolute() {
        assert_eq!(
            safe_rel_path(Path::new("ok/a.txt")).as_deref(),
            Some(Path::new("ok/a.txt"))
        );
        assert!(safe_rel_path(Path::new("../evil")).is_none());
        assert!(safe_rel_path(Path::new("/etc/passwd")).is_none());
    }
}
