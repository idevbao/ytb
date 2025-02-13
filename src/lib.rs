/// A concurrent video downloader application.
///
/// This library provides functionality to download videos from various sources
/// including Google Sheets and local text files. It handles concurrent downloads,
/// progress tracking, and resource management.
///
/// # Architecture
///
/// The application is structured into several key components:
/// - `Config`: Application configuration management
/// - `Downloader`: Core video downloading functionality
/// - `SheetClient`: Google Sheets integration
/// - `DownloadProgress`: Progress tracking and reporting
///
/// # Example
/// ```no_run
/// use application::{Config, Downloader};
///
/// async fn example() {
///     let config = Config::default();
///     let downloader = Downloader::new(config).await.unwrap();
///     // ... use downloader
/// }
/// ```
// Move shared structs, traits and functions here
pub mod config;
pub mod downloader;
pub mod error;
pub mod progress;
pub mod sheet;

// Re-export commonly used items
pub use config::Config;
pub use downloader::Downloader;
pub use error::AppError;
pub use progress::DownloadProgress;
pub use sheet::SheetClient;
