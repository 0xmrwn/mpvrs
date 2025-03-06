# Tauri Plugin Playa

A Tauri v2 plugin for video playback using mpv. This plugin provides a simple and powerful API for playing videos in Tauri applications by leveraging the mpv media player.

## Features

- Play local video files or streaming URLs with mpv
- Control playback (pause, resume, seek, volume)
- Monitor playback progress and events
- Support for presets optimized for different scenarios (streaming, quality, performance)
- Event-driven API for reacting to playback changes
- Cross-platform support (Windows, macOS, Linux)

## Installation

Add the plugin to your Tauri project by adding these dependencies to your `Cargo.toml`:

```toml
[dependencies]
tauri-plugin-playa = { git = "https://github.com/yourusername/tauri-plugin-playa" }
```

Then register the plugin in your Tauri application:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_playa::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Import and use the plugin in your JavaScript/TypeScript code:

```typescript
import { play, control, close, listPresets } from 'tauri-plugin-playa-api';
```

## Requirements

- mpv must be installed on the user's system
- Compatible with Tauri v2.0 or newer

## API

### JavaScript/TypeScript API

```typescript
// Play a video file or URL
const videoId = await play('/path/to/video.mp4', {
  preset: 'streaming',      // Optional preset name
  startTime: 120,           // Start at 2 minutes (optional)
  volume: 80,               // Set volume to 80% (optional)
  fullscreen: true,         // Start in fullscreen (optional)
});

// Control playback
await control(videoId, 'pause');
await control(videoId, 'resume');
await control(videoId, 'seek', 300);  // Seek to 5 minutes
await control(videoId, 'volume', 50); // Set volume to 50%

// Get information about playback
const info = await getInfo(videoId);
console.log(`Position: ${info.position}/${info.duration} seconds`);

// Close the video
await close(videoId);

// List available presets
const { presets, recommended } = await listPresets();
console.log(`Recommended preset: ${recommended}`);
console.log('Available presets:', presets);
```

### Events

The plugin emits the following events that you can listen to:

```typescript
import { listen } from '@tauri-apps/api/event';

// Listen for playback events
listen('video:started', (event) => {
  console.log(`Video started: ${event.payload.id}`);
});

listen('video:paused', (event) => {
  console.log(`Video paused: ${event.payload.id}`);
});

listen('video:resumed', (event) => {
  console.log(`Video resumed: ${event.payload.id}`);
});

listen('video:ended', (event) => {
  console.log(`Video ended: ${event.payload.id}`);
});

listen('video:closed', (event) => {
  console.log(`Video closed: ${event.payload.id}`);
});

listen('video:error', (event) => {
  console.error(`Video error: ${event.payload.message}`);
});

// Listen for progress updates
listen('video:progress', (event) => {
  const { id, position, duration, percent } = event.payload;
  console.log(`Progress: ${position}/${duration} (${percent * 100}%)`);
});
```

## Presets

The plugin comes with several presets for different playback scenarios:

- `streaming`: Optimized for streaming videos with lower latency
- `quality`: Prioritizes video quality (higher resolution, better scaling)
- `performance`: Optimized for better performance on lower-end devices
- `mobile`: Optimized for mobile devices with touch controls
- `default`: Balanced settings for most use cases

You can specify a preset when playing a video:

```typescript
const videoId = await play('http://example.com/stream.mp4', {
  preset: 'streaming'
});
```

## License

MIT
