use log::{error, info};
use neatflix_mpvrs::{config, setup_logging};
use std::process::Command;

fn check_mpv_installed() -> bool {
    match Command::new("which").arg("mpv").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn main() {
    // Initialize logging
    setup_logging();
    info!("neatflix-mpvrs integration example v{}", neatflix_mpvrs::version());
    
    // Check if mpv is installed
    if !check_mpv_installed() {
        eprintln!("Error: mpv is not installed or not in your PATH.");
        eprintln!("Please install mpv before using neatflix-mpvrs.");
        eprintln!("On macOS, you can install it with: brew install mpv");
        std::process::exit(1);
    }
    
    // Initialize default configuration
    if let Err(e) = config::initialize_default_config() {
        error!("Failed to initialize configuration: {}", e);
        std::process::exit(1);
    }
    
    // Example media file or URL - replace with your own file
    let media = "https://upload.wikimedia.org/wikipedia/commons/transcoded/f/f1/Sintel_movie_4K.webm/Sintel_movie_4K.webm.1080p.vp9.webm";
    
    // Optional extra arguments to override defaults
    let extra_args = ["--volume=70", "--fullscreen"];

    info!("Playing demo media: {}", media);
    if let Err(e) = neatflix_mpvrs::spawn_mpv(media, &extra_args) {
        error!("Error launching video player: {}", e);
        std::process::exit(1);
    }
} 