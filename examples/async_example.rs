use neatflix_mpvrs::{VideoManager, PlaybackOptions, VideoEvent};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    // Initialize logging
    neatflix_mpvrs::setup_logging();

    // Create a video manager
    let manager = VideoManager::new();
    
    // Flag to indicate when playback has ended or video has been closed
    let playback_ended = Arc::new(Mutex::new(false));
    let playback_ended_clone = playback_ended.clone();
    
    // Subscribe to video events
    let mut subscription = manager.subscribe().await;
    
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
        start_time: Some(10.0), // Start 10 seconds in
        title: Some("Example Video".to_string()),
        ..Default::default()
    };
    
    // Play a video (replace with your own video file)
    let video_id = match manager.play("http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4".to_string(), options).await {
        Ok(id) => {
            println!("Started video with ID: {}", id.to_string());
            id
        }
        Err(e) => {
            eprintln!("Error starting video: {}", e);
            return;
        }
    };
    
    // Wait for either 30 seconds or until the video ends naturally
    let timeout_duration = tokio::time::Duration::from_secs(30);
    let start_time = tokio::time::Instant::now();
    
    while !*playback_ended_clone.lock().unwrap() {
        // Check if we've exceeded the timeout
        if start_time.elapsed() >= timeout_duration {
            println!("Reached 30 second timeout, closing video");
            break;
        }
        
        // Check every 500ms
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    // Close the video
    if let Err(e) = manager.close(video_id).await {
        eprintln!("Error closing video: {}", e);
    }
    
    // Close all videos to ensure cleanup
    if let Err(e) = manager.close_all().await {
        eprintln!("Error closing all videos: {}", e);
    }
    
    // Signal that we're done by dropping the event task
    drop(event_task);
    
    println!("Example application completed successfully");
} 