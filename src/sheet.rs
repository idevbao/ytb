use crate::error::Result;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct SheetRow {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "")]
    pub _extra: Vec<String>,
}

/// Google Sheets integration for URL sourcing.
///
/// Provides functionality to fetch video URLs from published Google Sheets,
/// handling authentication, parsing, and error recovery.

/// Client for interacting with Google Sheets.
///
/// Handles:
/// - Sheet URL parsing
/// - CSV data fetching
/// - Response parsing
/// - Error handling
///
/// # Examples
///
/// ```no_run
/// use application::SheetClient;
///
/// async fn example() {
///     let client = SheetClient::new();
///     let urls = client.fetch_urls("https://docs.google.com/...").await;
/// }
/// ```
pub struct SheetClient {
    client: reqwest::Client,
}

impl SheetClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_urls(&self, sheet_url: &str) -> Result<Vec<String>> {
        let url = Url::parse(sheet_url)?;
        let segments: Vec<&str> = url.path_segments().unwrap().collect();
        let sheet_id = segments.get(2).ok_or("error")?;
        let csv_url = format!(
            "https://docs.google.com/spreadsheets/d/{}/export?format=csv&gid=0",
            sheet_id
        );

        println!("Fetching data from URL: {}", csv_url);

        // Fetch CSV data with error handling
        let response = self
            .client
            .get(csv_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch sheet: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch sheet. Status: {}", response.status()).into());
        }

        let content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response content: {}", e))?;

        println!("Received content length: {} bytes", content.len());

        // Simple parsing: split by lines and take non-empty URLs
        let urls: Vec<String> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect();

        if urls.is_empty() {
            return Err("No valid URLs found in the sheet".into());
        }

        println!("Successfully loaded {} URLs from sheet", urls.len());

        // Print first few URLs for debugging
        for (i, url) in urls.iter().take(3).enumerate() {
            println!("URL {}: {}", i + 1, url);
        }

        Ok(urls)
    }
}
