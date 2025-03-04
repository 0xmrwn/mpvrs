use neatflix_mpvrs::{VideoManager, PlaybackOptions, VideoEvent};

#[tokio::main]
async fn main() {
    // Initialize logging
    neatflix_mpvrs::setup_logging();

    // Create a video manager
    let manager = VideoManager::new();
    
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
                }
                VideoEvent::Closed { id } => {
                    println!("Video {} closed", id.to_string());
                }
                VideoEvent::Error { id, message } => {
                    println!("Video {} error: {}", id.to_string(), message);
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
    let video_id = match manager.play("path/to/your/video.mp4".to_string(), options).await {
        Ok(id) => {
            println!("Started video with ID: {}", id.to_string());
            id
        }
        Err(e) => {
            eprintln!("Error starting video: {}", e);
            return;
        }
    };
    
    // Wait for 30 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    
    // Close the video
    if let Err(e) = manager.close(video_id).await {
        eprintln!("Error closing video: {}", e);
    }
    
    // Wait for the event task to finish
    let _ = event_task.await;
} 