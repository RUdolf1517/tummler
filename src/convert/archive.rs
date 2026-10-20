use crate::error::TummlerError;
use crate::util::{get_extension, normalize_ext};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn is_archive_ext(ext: &str) -> bool {
    matches!(
        ext,
        "zip" | "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst" | "gz" | "bz2" | "xz" | "zst"
    )
}

/// Конвертация архивов (перепаковка) и сжатие обычных файлов.
///
/// Поддерживает: zip, tar, tar.gz/tgz, tar.bz2, tar.xz, tar.zst, gz, bz2, xz, zst
/// - Архив → архив: распаковать во временную директорию и запаковать заново
/// - Обычный файл → gz/zip/tar.*: сжать / упаковать один файл
pub fn convert_archive(input: &Path, output: &Path) -> Result<(), TummlerError> {
    let input_ext = normalize_ext(&get_extension(input).unwrap_or_default());
    let output_ext = normalize_ext(&get_extension(output).unwrap_or_default());

    if !is_archive_ext(&output_ext) {
        return Err(TummlerError::ArchiveError(format!(
            "Неподдерживаемый целевой формат архива: {output_ext}"
        )));
    }

    if !is_archive_ext(&input_ext) {
        return pack_plain_file(input, output, &output_ext);
    }

    if input_ext == output_ext {
        return Err(TummlerError::Other(format!(
            "Исходный и целевой форматы совпадают: {input_ext}"
        )));
    }

    // Временная директория для распаковки
    let temp_dir = tempfile::tempdir().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let temp_path = temp_dir.path();

    // 1. Распаковать
    unpack_archive(input, &input_ext, temp_path)?;

    // 2. Запаковать
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
            "Неподдерживаемый исходный формат архива: {ext}"
        ))),
    }
}

fn pack_plain_file(input: &Path, output: &Path, ext: &str) -> Result<(), TummlerError> {
    match ext {
        "gz" | "bz2" | "xz" | "zst" => compress_file(input, output, ext),
        "zip" | "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst" => {
            let temp_dir = tempfile::tempdir().map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
            let name = input.file_name().unwrap_or_else(|| std::ffi::OsStr::new("file"));
            let dest = temp_dir.path().join(name);
            std::fs::copy(input, &dest).map_err(|e| TummlerError::IoError(e.to_string()))?;
            pack_archive(temp_dir.path(), output, ext)
        }
        _ => Err(TummlerError::ArchiveError(format!(
            "Неподдерживаемый целевой формат архива: {ext}"
        ))),
    }
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
                "Неподдерживаемый кодек сжатия: {ext}"
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
            "Нет файлов для сжатия в .{ext}"
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
            "Формат .{ext} поддерживает только один файл. Используйте .{hint} для нескольких файлов."
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
            "Неподдерживаемый целевой формат архива: {ext}"
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

fn unpack_tar(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let mut archive = tar::Archive::new(file);
    archive.unpack(dest).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn unpack_tar_gz(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(dest).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn unpack_tar_bz2(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let bz = bzip2::read::BzDecoder::new(file);
    let mut archive = tar::Archive::new(bz);
    archive.unpack(dest).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn unpack_tar_xz(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let xz = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(xz);
    archive.unpack(dest).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn unpack_tar_zst(input: &Path, dest: &Path) -> Result<(), TummlerError> {
    let file = File::open(input).map_err(|e| TummlerError::IoError(e.to_string()))?;
    let decoder = zstd::stream::read::Decoder::new(file).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
    Ok(())
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
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut files: Vec<PathBuf> = Vec::new();
    collect_files_recursive(src_dir, &mut files);

    for path in &files {
        let name = path.strip_prefix(src_dir).unwrap_or(path);
        let name_str = name.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(&name_str, options).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
        } else {
            zip.start_file(&name_str, options).map_err(|e| TummlerError::ArchiveError(e.to_string()))?;
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
}
