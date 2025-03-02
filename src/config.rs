use crate::{Error, Result};
use log::{debug, info};
use std::path::PathBuf;

/// Ensures that the MPV configuration directory exists
pub fn ensure_config_dir() -> Result<PathBuf> {
    let config_path = get_mpv_config_path();
    if !config_path.exists() {
        debug!("Creating MPV config directory: {:?}", config_path);
        std::fs::create_dir_all(&config_path)
            .map_err(|e| Error::ConfigError(format!("Failed to create config directory: {}", e)))?;
    }
    Ok(config_path)
}

/// Returns the path to the dedicated mpv configuration directory.
pub fn get_mpv_config_path() -> PathBuf {
    let mut path = crate::get_assets_path();
    path.push("mpv_config");
    path
}

/// Initializes the default mpv configuration
pub fn initialize_default_config() -> Result<()> {
    let config_dir = ensure_config_dir()?;
    let mpv_conf_path = config_dir.join("mpv.conf");
    
    if !mpv_conf_path.exists() {
        info!("Creating default MPV configuration at: {:?}", mpv_conf_path);
        let default_config = concat!(
            "# MPV Configuration for neatflix-mpvrs\n",
            "# Auto-generated default configuration\n\n",
            "# Video output settings\n",
            "vo=libmpv\n",
            "hwdec=auto\n\n",
            "# Audio settings\n",
            "audio-channels=stereo\n",
            "volume=80\n\n",
            "# UI settings\n",
            "osc=yes\n",
            "osd-font-size=30\n",
            "osd-bar=yes\n"
        );
        
        std::fs::write(&mpv_conf_path, default_config)
            .map_err(|e| Error::ConfigError(format!("Failed to write default config: {}", e)))?;
    }
    
    Ok(())
} 