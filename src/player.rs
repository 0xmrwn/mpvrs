use crate::{Error, Result};
use log::{debug, error, info};
use std::path::PathBuf;
use std::process::Command;

/// Spawns mpv with the specified media file or URL.
/// Additional command-line arguments can override default configurations.
pub fn spawn_mpv(file_or_url: &str, extra_args: &[&str]) -> Result<()> {
    info!("Launching mpv for media: {}", file_or_url);

    // Build the argument list with key options:
    // - Enable the OSC for controls.
    // - Load the custom Lua OSC script.
    // - Load the dedicated mpv configuration file.
    let script_path = format!("--script={}", get_lua_theme_path().to_string_lossy());
    let config_dir_path = format!("--config-dir={}", get_mpv_config_path().to_string_lossy());
    
    let mut args = vec![
        "--osc=yes",
        &script_path,
        &config_dir_path,
    ];
    
    args.extend_from_slice(extra_args);
    args.push(file_or_url);

    debug!("MPV arguments: {:?}", args);

    // Spawn mpv asynchronously. For development, rely on the system-installed mpv.
    match Command::new("mpv").args(&args).spawn() {
        Ok(child) => {
            debug!("MPV process spawned with PID: {:?}", child.id());
            Ok(())
        }
        Err(e) => {
            error!("Failed to launch mpv: {}", e);
            Err(Error::Io(e))
        }
    }
}

/// Returns the path to the custom Lua OSC theme.
fn get_lua_theme_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("lua");
    path.push("osc-tethys.lua");
    path
}

/// Returns the path to the dedicated mpv configuration directory.
fn get_mpv_config_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("mpv_config");
    path
} 