use crate::progress::DownloadProgress;
use crate::{config::Config, error::Result};
use futures::stream::{self, StreamExt};
use yt_dlp::fetcher::deps::Libraries;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::instrument;
use yt_dlp::model::Video;
use yt_dlp::Youtube;

/// A downloader that manages concurrent video downloads and processing
///
/// # Fields
/// * `fetcher` - Thread-safe reference to Youtube downloader instance
/// * `semaphore` - Controls concurrent download limits
/// * `config` - Application configuration settings
/// * `active_downloads` - Counter for currently active downloads
pub struct Downloader {
    fetcher: Arc<Youtube>,
    semaphore: Arc<Semaphore>,
    config: Arc<Config>,
    active_downloads: Arc<AtomicUsize>,
}

impl Downloader {
    /// Creates a new `Downloader` instance with the specified configuration
    ///
    /// # Arguments
    /// * `config` - Configuration settings for the downloader
    ///
    /// # Returns
    /// * `Result<Self>` - A new Downloader instance or an error
    ///
    /// # Errors
    /// * If directory creation fails
    /// * If Youtube initialization fails
    #[instrument(skip(config))]
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize directories
        for dir in [&config.output_dir, &config.input_dir, &config.libraries_dir] {
            tokio::fs::create_dir_all(dir).await?;
        }

        let fetcher = Self::initialize_youtube(&config).await?;

        Ok(Self {
            fetcher: Arc::new(fetcher),
            semaphore: Arc::new(Semaphore::new(config.concurrent_downloads)),
            config: Arc::new(config),
            active_downloads: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Initializes the Youtube downloader with required binaries
    ///
    /// # Arguments
    /// * `config` - Configuration containing paths for libraries and output
    ///
    /// # Returns
    /// * `Result<Youtube>` - Initialized Youtube instance or an error
    ///
    /// # Details
    /// Checks for existing yt-dlp and ffmpeg binaries. If not found,
    /// downloads new ones. Otherwise, uses existing binaries and updates the downloader.
    async fn initialize_youtube(config: &Config) -> Result<Youtube> {
        if !config.libraries_dir.join("yt-dlp").exists()
            || !config.libraries_dir.join("ffmpeg").exists()
        {
            let youtube =
                Youtube::with_new_binaries(config.libraries_dir.clone(), config.output_dir.clone())
                    .await?;
            return Ok(youtube);
        }

        let yt_dlp = config.libraries_dir.join("yt-dlp");
        let ffmpeg = config.libraries_dir.join("ffmpeg");
        let libraries = Libraries::new(yt_dlp, ffmpeg);
        let youtube = Youtube::new(libraries, config.output_dir.clone())?;
        youtube.update_downloader().await?;

        Ok(youtube)
    }

    /// Downloads a single video from the given URL
    ///
    /// # Arguments
    /// * `url` - URL of the video to download
    /// * `index` - Position of this video in the download queue
    ///
    /// # Returns
    /// * `Result<()>` - Success or error status
    ///
    /// # Details
    /// Handles the complete download process including:
    /// 1. Fetching video information
    /// 2. Downloading audio and video separately
    /// 3. Combining them into final output
    /// 4. Cleaning up temporary files
    #[instrument(skip(self))]
    async fn download_video(&self, url: &String, index: usize) -> Result<()> {
        let _active = DownloadGuard::new(&self.active_downloads);
        let video = self.fetcher.fetch_video_infos(url.clone()).await?;

        let filenames: FileNames = FileNames {
            audio: format!("audio_{}.mp3", video.id),
            video: format!("video_{}.mp4", video.id),
            name: format!("{}_{}_{}.mp4", index, video.id, video.title),
        };

        self.process_download(&video, &filenames).await?;
        self.cleanup_temp_files(&filenames).await?;

        Ok(())
    }

    /// Removes temporary audio and video files after processing
    ///
    /// # Arguments
    /// * `filenames` - Structure containing paths to temporary files
    ///
    /// # Returns
    /// * `Result<()>` - Success status (errors are logged but not propagated)
    async fn cleanup_temp_files(&self, filenames: &FileNames) -> Result<()> {
        if let Err(e) = std::fs::remove_file(format!("output/{}", &filenames.audio)) {
            eprintln!("Warning: Could not delete temporary audio file: {}", e);
        }
        if let Err(e) = std::fs::remove_file(format!("output/{}", &filenames.video)) {
            eprintln!("Warning: Could not delete temporary video file: {}", e);
        }

        Ok(())
    }

    /// Processes the download and combination of audio and video streams
    ///
    /// # Arguments
    /// * `video` - Video metadata and format information
    /// * `filenames` - Structure containing output file paths
    ///
    /// # Returns
    /// * `Result<()>` - Success or error status
    ///
    /// # Details
    /// 1. Downloads best quality audio if available
    /// 2. Downloads best quality video if available
    /// 3. Combines audio and video into final output file
    async fn process_download(&self, video: &Video, filenames: &FileNames) -> Result<()> {
        if let Some(audio_format) = video.best_audio_format() {
            self.fetcher
                .download_format(&audio_format, &filenames.audio)
                .await?;
        }

        if let Some(video_format) = video.best_video_format() {
            self.fetcher
                .download_format(&video_format, &filenames.video)
                .await?;
        }

        self.fetcher
            .combine_audio_and_video(&filenames.audio, &filenames.video, &filenames.name)
            .await?;

        Ok(())
    }

    /// Processes a list of URLs for concurrent downloading
    ///
    /// # Arguments
    /// * `urls` - Vector of video URLs to process
    ///
    /// # Returns
    /// * `Result<()>` - Overall success or error status
    ///
    /// # Details
    /// * Manages concurrent downloads using a semaphore
    /// * Tracks progress and provides statistics
    /// * Handles errors for individual downloads while continuing with others
    pub async fn process_urls(&self, urls: &Vec<String>) -> Result<()> {
        let total_videos = urls.len();
        println!("Found {} videos to download", total_videos);
        let progress = Arc::new(Mutex::new(DownloadProgress::new(total_videos)));

        let download_tasks = stream::iter(urls.into_iter().enumerate())
            .map(|(index, url)| {
                let progress = Arc::clone(&progress);
                let sem = Arc::clone(&self.semaphore);

                async move {
                    let _permit = sem.acquire().await.unwrap();
                    println!("Starting download for video {}", index + 1);

                    let start = std::time::Instant::now();
                    let result = self.download_video(url, index + 1).await;
                    let duration = start.elapsed();

                    let success = result.is_ok();
                    let mut progress_guard = progress.lock().await;

                    match result {
                        Ok(_) => println!(
                            "Video {} completed in {:.1}s",
                            index + 1,
                            duration.as_secs_f64()
                        ),
                        Err(e) => {
                            let error_msg = e.to_string();
                            eprintln!("Failed to download video {}: {}", index + 1, error_msg);
                            progress_guard.record_failure(url, error_msg);
                        }
                    }
                    progress_guard.update(success);
                }
            })
            .buffer_unordered(10);

        download_tasks.collect::<Vec<_>>().await;

        // Print final statistics and export failures
        let final_progress = progress.lock().await;
        println!("\nDownload Summary:");
        println!(
            "Total time: {:.1}s",
            final_progress.start_time.elapsed().as_secs_f64()
        );
        println!(
            "Successfully downloaded: {}",
            final_progress.completed - final_progress.errors
        );
        println!("Failed downloads: {}", final_progress.errors);

        if let Err(e) = final_progress.export_failures() {
            eprintln!("Failed to export failure report: {}", e);
        }

        Ok(())
    }

    /// Returns a reference to the configuration
    ///
    /// # Returns
    /// * `&Config` - Reference to the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// RAII guard for tracking active downloads
///
/// Automatically increments counter on creation and
/// decrements it when dropped
struct DownloadGuard<'a> {
    counter: &'a AtomicUsize,
}

impl<'a> DownloadGuard<'a> {
    fn new(counter: &'a AtomicUsize) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self { counter }
    }
}

impl<'a> Drop for DownloadGuard<'a> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Structure holding temporary and final filenames for a download
///
/// # Fields
/// * `audio` - Temporary audio file name
/// * `video` - Temporary video file name
/// * `name` - Final output file name
struct FileNames {
    audio: String,
    video: String,
    name: String,
}
