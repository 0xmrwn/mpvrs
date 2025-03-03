use log::error;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub mod config;
mod player;
pub mod presets;

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

/// Spawns mpv with the specified media file or URL and a preset.
/// The preset will override default configurations, and extra_args can override preset settings.
pub fn spawn_mpv_with_preset(file_or_url: &str, preset_name: Option<&str>, extra_args: &[&str]) -> Result<()> {
    player::spawn_mpv_with_preset(file_or_url, preset_name, extra_args)
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

// Re-export preset API functions
pub use presets::{
    list_available_presets,
    get_preset_details,
    get_recommended_preset,
};

/// Apply a preset to get mpv arguments
pub fn apply_preset(preset_name: &str) -> Result<Vec<String>> {
    presets::apply_preset(preset_name)
} 