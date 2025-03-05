use neatflix_mpvrs::{VideoManager, PlaybackOptions, VideoEvent};
use std::env;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    // Initialize logging
    neatflix_mpvrs::setup_logging();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <video_path> [start_time] [preset]", args[0]);
        return;
    }

    // Parse arguments
    let video_path = &args[1];
    let start_time = args.get(2).and_then(|s| s.parse::<f64>().ok());
    let preset = args.get(3).map(|s| s.to_string());

    // Create a video manager
    let manager = VideoManager::new();
    
    // Subscribe to video events
    let mut subscription = manager.subscribe().await;
    
    // Flag to indicate when playback has ended or video has been closed
    let playback_ended = Arc::new(Mutex::new(false));
    let playback_ended_clone = playback_ended.clone();
    
    // Start a task to handle events
    let event_task = tokio::spawn(async move {
        while let Some(event) = subscription.recv().await {
            match event {
                VideoEvent::Progress { id, position, duration, percent } => {
                    println!(
                        "Video {} progress: {:.1}s / {:.1}s ({:.1}%)",
                        id.to_string(),
                        position,
                        duration,
                        percent
                    );
                }
                VideoEvent::Started { id } => {
                    println!("Video {} started", id.to_string());
                }
                VideoEvent::Paused { id } => {
                    println!("Video {} paused", id.to_string());
                }
                VideoEvent::Resumed { id } => {
                    println!("Video {} resumed", id.to_string());
                }
                VideoEvent::Ended { id } => {
                    println!("Video {} ended", id.to_string());
                    // Set the flag to indicate playback has ended
                    if let Ok(mut ended) = playback_ended.lock() {
                        *ended = true;
                    }
                }
                VideoEvent::Closed { id } => {
                    println!("Video {} closed", id.to_string());
                    // Set the flag to indicate video has been closed
                    if let Ok(mut ended) = playback_ended.lock() {
                        *ended = true;
                    }
                }
                VideoEvent::Error { id, message } => {
                    println!("Video {} error: {}", id.to_string(), message);
                    // Also set the flag on error
                    if let Ok(mut ended) = playback_ended.lock() {
                        *ended = true;
                    }
                }
            }
        }
    });
    
    // Create playback options
    let options = PlaybackOptions {
        start_time,
        preset,
        title: Some("Example Video".to_string()),
        ..Default::default()
    };
    
    // Play the video
    let video_id = match manager.play(video_path.to_string(), options).await {
        Ok(id) => {
            println!("Started video with ID: {}", id.to_string());
            id
        }
        Err(e) => {
            eprintln!("Error starting video: {}", e);
            return;
        }
    };
    
    // Wait for the video to end or be closed, checking the flag periodically
    while !*playback_ended_clone.lock().unwrap() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    // Ensure the video is properly closed if it hasn't been already
    let _ = manager.close(video_id).await;
    
    // Close the manager to ensure all resources are released
    let _ = manager.close_all().await;
    
    // Signal to the event task that we're done by dropping the subscription
    drop(event_task);
    
    println!("Example application completed successfully");
} 