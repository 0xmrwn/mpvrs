# Tauri v2 Integration Guide for neatflix-mpvrs

This guide explains how to integrate the neatflix-mpvrs library with a Tauri v2 application, focusing on creating a plugin that leverages the library's video playback and window control capabilities.

## Overview

To integrate neatflix-mpvrs with Tauri v2, we'll create a Tauri plugin that wraps the core library functionalities. This approach keeps the core library focused on video playback while providing a clean interface for Tauri applications.

## Step 1: Initialize a New Plugin Project

Use the Tauri CLI to create a new plugin project:

```bash
npx @tauri-apps/cli plugin new neatflix-player
cd tauri-plugin-neatflix-player
```

## Step 2: Configure Dependencies

Update your `Cargo.toml` to include neatflix-mpvrs:

```toml
[package]
name = "tauri-plugin-neatflix-player"
version = "0.1.0"
edition = "2021"
description = "Tauri plugin for neatflix-mpvrs video player"
license = "MIT"

[dependencies]
neatflix-mpvrs = "0.1.0"
tauri = { version = "2.0.0-alpha", features = ["api-all"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.28", features = ["full"] }
thiserror = "1.0"
```

## Step 3: Define Command Permissions

Create permission files to allow access to your plugin commands:

```toml
# permissions/default.toml
"$schema" = "schemas/schema.json"

[default]
description = "Allows basic video playback functionality"
permissions = ["allow-play-video", "allow-get-progress"]
```

```toml
# permissions/playback.toml
"$schema" = "schemas/schema.json"

[[permission]]
identifier = "allow-play-video"
description = "Allows playing videos"
commands.allow = ["play_video"]

[[permission]]
identifier = "allow-close-video"
description = "Allows closing videos"
commands.allow = ["close_video"]

[[permission]]
identifier = "allow-get-progress"
description = "Allows retrieving video playback progress"
commands.allow = ["get_progress"]
```

## Step 4: Define Plugin State and Error Types

Create the necessary types in `src/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::Runtime;

// Plugin state to hold the VideoManager
pub struct NeatflixPlayerState<R: Runtime> {
    pub manager: Arc<Mutex<neatflix_mpvrs::VideoManager>>,
    pub app: tauri::AppHandle<R>,
}

// Window configuration struct for frontend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowConfig {
    pub borderless: bool,
    pub position: Option<(i32, i32)>,
    pub size: Option<(u32, u32)>,
    pub always_on_top: bool,
    pub opacity: Option<f32>,
    pub start_hidden: bool,
}

// PlaybackOptions for frontend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybackOptions {
    pub start_time: Option<f64>,
    pub preset: Option<String>,
    pub extra_args: Vec<String>,
    pub title: Option<String>,
    pub report_progress: bool,
    pub progress_interval_ms: Option<u64>,
    pub window: Option<WindowConfig>,
}
```

## Step 5: Define Plugin Commands

Create your commands in `src/commands.rs`:

```rust
use crate::models::{NeatflixPlayerState, PlaybackOptions, WindowConfig};
use neatflix_mpvrs::{VideoId, WindowOptions};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, AppHandle, Runtime, State, Window};

// Converts frontend WindowConfig to library WindowOptions
fn convert_window_config(config: &WindowConfig) -> WindowOptions {
    WindowOptions {
        borderless: config.borderless,
        position: config.position,
        size: config.size,
        always_on_top: config.always_on_top,
        opacity: config.opacity,
        start_hidden: config.start_hidden,
    }
}

// Play video command
#[command]
pub async fn play_video<R: Runtime>(
    state: State<'_, NeatflixPlayerState<R>>,
    path: String,
    options: Option<PlaybackOptions>,
) -> Result<String, String> {
    let state_guard = state.inner();
    let manager = state_guard.manager.lock().unwrap();
    
    // Convert frontend options to library options
    let options = match options {
        Some(opts) => {
            let mut lib_options = neatflix_mpvrs::PlaybackOptions {
                start_time: opts.start_time,
                preset: opts.preset,
                extra_args: opts.extra_args,
                title: opts.title,
                report_progress: opts.report_progress,
                progress_interval_ms: opts.progress_interval_ms,
                window: None,
            };
            
            // Convert window options if provided
            if let Some(window) = opts.window {
                lib_options.window = Some(convert_window_config(&window));
            }
            
            lib_options
        }
        None => neatflix_mpvrs::PlaybackOptions::default(),
    };
    
    // Play the video
    manager.play(path, options)
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

// Close video command
#[command]
pub async fn close_video<R: Runtime>(
    state: State<'_, NeatflixPlayerState<R>>,
    id: String,
) -> Result<(), String> {
    // Implementation details
    Ok(())
}

// Update window properties command
#[command]
pub async fn update_window<R: Runtime>(
    state: State<'_, NeatflixPlayerState<R>>,
    id: String,
    window_config: WindowConfig,
) -> Result<(), String> {
    // Implementation details
    Ok(())
}
```

## Step 6: Implement the Plugin

Create your plugin in `src/lib.rs`:

```rust
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};

mod commands;
mod models;

use commands::*;
use models::*;
use neatflix_mpvrs::{VideoEvent, VideoManager};
use std::sync::{Arc, Mutex};

// Extension trait for easy access to the plugin
pub trait NeatflixPlayerExt<R: Runtime> {
    fn neatflix_player(&self) -> &NeatflixPlayerState<R>;
}

impl<R: Runtime, T: Manager<R>> NeatflixPlayerExt<R> for T {
    fn neatflix_player(&self) -> &NeatflixPlayerState<R> {
        self.state::<NeatflixPlayerState<R>>().inner()
    }
}

/// Initialize the plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("neatflix-player")
        .setup(|app, _| {
            // Initialize the video manager
            let manager = VideoManager::new();
            
            // Store the manager in the state
            let state = NeatflixPlayerState {
                manager: Arc::new(Mutex::new(manager)),
                app: app.clone(),
            };
            
            app.manage(state);
            
            // Set up event forwarding from native player to Tauri
            let app_handle = app.clone();
            setup_event_forwarding(app_handle);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play_video,
            close_video,
            update_window,
        ])
        .build()
}

// Set up event forwarding from native player to Tauri
fn setup_event_forwarding<R: Runtime>(app_handle: AppHandle<R>) {
    // Implementation details for event forwarding
}
```

## Step 7: Create TypeScript API

Create TypeScript bindings in `guest-js/index.ts`:

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Window configuration type
interface WindowConfig {
  borderless?: boolean;
  position?: [number, number];
  size?: [number, number];
  alwaysOnTop?: boolean;
  opacity?: number;
  startHidden?: boolean;
}

// Playback options type
interface PlaybackOptions {
  startTime?: number;
  preset?: string;
  extraArgs?: string[];
  title?: string;
  reportProgress?: boolean;
  progressIntervalMs?: number;
  window?: WindowConfig;
}

// Video event types
type VideoEventType = 
  | 'progress'
  | 'started'
  | 'paused'
  | 'resumed'
  | 'ended'
  | 'closed'
  | 'error';

// Play a video
export async function playVideo(
  path: string, 
  options?: PlaybackOptions
): Promise<string> {
  return await invoke('plugin:neatflix-player|play_video', {
    path,
    options
  });
}

// Close a video
export async function closeVideo(id: string): Promise<void> {
  await invoke('plugin:neatflix-player|close_video', { id });
}

// Update window properties
export async function updateWindow(
  id: string, 
  config: WindowConfig
): Promise<void> {
  await invoke('plugin:neatflix-player|update_window', {
    id,
    windowConfig: config
  });
}
```

## Step 8: Example Usage in a Tauri App

### Register the Plugin

In your Tauri app's `src-tauri/src/main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_neatflix_player::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Use the Plugin from the Frontend

```typescript
import { playVideo, closeVideo, updateWindow } from 'tauri-plugin-neatflix-player-api';
import { listen } from '@tauri-apps/api/event';

// Play a local video in a customized window
async function playLocalVideo() {
  try {
    const videoId = await playVideo('/path/to/video.mp4', {
      startTime: 30, // Start 30 seconds in
      title: 'My Video Player',
      window: {
        borderless: true,
        size: [800, 450],
        alwaysOnTop: true,
        opacity: 0.95
      }
    });
    
    console.log('Video playing with ID:', videoId);
    return videoId;
  } catch (error) {
    console.error('Failed to play video:', error);
  }
}

// Play an HTTP stream
async function playStreamingVideo() {
  try {
    const videoId = await playVideo('https://example.com/stream.mp4', {
      preset: 'low-latency', // Use predefined low-latency preset
      reportProgress: true,
      progressIntervalMs: 500 // Get updates every 500ms
    });
    
    // Listen for video events
    const unlistenFunc = await listen('video-event', (event) => {
      const { type, id, position, duration, percent } = event.payload;
      
      switch (type) {
        case 'progress':
          updateProgressBar(percent);
          break;
        case 'paused':
          showPausedIndicator();
          break;
        case 'ended':
          showPlaybackEnded();
          closeVideo(id);
          break;
      }
    });
    
    return videoId;
  } catch (error) {
    console.error('Failed to play stream:', error);
  }
}

// Update window properties after video is playing
async function togglePictureInPicture(videoId, enablePiP) {
  try {
    if (enablePiP) {
      // Make a small floating window
      await updateWindow(videoId, {
        size: [320, 180],
        position: [window.screen.width - 340, 20],
        alwaysOnTop: true,
        borderless: true
      });
    } else {
      // Restore to normal window
      await updateWindow(videoId, {
        size: [800, 450],
        position: [100, 100],
        alwaysOnTop: false
      });
    }
  } catch (error) {
    console.error('Failed to update window:', error);
  }
}
```

## Advanced Integration Examples

### Creating a Transparent Player Overlay

You can create a transparent floating video player that overlays your Tauri application:

```typescript
async function createOverlayPlayer(videoPath) {
  // Get the main window position and size
  const { x, y, width, height } = await getMainWindowBounds();
  
  // Calculate a position that overlays the main window
  const videoId = await playVideo(videoPath, {
    window: {
      borderless: true,
      position: [x + 20, y + 20],
      size: [width - 40, height / 3],
      opacity: 0.9,  // Semi-transparent
      alwaysOnTop: true
    }
  });
  
  return videoId;
}
```

### Adapting Player to Window Changes

Listen for window resize events and update the video player accordingly:

```typescript
// In your Tauri app frontend
import { listen } from '@tauri-apps/api/event';
import { getCurrent } from '@tauri-apps/api/window';

async function setupAdaptivePlayer(videoId) {
  const mainWindow = getCurrent();
  
  // Listen for window resize events
  await listen('tauri://resize', async () => {
    const { position, size } = await mainWindow.outerPosition();
    
    // Update video player window
    await updateWindow(videoId, {
      position: [position.x, position.y + 30], // Just below title bar
      size: [size.width, size.height * 0.7]
    });
  });
}
```

## Conclusion

This integration approach leverages the neatflix-mpvrs library's window control capabilities to create a flexible video playback solution for Tauri applications. The plugin architecture allows clean separation of concerns while providing a simple API for frontend developers.

By utilizing the window integration features, you can create a variety of video player experiences:
- Standalone video windows with custom appearance
- Picture-in-picture floating players
- Transparent overlay players
- Adaptive players that respond to window changes

For production applications, consider implementing additional features such as:
- Proper error handling with user-friendly messages
- Automatic reconnection for streaming sources
- Performance optimizations for different device capabilities
- Accessibility considerations for media controls

Remember that window integration behavior may vary slightly across platforms (macOS, Windows, Linux), so testing on all target platforms is recommended.
