# Tauri v2 Integration Guide

This guide explains how to integrate the neatflix-mpvrs library with a Tauri v2 application.

## Overview

To integrate neatflix-mpvrs with Tauri v2, we recommend creating a separate crate that implements the Tauri plugin interface. This approach keeps the core library focused and avoids unnecessary dependencies.

## Step 1: Create a New Crate

First, create a new crate for the Tauri plugin:

```bash
cargo new --lib neatflix-mpvrs-tauri
cd neatflix-mpvrs-tauri
```

## Step 2: Configure Cargo.toml

Add the necessary dependencies to your Cargo.toml:

```toml
[package]
name = "neatflix-mpvrs-tauri"
version = "0.1.0"
edition = "2021"
description = "Tauri plugin for neatflix-mpvrs"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"

[dependencies]
neatflix-mpvrs = { version = "0.1.0", features = ["async"] }
tauri = { version = "2.0.0-alpha", features = ["api-all"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.28", features = ["full"] }
```

## Step 3: Implement the Plugin

Create the plugin implementation in `src/lib.rs`:

```rust
use neatflix_mpvrs::{AsyncVideoManager, PlaybackOptions, VideoEvent, VideoId, Error};
use std::sync::Arc;
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};
use tokio::sync::Mutex;

// State to hold the video manager
struct VideoManagerState {
    manager: Arc<Mutex<AsyncVideoManager>>,
}

// Initialize the plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("neatflix-mpvrs")
        .setup(|app| {
            // Initialize the video manager
            let manager = AsyncVideoManager::new();
            
            // Store the manager in the state
            app.manage(VideoManagerState {
                manager: Arc::new(Mutex::new(manager)),
            });
            
            // Set up event forwarding
            let app_handle = app.app_handle();
            let manager_state = app.state::<VideoManagerState>();
            let manager_clone = manager_state.manager.clone();
            
            tokio::spawn(async move {
                let manager = manager_clone.lock().await;
                let mut subscription = manager.subscribe().await;
                
                while let Some(event) = subscription.recv().await {
                    // Convert the event to a serializable format
                    let event_json = serde_json::to_value(&event).unwrap();
                    
                    // Emit the event to the frontend
                    app_handle.emit_all("video-event", event_json).ok();
                }
            });
            
            Ok(())
        })
        .build()
}

// Command to play a video
#[tauri::command]
async fn play_video(
    state: State<'_, VideoManagerState>,
    path: String,
    options: Option<PlaybackOptions>,
) -> Result<String, String> {
    let manager = state.manager.lock().await;
    let options = options.unwrap_or_default();
    
    manager.play(path, options)
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

// Command to close a video
#[tauri::command]
async fn close_video(
    state: State<'_, VideoManagerState>,
    id: String,
) -> Result<(), String> {
    let manager = state.manager.lock().await;
    
    // Parse the video ID
    let video_id = id.parse::<uuid::Uuid>()
        .map_err(|e| format!("Invalid video ID: {}", e))?;
    
    // Create a VideoId from the UUID
    let video_id = VideoId(video_id);
    
    manager.close(video_id)
        .await
        .map_err(|e| e.to_string())
}

// Command to close all videos
#[tauri::command]
async fn close_all_videos(
    state: State<'_, VideoManagerState>,
) -> Result<(), String> {
    let manager = state.manager.lock().await;
    
    manager.close_all()
        .await
        .map_err(|e| e.to_string())
}
```

## Step 4: Register the Commands

Update `src/lib.rs` to register the commands:

```rust
// Register the commands
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("neatflix-mpvrs")
        .setup(|app| {
            // ... setup code ...
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play_video,
            close_video,
            close_all_videos,
        ])
        .build()
}
```

## Step 5: Use the Plugin in Your Tauri App

In your Tauri app's `main.rs`, register the plugin:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(neatflix_mpvrs_tauri::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Step 6: Use the Plugin from the Frontend

From your frontend JavaScript/TypeScript code:

```typescript
// Play a video
await invoke('play_video', { 
  path: '/path/to/video.mp4',
  options: {
    startTime: 10.0,
    title: 'My Video',
    reportProgress: true
  }
});

// Listen for video events
await listen('video-event', (event) => {
  const { id, type, position, duration, percent } = event.payload;
  
  if (type === 'Progress') {
    console.log(`Progress: ${position}s / ${duration}s (${percent}%)`);
  } else if (type === 'Ended') {
    console.log('Video ended');
  }
});

// Close a video
await invoke('close_video', { id: videoId });

// Close all videos
await invoke('close_all_videos');
```

## Conclusion

This integration approach keeps the core neatflix-mpvrs library focused on video playback functionality while providing a clean interface for Tauri applications. The separation of concerns makes it easier to maintain both the core library and the Tauri integration.

For more advanced usage, you can extend the plugin with additional commands and events as needed. 