use thiserror::Error;

#[derive(Debug, Error)]
pub enum TummlerError {
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("unsupported conversion: {0} → {1}")]
    UnsupportedConversion(String, String),

    #[error("missing dependency: {0}. {1}")]
    DependencyMissing(String, String),

    #[error("image conversion failed: {0}")]
    ImageError(String),

    #[error("media conversion failed (ffmpeg): {0}")]
    MediaError(String),

    #[error("document conversion failed (pandoc): {0}")]
    DocumentError(String),

    #[error("spreadsheet conversion failed: {0}")]
    SpreadsheetError(String),

    #[error("office conversion failed: {0}")]
    OfficeError(String),

    #[error("archive conversion failed: {0}")]
    ArchiveError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for TummlerError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}
