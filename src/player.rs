use crate::{Error, Result};
use log::{debug, error, info, warn};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

/// Validates configuration files to ensure they don't have common issues
/// like trailing spaces after boolean values
fn validate_config_files() -> Result<()> {
    let script_opts_dir = {
        let mut path = crate::get_assets_path();
        path.push("mpv_config");
        path.push("script-opts");
        path
    };
    
    if !script_opts_dir.exists() {
        warn!("Script options directory not found at: {}", script_opts_dir.display());
        return Ok(());
    }
    
    let config_files = vec![
        "autoload.conf",
        "mpv_thumbnail_script.conf",
        "osc.conf",
        "tethys.conf"
    ];
    
    for file_name in config_files {
        let file_path = script_opts_dir.join(file_name);
        if !file_path.exists() {
            debug!("Config file not found, skipping: {}", file_path.display());
            continue;
        }
        
        debug!("Validating config file: {}", file_path.display());
        validate_config_file(&file_path)?;
    }
    
    Ok(())
}

/// Validates a single configuration file for common issues
fn validate_config_file(file_path: &PathBuf) -> Result<()> {
    let file = fs::File::open(file_path)
        .map_err(|e| Error::ConfigError(format!("Failed to open config file {}: {}", file_path.display(), e)))?;
    
    let reader = BufReader::new(file);
    let mut fixed_lines = Vec::new();
    let mut needs_fixing = false;
    
    for line in reader.lines() {
        let line = line.map_err(|e| Error::ConfigError(format!("Failed to read line from {}: {}", file_path.display(), e)))?;
        
        // Check for boolean values with trailing spaces
        if line.contains("=yes ") || line.contains("=no ") {
            let fixed_line = line.replace("=yes ", "=yes").replace("=no ", "=no");
            fixed_lines.push(fixed_line);
            needs_fixing = true;
            warn!("Fixed trailing space in boolean value in {}: '{}'", file_path.display(), line);
        } else {
            fixed_lines.push(line);
        }
    }
    
    // Write back the fixed file if needed
    if needs_fixing {
        fs::write(file_path, fixed_lines.join("\n"))
            .map_err(|e| Error::ConfigError(format!("Failed to write fixed config file {}: {}", file_path.display(), e)))?;
        info!("Fixed configuration file: {}", file_path.display());
    }
    
    Ok(())
}

/// Spawns mpv with the specified media file or URL.
/// Additional command-line arguments can override default configurations.
pub fn spawn_mpv(file_or_url: &str, extra_args: &[&str]) -> Result<()> {
    info!("Launching mpv for media: {}", file_or_url);
    
    // Validate configuration files before launching mpv
    if let Err(e) = validate_config_files() {
        warn!("Error validating config files: {}. Continuing anyway...", e);
    }

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
        return Err(Error::ConfigError(format!("Tethys OSC script not found at: {}", tethys_script_str)));
    }
    if !thumbnail_script_path.exists() {
        error!("Thumbnail script not found at: {}", thumbnail_script_str);
        return Err(Error::ConfigError(format!("Thumbnail script not found at: {}", thumbnail_script_str)));
    }
    if !autoload_script_path.exists() {
        error!("Autoload script not found at: {}", autoload_script_str);
        return Err(Error::ConfigError(format!("Autoload script not found at: {}", autoload_script_str)));
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