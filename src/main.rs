use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use yt_dlp::fetcher::deps::Libraries;
use yt_dlp::Youtube;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match first_check().await {
        Ok(fetcher) => {
            // Read URLs from any .txt file in input directory
            let input_dir = PathBuf::from("input");
            let entries = std::fs::read_dir(input_dir)?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
                    println!("Processing file: {:?}", path);
                    let urls = read_urls(&path)?;

                    for (index, url) in urls.iter().enumerate() {
                        println!("Processing video {}/{}: {}", index + 1, urls.len(), url);

                        match download_video(&fetcher, url.to_string(), index).await {
                            Ok(_) => println!("Successfully downloaded video {}", index + 1),
                            Err(e) => eprintln!("Failed to download video {}: {}", index + 1, e),
                        }
                    }
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
    if let Err(e) = std::fs::remove_file( format!("output/{}",&audio_filename)) {
        eprintln!("Warning: Could not delete temporary audio file: {}", e);
    }
    if let Err(e) = std::fs::remove_file(format!("output/{}",&video_filename)) {
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
