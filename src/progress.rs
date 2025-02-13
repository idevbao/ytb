use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, Instant};

/// Progress tracking and reporting functionality.
///
/// Provides mechanisms to track and display download progress,
/// including completion rates, time estimates, and error counts.

/// Tracks and reports progress for batch video downloads.
///
/// Maintains statistics about ongoing downloads including:
/// - Total number of videos
/// - Completed downloads
/// - Error counts
/// - Time estimates
///
/// # Examples
///
/// ```
/// use application::DownloadProgress;
///
/// let progress = DownloadProgress::new(10);
/// progress.update(true); // Update with successful download
/// ```
pub struct DownloadProgress {
    pub total_videos: usize,
    pub completed: usize,
    pub start_time: Instant,
    pub errors: usize,
    failed_urls: Vec<(String, String)>, // (URL, error message)
}

impl DownloadProgress {
    pub fn new(total_videos: usize) -> Self {
        Self {
            total_videos,
            completed: 0,
            start_time: Instant::now(),
            errors: 0,
            failed_urls: Vec::new(),
        }
    }

    pub fn update(&mut self, success: bool) {
        self.completed += 1;
        if !success {
            self.errors += 1;
        }
        self.print_progress();
    }

    pub fn print_progress(&self) {
        let elapsed = self.start_time.elapsed();
        let avg_time_per_video = if self.completed > 0 {
            elapsed.div_f64(self.completed as f64)
        } else {
            Duration::from_secs(0)
        };

        let remaining_videos = self.total_videos - self.completed;
        let est_remaining_time = avg_time_per_video.mul_f64(remaining_videos as f64);

        println!(
            "Progress: {}/{} videos completed ({:.1}%)",
            self.completed,
            self.total_videos,
            (self.completed as f64 / self.total_videos as f64) * 100.0
        );
        println!("Elapsed time: {:.1}s", elapsed.as_secs_f64());
        println!(
            "Estimated time remaining: {:.1}s",
            est_remaining_time.as_secs_f64()
        );
        println!(
            "Successful: {}, Failed: {}",
            self.completed - self.errors,
            self.errors
        );
        println!("----------------------------------------");
    }

    pub fn record_failure(&mut self, url: &String, error: String) {
        self.failed_urls.push((url.to_string(), error));
    }

    /// Exports failed download information to a file
    ///
    /// Creates or appends to 'output/failed.txt' with details of each failed download
    pub fn export_failures(&self) -> std::io::Result<()> {
        if self.failed_urls.is_empty() {
            return Ok(());
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("output/failed.txt")?;

        let mut writer = std::io::BufWriter::new(file);

        writeln!(
            writer,
            "\n=== Failed Downloads Report {} ===",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;

        for (url, error) in &self.failed_urls {
            writeln!(writer, "URL: {}", url)?;
            writeln!(writer, "Error: {}", error)?;
            writeln!(writer, "---")?;
        }

        writer.flush()?;
        Ok(())
    }
}
