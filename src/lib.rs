use log::error;
use std::io;
use std::path::PathBuf;
use std::process::Child;
use thiserror::Error;

pub mod config;
mod player;
pub mod presets;
pub mod plugin;

/// Error type for the neatflix-mpvrs library
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("MPV error: {0}")]
    MpvError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Initializes logging for the library.
pub fn setup_logging() {
    env_logger::init();
}

/// Spawns mpv with the specified media file or URL.
/// Additional command-line arguments can override default configurations.
/// Returns the process handle and socket path for IPC communication.
pub fn spawn_mpv(file_or_url: &str, extra_args: &[&str]) -> Result<(Child, String)> {
    player::process::spawn_mpv(file_or_url, extra_args)
}

/// Spawns mpv with the specified media file or URL and a preset.
/// The preset will override default configurations, and extra_args can override preset settings.
/// Returns the process handle and socket path for IPC communication.
pub fn spawn_mpv_with_preset(file_or_url: &str, preset_name: Option<&str>, extra_args: &[&str]) -> Result<(Child, String)> {
    player::process::spawn_mpv_with_preset(file_or_url, preset_name, extra_args)
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

// Re-export IPC client
pub use player::ipc::MpvIpcClient;

/// Creates a new IPC client connected to the specified socket path.
pub fn connect_ipc(socket_path: &str) -> Result<player::ipc::MpvIpcClient> {
    player::ipc::MpvIpcClient::connect(socket_path)
}

// Re-export event system
pub use player::events::{MpvEvent, MpvEventListener};

/// Creates a new event listener for the specified IPC client.
pub fn create_event_listener(ipc_client: player::ipc::MpvIpcClient) -> player::events::MpvEventListener {
    player::events::MpvEventListener::new(ipc_client)
}

// Re-export plugin API
pub use plugin::{VideoManager, VideoId, PlaybackOptions, VideoEvent, EventSubscription}; 