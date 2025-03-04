# neatflix-mpvrs

A Rust video player library using mpv via CLI and JSON IPC.

## Features

- Launch video playback for local files or HTTP streams
- Launch video or stream at specific timestamps
- Close specific streams
- Close all streams
- Listen for progress updates and important playback events

## Usage

### Basic Usage

```rust
use neatflix_mpvrs::{VideoManager, PlaybackOptions, VideoEvent};

// Create a video manager
let mut manager = VideoManager::new();

// Subscribe to video events
let subscription = manager.subscribe();

// Handle events in a separate thread
std::thread::spawn(move || {
    for event in subscription.iter() {
        match event {
            VideoEvent::Progress { id, position, duration, percent } => {
                println!("Progress: {:.1}s / {:.1}s ({:.1}%)", position, duration, percent);
            }
            VideoEvent::Started { id } => println!("Video started"),
            VideoEvent::Paused { id } => println!("Video paused"),
            VideoEvent::Resumed { id } => println!("Video resumed"),
            VideoEvent::Ended { id } => println!("Video ended"),
            VideoEvent::Closed { id } => println!("Video closed"),
            VideoEvent::Error { id, message } => println!("Error: {}", message),
        }
    }
});

// Create playback options
let options = PlaybackOptions {
    start_time: Some(10.0), // Start 10 seconds in
    title: Some("My Video".to_string()),
    ..Default::default()
};

// Play a video
let video_id = manager.play("/path/to/video.mp4", options).unwrap();

// Later, close the video
manager.close(video_id).unwrap();

// Or close all videos
manager.close_all().unwrap();
```

### Async API

The library also provides an async API that can be enabled with the `async` feature:

```rust
use neatflix_mpvrs::{AsyncVideoManager, PlaybackOptions, VideoEvent};

// Create a video manager
let manager = AsyncVideoManager::new();

// Subscribe to video events
let mut subscription = manager.subscribe().await;

// Handle events in a separate task
tokio::spawn(async move {
    while let Some(event) = subscription.recv().await {
        match event {
            VideoEvent::Progress { id, position, duration, percent } => {
                println!("Progress: {:.1}s / {:.1}s ({:.1}%)", position, duration, percent);
            }
            // Handle other events...
        }
    }
});

// Create playback options
let options = PlaybackOptions {
    start_time: Some(10.0),
    title: Some("My Video".to_string()),
    ..Default::default()
};

// Play a video
let video_id = manager.play("/path/to/video.mp4".to_string(), options).await.unwrap();

// Later, close the video
manager.close(video_id).await.unwrap();

// Or close all videos
manager.close_all().await.unwrap();
```

To enable the async API, add the `async` feature to your Cargo.toml:

```toml
[dependencies]
neatflix-mpvrs = { version = "0.1.0", features = ["async"] }
```

### Playback Options

The `PlaybackOptions` struct allows you to customize video playback:

```rust
let options = PlaybackOptions {
    // Start time in seconds
    start_time: Some(30.0),
    
    // Preset to use (default, high-quality, low-latency, etc.)
    preset: Some("high-quality".to_string()),
    
    // Additional mpv arguments
    extra_args: vec!["--volume=50".to_string()],
    
    // Window title
    title: Some("Custom Title".to_string()),
    
    // Whether to enable progress reporting
    report_progress: true,
    
    // Progress reporting interval in milliseconds
    progress_interval_ms: Some(500),
};
```

### Event Handling

The library provides a simple event subscription system:

```rust
// Subscribe to events
let subscription = manager.subscribe();

// Process events
for event in subscription.iter() {
    // Handle events
}

// Or process events without blocking
match subscription.try_recv() {
    Ok(event) => {
        // Handle event
    }
    Err(std::sync::mpsc::TryRecvError::Empty) => {
        // No events available
    }
    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
        // Channel disconnected
    }
}
```

## Integration with External Applications

This library is designed to be easily integrated with external Rust applications. For Tauri v2 integration, a separate crate is recommended to avoid unnecessary dependencies.

### Tauri v2 Integration

For Tauri v2 integration, create a separate crate that depends on neatflix-mpvrs and implements the Tauri plugin interface. This approach keeps the core library focused and avoids unnecessary dependencies.

Example structure for a Tauri plugin:

```
neatflix-mpvrs-tauri/
  ├── Cargo.toml
  └── src/
      ├── lib.rs        # Tauri plugin implementation
      └── commands.rs   # Tauri commands that wrap the VideoManager API
```

## Requirements

- mpv must be installed and available in the PATH
- Rust 1.56 or later

## License

MIT