use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, Mutex};
use yt_dlp::fetcher::deps::Libraries;
use yt_dlp::Youtube;
use futures::stream::{self, StreamExt};

// Structure to track download progress
struct DownloadProgress {
    total_videos: usize,
    completed: usize,
    start_time: Instant,
    errors: usize,
}

impl DownloadProgress {
    fn new(total_videos: usize) -> Self {
        Self {
            total_videos,
            completed: 0,
            start_time: Instant::now(),
            errors: 0,
        }
    }

    fn update(&mut self, success: bool) {
        self.completed += 1;
        if !success {
            self.errors += 1;
        }
        self.print_progress();
    }

    fn print_progress(&self) {
        let elapsed = self.start_time.elapsed();
        let avg_time_per_video = if self.completed > 0 {
            elapsed.div_f64(self.completed as f64)
        } else {
            Duration::from_secs(0)
        };

        let remaining_videos = self.total_videos - self.completed;
        let est_remaining_time = avg_time_per_video.mul_f64(remaining_videos as f64);

        println!("Progress: {}/{} videos completed ({:.1}%)", 
            self.completed, 
            self.total_videos,
            (self.completed as f64 / self.total_videos as f64) * 100.0
        );
        println!("Elapsed time: {:.1}s", elapsed.as_secs_f64());
        println!("Estimated time remaining: {:.1}s", est_remaining_time.as_secs_f64());
        println!("Successful: {}, Failed: {}", self.completed - self.errors, self.errors);
        println!("----------------------------------------");
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match first_check().await {
        Ok(fetcher) => {
            let fetcher = Arc::new(fetcher);
            let input_dir = PathBuf::from("input");
            let entries = std::fs::read_dir(input_dir)?;
            let semaphore = Arc::new(Semaphore::new(3));

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
                    println!("Processing file: {:?}", path);
                    let urls = read_urls(&path)?;
                    let total_videos = urls.len();
                    
                    println!("Found {} videos to download", total_videos);
                    let progress = Arc::new(Mutex::new(DownloadProgress::new(total_videos)));

                    let download_tasks = stream::iter(urls.into_iter().enumerate())
                        .map(|(index, url)| {
                            let fetcher = Arc::clone(&fetcher);
                            let sem = Arc::clone(&semaphore);
                            let progress = Arc::clone(&progress);
                            
                            async move {
                                let _permit = sem.acquire().await.unwrap();
                                println!("Starting download for video {}", index + 1);
                                
                                let start = Instant::now();
                                let result = download_video(&fetcher, url.clone(), index).await;
                                let duration = start.elapsed();
                                
                                let success = result.is_ok();
                                progress.lock().await.update(success);
                                
                                match result {
                                    Ok(_) => println!("Video {} completed in {:.1}s", index + 1, duration.as_secs_f64()),
                                    Err(e) => eprintln!("Failed to download video {}: {}", index + 1, e),
                                }
                            }
                        })
                        .buffer_unordered(10);

                    download_tasks.collect::<Vec<_>>().await;
                    
                    // Print final statistics
                    let final_progress = progress.lock().await;
                    println!("\nDownload Summary:");
                    println!("Total time: {:.1}s", final_progress.start_time.elapsed().as_secs_f64());
                    println!("Successfully downloaded: {}", final_progress.completed - final_progress.errors);
                    println!("Failed downloads: {}", final_progress.errors);
                }
            }
        }
        Err(e) => eprintln!("Failed to initialize: {}", e),
    }

    Ok(())
}

async fn download_video(
    fetcher: &Youtube,
    url: String,
    index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let video = fetcher.fetch_video_infos(url.clone()).await?;

    // Extract video ID from URL
    let video_id = url.split('/').last().unwrap_or_default();
    
    // Create filenames using video ID
    let audio_filename = format!("{}.mp3", video_id);
    let video_filename = format!("{}.mp4", video_id);
    let output_filename = format!("ytb_{}_{}.mp4", index, video_id);

    // Download audio
    if let Some(audio_format) = video.best_audio_format() {
        fetcher
            .download_format(&audio_format, &audio_filename)
            .await?;
    }

    // Download video
    if let Some(video_format) = video.best_video_format() {
        fetcher
            .download_format(&video_format, &video_filename)
            .await?;
    }

    // Combine audio and video
    fetcher
        .combine_audio_and_video(&audio_filename, &video_filename, &output_filename)
        .await?;

    // Clean up temporary files from output directory
    if let Err(e) = std::fs::remove_file(format!("output/{}", &audio_filename)) {
        eprintln!("Warning: Could not delete temporary audio file: {}", e);
    }
    if let Err(e) = std::fs::remove_file(format!("output/{}", &video_filename)) {
        eprintln!("Warning: Could not delete temporary video file: {}", e);
    }

    Ok(())
}

fn read_urls(path: &PathBuf) -> io::Result<Vec<String>> {
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

async fn first_check() -> Result<Youtube, yt_dlp::error::Error> {
    let output_dir = PathBuf::from("output");
    let input_dir = PathBuf::from("input");
    let libraries_dir = PathBuf::from("libs");
    let fetcher;

    // Create directories if they don't exist
    for dir in [&output_dir, &input_dir, &libraries_dir] {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
    }

    // Check if any .txt file exists in input directory
    let has_txt_file = std::fs::read_dir(&input_dir)?
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or(false, |ext| ext == "txt")
        });

    // Create videos.txt if no .txt files exist
    if !has_txt_file {
        let videos_file = input_dir.join("videos.txt");
        std::fs::File::create(&videos_file)?;
        println!("Created videos.txt file in input directory");
    }

    if !libraries_dir.join("yt-dlp").exists() || !libraries_dir.join("ffmpeg").exists() {
        fetcher = Youtube::with_new_binaries(libraries_dir.clone(), output_dir).await?;
    } else {
        let youtube = libraries_dir.join("yt-dlp");
        let ffmpeg = libraries_dir.join("ffmpeg");
        let libraries = Libraries::new(youtube, ffmpeg);
        fetcher = Youtube::new(libraries, output_dir)?;
        fetcher.update_downloader().await?;
    }

    Ok(fetcher)
}
