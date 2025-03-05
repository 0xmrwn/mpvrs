use log::{debug, warn, error};
use std::io;
use std::path::PathBuf;
use std::process::Child;
use thiserror::Error;
use std::{thread, time::Duration};
use config::ipc::{DEFAULT_MAX_RECONNECT_ATTEMPTS, DEFAULT_RECONNECT_DELAY_MS};

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
    player::process::spawn_mpv_legacy(file_or_url, extra_args)
}

/// Spawns mpv with the specified media file or URL and a preset.
/// The preset will override default configurations, and extra_args can override preset settings.
/// Returns the process handle and socket path for IPC communication.
pub fn spawn_mpv_with_preset(file_or_url: &str, preset_name: Option<&str>, extra_args: &[&str]) -> Result<(Child, String)> {
    player::process::spawn_mpv_with_preset_legacy(file_or_url, preset_name, extra_args)
}

/// Spawns mpv with the specified media file or URL and options.
/// Returns the process handle and socket path for IPC communication.
pub fn spawn_mpv_with_options(file_or_url: &str, options: &player::process::SpawnOptions) -> Result<(Child, String)> {
    player::process::spawn_mpv(file_or_url, options)
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
    let max_attempts = DEFAULT_MAX_RECONNECT_ATTEMPTS;
    let delay_ms = DEFAULT_RECONNECT_DELAY_MS;
    
    for attempt in 0..max_attempts {
        debug!("Attempting to connect to mpv IPC socket (attempt {}/{})", attempt + 1, max_attempts);
        
        match player::ipc::MpvIpcClient::connect(socket_path) {
            Ok(client) => return Ok(client),
            Err(e) => {
                if attempt < max_attempts - 1 {
                    warn!("Failed to connect to IPC socket (attempt {}/{}), retrying in {}ms: {}", 
                          attempt + 1, max_attempts, delay_ms, e);
                    thread::sleep(Duration::from_millis(delay_ms));
                } else {
                    error!("Failed to connect to IPC socket after {} attempts: {}", max_attempts, e);
                    return Err(e);
                }
            }
        }
    }
    
    // This should not be reachable due to the return in the error case above
    unreachable!("Loop exited without returning");
}

// Re-export event system
pub use player::events::{MpvEvent, MpvEventListener};

/// Creates a new event listener for the specified IPC client.
pub fn create_event_listener(ipc_client: player::ipc::MpvIpcClient) -> player::events::MpvEventListener {
    player::events::MpvEventListener::new(ipc_client)
}

// Re-export plugin API
pub use plugin::{VideoManager, VideoId, PlaybackOptions, VideoEvent, EventSubscription, WindowOptions};
pub use player::process::SpawnOptions; 