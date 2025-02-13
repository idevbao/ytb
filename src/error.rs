use std::io;
use thiserror::Error;
use url;

/// Error types for the application.
///
/// Defines a comprehensive error handling system that covers:
/// - IO operations
/// - Network requests
/// - URL parsing
/// - Video processing
/// - External service interactions

/// Represents all possible errors that can occur in the application.
///
/// # Error Categories
///
/// - IO: File system operations
/// - Network: HTTP requests and responses
/// - Parsing: URL and data parsing
/// - Youtube: Video download and processing
/// - Custom: Application-specific errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Sheet error: {0}")]
    Sheet(String),

    #[error("Youtube error: {0}")]
    Youtube(#[from] yt_dlp::error::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("{0}")]
    Custom(String),
}

impl From<&str> for AppError {
    fn from(error: &str) -> Self {
        AppError::Custom(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        AppError::Custom(error)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
