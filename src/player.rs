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
        path.push("script-opts");
        path
    };
    
    if !script_opts_dir.exists() {
        warn!("Script options directory not found at: {}", script_opts_dir.display());
        return Ok(());
    }
    
    let config_files = vec![
        "uosc.conf"
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
pub fn spawn_mpv(file_or_url: &str, extra_args: &[&str]) -> crate::Result<()> {
    info!("Launching mpv for media: {}", file_or_url);
    
    // Validate configuration files before launching mpv
    if let Err(e) = validate_config_files() {
        warn!("Error validating config files: {}. Continuing anyway...", e);
    }

    // Build the argument list with key options:
    // - Use uosc instead of standard OSC
    // - Use our custom config directory
    
    // Create String values that will live for the entire function
    let config_dir_path = get_mpv_config_path();
    let config_dir_str = config_dir_path.to_str().unwrap().to_string();
    debug!("MPV config directory: {}", config_dir_str);
    
    // Build args using mpv's --option=value format
    let mut args = Vec::<String>::new();
    
    // Add verbose flag to see script loading errors
    args.push("--msg-level=all=v".to_string());
    
    // Add configuration directory
    args.push(format!("--config-dir={}", config_dir_str));
    
    // Ensure uosc is used instead of the standard OSC
    args.push("--osc=no".to_string());
    args.push("--osd-bar=no".to_string());
    args.push("--border=no".to_string());
    
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
            Err(crate::Error::Io(e))
        }
    }
}

/// Spawns mpv with the specified media file or URL and a preset.
/// The preset will override default configurations, and extra_args can override preset settings.
pub fn spawn_mpv_with_preset(file_or_url: &str, preset_name: Option<&str>, extra_args: &[&str]) -> crate::Result<()> {
    info!("Launching mpv for media: {} with preset: {:?}", file_or_url, preset_name);
    
    // Validate configuration files before launching mpv
    if let Err(e) = validate_config_files() {
        warn!("Error validating config files: {}. Continuing anyway...", e);
    }

    // Create String values that will live for the entire function
    let config_dir_path = get_mpv_config_path();
    let config_dir_str = config_dir_path.to_str().unwrap().to_string();
    debug!("MPV config directory: {}", config_dir_str);
    
    // Build args using mpv's --option=value format
    let mut args = Vec::<String>::new();
    
    // Add verbose flag to see script loading errors
    args.push("--msg-level=all=v".to_string());
    
    // Add configuration directory
    args.push(format!("--config-dir={}", config_dir_str));
    
    // Ensure uosc is used instead of the standard OSC
    args.push("--osc=no".to_string());
    args.push("--osd-bar=no".to_string());
    args.push("--border=no".to_string());
    
    // If a preset is specified, add its configuration options
    if let Some(preset_name) = preset_name {
        match crate::presets::apply_preset(preset_name) {
            Ok(preset_args) => {
                debug!("Applying preset '{}' with args: {:?}", preset_name, preset_args);
                args.extend(preset_args);
            },
            Err(e) => {
                warn!("Failed to apply preset '{}': {}. Continuing with default settings.", preset_name, e);
            }
        }
    }
    
    // Add any extra arguments (these will override preset settings)
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
            Err(crate::Error::Io(e))
        }
    }
}

/// Returns the path to the dedicated mpv configuration directory.
fn get_mpv_config_path() -> PathBuf {
    crate::get_assets_path()
} 