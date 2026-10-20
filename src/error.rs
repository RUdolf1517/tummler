use thiserror::Error;

#[derive(Debug, Error)]
pub enum TummlerError {
    #[error("Файл не найден: {0}")]
    FileNotFound(String),

    #[error("Неподдерживаемый формат: {0}")]
    UnsupportedFormat(String),

    #[error("Неподдерживаемая конвертация: {0} → {1}")]
    UnsupportedConversion(String, String),

    #[error("Внешняя зависимость не найдена: {0}. {1}")]
    DependencyMissing(String, String),

    #[error("Ошибка конвертации изображения: {0}")]
    ImageError(String),

    #[error("Ошибка конвертации медиа (ffmpeg): {0}")]
    MediaError(String),

    #[error("Ошибка конвертации документа (pandoc): {0}")]
    DocumentError(String),

    #[error("Ошибка конвертации архива: {0}")]
    ArchiveError(String),

    #[error("Ошибка ввода-вывода: {0}")]
    IoError(String),

    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for TummlerError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}
