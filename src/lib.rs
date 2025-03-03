use log::error;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub mod config;
mod player;

/// Error type for the neatflix-mpvrs library
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("MPV error: {0}")]
    MpvError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Initializes logging for the library.
pub fn setup_logging() {
    env_logger::init();
}

/// Spawns mpv with the specified media file or URL.
/// Additional command-line arguments can override default configurations.
pub fn spawn_mpv(file_or_url: &str, extra_args: &[&str]) -> Result<()> {
    player::spawn_mpv(file_or_url, extra_args)
}

/// Returns the path to the mpv_config directory
pub fn get_assets_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("mpv_config");
    path
}

/// Returns the version of the library
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
} 