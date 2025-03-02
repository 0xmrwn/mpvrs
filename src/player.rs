use crate::{Error, Result};
use log::{debug, error, info};
use std::path::PathBuf;
use std::process::Command;

/// Spawns mpv with the specified media file or URL.
/// Additional command-line arguments can override default configurations.
pub fn spawn_mpv(file_or_url: &str, extra_args: &[&str]) -> Result<()> {
    info!("Launching mpv for media: {}", file_or_url);

    // Build the argument list with key options:
    // - Disable default OSC as it's handled by our config
    // - Load the Tethys OSC script, thumbnail script, and autoload script
    // - Use our custom config directory
    
    // Create String values that will live for the entire function
    let config_dir_path = get_mpv_config_path();
    let config_dir_str = config_dir_path.to_str().unwrap().to_string();
    debug!("MPV config directory: {}", config_dir_str);
    
    let tethys_script_path = get_tethys_script_path();
    let tethys_script_str = tethys_script_path.to_str().unwrap().to_string();
    debug!("Tethys OSC script path: {}", tethys_script_str);
    
    let thumbnail_script_path = get_thumbnail_script_path();
    let thumbnail_script_str = thumbnail_script_path.to_str().unwrap().to_string();
    debug!("Thumbnail script path: {}", thumbnail_script_str);
    
    let autoload_script_path = get_autoload_script_path();
    let autoload_script_str = autoload_script_path.to_str().unwrap().to_string();
    debug!("Autoload script path: {}", autoload_script_str);
    
    // Verify that all script files exist
    if !tethys_script_path.exists() {
        error!("Tethys OSC script not found at: {}", tethys_script_str);
    }
    if !thumbnail_script_path.exists() {
        error!("Thumbnail script not found at: {}", thumbnail_script_str);
    }
    if !autoload_script_path.exists() {
        error!("Autoload script not found at: {}", autoload_script_str);
    }
    
    // Build args using mpv's --option=value format
    let mut args = Vec::<String>::new();
    
    // Add verbose flag to see script loading errors
    args.push("--msg-level=all=v".to_string());
    
    // Add configuration directory
    args.push(format!("--config-dir={}", config_dir_str));
    
    // Add scripts manually so they're loaded in the right order
    args.push(format!("--script={}", tethys_script_str));
    args.push(format!("--script={}", thumbnail_script_str));
    args.push(format!("--script={}", autoload_script_str));
    
    // Add any extra arguments
    for arg in extra_args {
        args.push(arg.to_string());
    }
    
    // Add the file or URL
    args.push(file_or_url.to_string());

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

/// Returns the path to the Tethys OSC script
fn get_tethys_script_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("lua");
    path.push("osc_tethys.lua");
    path
}

/// Returns the path to the thumbnail script
fn get_thumbnail_script_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("lua");
    path.push("mpv_thumbnail_script_server.lua");
    path
}

/// Returns the path to the autoload script
fn get_autoload_script_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("lua");
    path.push("autoload.lua");
    path
}

/// Returns the path to the dedicated mpv configuration directory.
fn get_mpv_config_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("mpv_config");
    path
} 