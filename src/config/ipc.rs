use crate::Result;
use log::{debug, info};
use std::path::PathBuf;
use std::fs;

/// Default timeout for IPC connections in milliseconds
pub const DEFAULT_IPC_TIMEOUT_MS: u64 = 5000;

/// Default polling interval for IPC events in milliseconds
pub const DEFAULT_IPC_POLL_INTERVAL_MS: u64 = 100;

/// IPC configuration options
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Timeout for IPC connections in milliseconds
    pub timeout_ms: u64,
    
    /// Polling interval for IPC events in milliseconds
    pub poll_interval_ms: u64,
    
    /// Whether to automatically reconnect on connection loss
    pub auto_reconnect: bool,
    
    /// Maximum number of reconnection attempts
    pub max_reconnect_attempts: u32,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_IPC_TIMEOUT_MS,
            poll_interval_ms: DEFAULT_IPC_POLL_INTERVAL_MS,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
        }
    }
}

/// Ensures the IPC socket directory exists and is writable
pub fn ensure_ipc_socket_dir() -> Result<PathBuf> {
    #[cfg(target_family = "unix")]
    {
        // On Unix, we use /tmp which should already exist and be writable
        let socket_dir = PathBuf::from("/tmp");
        debug!("Using IPC socket directory: {}", socket_dir.display());
        Ok(socket_dir)
    }
    
    #[cfg(target_family = "windows")]
    {
        // On Windows, we use the temp directory
        let socket_dir = std::env::temp_dir();
        debug!("Using IPC socket directory: {}", socket_dir.display());
        Ok(socket_dir)
    }
}

/// Cleans up old IPC sockets that might have been left behind
pub fn cleanup_old_ipc_sockets() -> Result<()> {
    #[cfg(target_family = "unix")]
    {
        let socket_dir = ensure_ipc_socket_dir()?;
        
        // Find and remove old mpv sockets
        if let Ok(entries) = fs::read_dir(socket_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.starts_with("mpv-socket-") {
                            if let Err(e) = fs::remove_file(&path) {
                                debug!("Failed to remove old IPC socket {}: {}", path.display(), e);
                            } else {
                                info!("Removed old IPC socket: {}", path.display());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // No cleanup needed on Windows as named pipes are managed by the OS
    
    Ok(())
} 