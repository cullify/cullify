use std::io;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
    #[error("config parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("config serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("json serialize error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("tauri runtime error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("app data directory is not available")]
    MissingDataDir,
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("folder does not exist or is not a directory: {0}")]
    InvalidFolder(String),
    #[error("folder has no supported images: {0}")]
    NoSupportedImages(String),
    #[error("unsupported project mode: {0}")]
    UnsupportedMode(String),
    #[error("project name cannot be empty")]
    EmptyProjectName,
    #[error("invalid decision: {0}")]
    InvalidDecision(String),
    #[error("photo batch update mismatch: expected {expected}, updated {updated}")]
    PhotoBatchMismatch { expected: usize, updated: usize },
    #[error("project has no photos to export: {0}")]
    EmptyExport(String),
    #[error("source photo is missing: {0}")]
    SourcePhotoMissing(String),
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}
