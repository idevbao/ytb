use serde::Deserialize;
use std::path::PathBuf;

/// Configuration management for the application.
///
/// Provides centralized configuration options for controlling:
/// - Concurrent download limits
/// - Directory paths
/// - Buffer sizes
/// - External service URLs

/// Configuration for the video downloader application.
///
/// Controls various aspects of the application's behavior including
/// concurrency limits, file paths, and external service configurations.
///
/// # Examples
///
/// ```
/// use application::Config;
///
/// let config = Config::default();
/// assert!(config.concurrent_downloads > 0);
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    pub concurrent_downloads: usize,
    pub buffer_size: usize,
    pub output_dir: PathBuf,
    pub input_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub sheet_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            concurrent_downloads: 10,
            buffer_size: 10,
            output_dir: PathBuf::from("output"),
            input_dir: PathBuf::from("input"),
            libraries_dir: PathBuf::from("libs"),
            sheet_url: Some(String::from("https://docs.google.com/spreadsheets/d/160Obd-Z9nMz2LfnbqUVvvwCvel7AGfjwREZtVwtM1_M")),
        }
    }
}
