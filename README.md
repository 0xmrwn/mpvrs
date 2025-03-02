# Neatflix MPV Player

A macOS-focused Rust video player using mpv with a Netflix-like UI. This library provides a simple way to spawn mpv with a custom configuration and a custom OSC (On-Screen Controller) that mimics popular streaming services.

## Features

- Spawn mpv in its own window with a Netflix-like UI
- Custom Lua OSC for playback controls
- Dedicated configuration for stability
- Simple API for media playback
- macOS-focused (with future plans for Windows and Linux)

## Prerequisites

- mpv must be installed on your system
- Rust (latest stable version)

### macOS Installation

```bash
brew install mpv
```

## Usage

### Command Line Usage

```bash
neatflix-mpvrs <media_file_or_url> [extra_mpv_args...]
```

Example:
```bash
neatflix-mpvrs ~/Videos/movie.mp4 --fullscreen --volume=70
```

### Library Usage

```rust
use neatflix_mpvrs::{config, setup_logging};

fn main() {
    // Initialize logging
    setup_logging();
    
    // Initialize default configuration
    config::initialize_default_config().unwrap();
    
    // Play a media file or URL
    let media = "path/to/your/media.mp4";
    let extra_args = ["--fullscreen", "--volume=70"];
    
    neatflix_mpvrs::spawn_mpv(media, &extra_args).unwrap();
}
```

## Configuration

The player uses a dedicated mpv configuration located in the `assets/mpv_config` directory. You can override these settings by passing extra arguments when spawning mpv.

## Custom OSC

The player uses a customized version of the mpv-osc-modern Lua script to provide a Netflix-like UI for playback controls. The custom OSC is located in the `assets/lua` directory.

## Building

```bash
cargo build --release
```

## License

MIT License