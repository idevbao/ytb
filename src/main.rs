use application::error::Result;
use application::{Config, Downloader, SheetClient};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use tracing::{error, info};

/// Main entry point for the application.
///
/// # Steps
/// 1. Initializes logging with file, line numbers and thread IDs
/// 2. Creates a default configuration
/// 3. Initializes the downloader with required directories
/// 4. Runs the main application logic
///
/// # Errors
/// Returns error if:
/// - Logging initialization fails
/// - Downloader creation fails
/// - Application processing fails
#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting application...");

    let config = Config::default();
    let downloader = Downloader::new(config).await?;

    if let Err(e) = run_application(&downloader).await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }

    info!("Application completed successfully");
    Ok(())
}

/// Orchestrates concurrent processing of video downloads from multiple sources.
///
/// # Processing Flow
/// 1. Creates a vector of concurrent tasks
/// 2. If configured, adds Google Sheet processing task
/// 3. Adds local file processing task
/// 4. Executes all tasks concurrently
///
/// # Arguments
/// * `downloader` - Handles video download operations and configuration
///
/// # Errors
/// Returns error if either:
/// - Sheet processing fails
/// - Local file processing fails
/// - Task joining fails
async fn run_application(downloader: &Downloader) -> Result<()> {
    let mut tasks: Vec<futures::future::BoxFuture<'_, Result<()>>> = Vec::new();

    // Process Google Sheet if configured
    if let Some(sheet_url) = &downloader.config().sheet_url {
        let sheet_client = SheetClient::new();
        // tasks.push(Box::pin());
        process_sheet(downloader, sheet_client, sheet_url).await;
    }

    // Process local files
    // tasks.push(Box::pin(process_local_files(downloader)));
    process_local_files(downloader).await;
    

    // Run all tasks concurrently
    futures::future::try_join_all(tasks).await?;
    Ok(())
}

/// Processes video URLs from a Google Sheet source.
///
/// # Processing Steps
/// 1. Fetches URLs from the provided Google Sheet
/// 2. Downloads videos for all valid URLs
///
/// # Arguments
/// * `downloader` - Handles video download operations
/// * `sheet_client` - Manages Google Sheet interactions
/// * `sheet_url` - Complete URL to the Google Sheet
///
/// # Errors
/// Returns error if:
/// - Sheet URL is invalid
/// - Sheet access fails
/// - URL fetching fails
/// - Video downloading fails
async fn process_sheet(
    downloader: &Downloader,
    sheet_client: SheetClient,
    sheet_url: &String,
) -> Result<()> {
    let urls = sheet_client.fetch_urls(&sheet_url).await?;
    let reuslt = downloader.process_urls(&urls).await?;
    Ok(reuslt)
}

/// Processes video URLs from local text files.
///
/// # Processing Steps
/// 1. Reads the input directory
/// 2. Processes each .txt file found
/// 3. Downloads videos from URLs in each file
///
/// # Arguments
/// * `downloader` - Handles video download operations and configuration
///
/// # Errors
/// Returns error if:
/// - Directory reading fails
/// - File reading fails
/// - URL parsing fails
/// - Video downloading fails
async fn process_local_files(downloader: &Downloader) -> Result<()> {
    // Then process local files
    let input_dir = &downloader.config().input_dir;
    let entries = std::fs::read_dir(input_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            println!("Processing file: {:?}", path);
            let urls = read_urls(&path).await?;
            let reuslt = downloader.process_urls(&urls).await?;
            return Ok(reuslt);
        }
    }
    Ok(())
}

/// Reads and validates URLs from a text file.
///
/// # Format
/// - One URL per line
/// - Empty lines are ignored
/// - Lines are trimmed of whitespace
///
/// # Arguments
/// * `path` - Path to the text file containing URLs
///
/// # Returns
/// A vector of validated URLs as strings
///
/// # Errors
/// Returns error if:
/// - File cannot be opened
/// - File reading fails
/// - Line parsing fails
async fn read_urls(path: &PathBuf) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut urls = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            urls.push(trimmed.to_string());
        }
    }

    Ok(urls)
}
