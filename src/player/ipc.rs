use crate::{Error, Result};
use log::{debug, error, warn};
use serde_json::{Value, json};
use std::io::{Write, BufRead, BufReader};
use std::time::{Duration, Instant};
use crate::config::ipc::IpcConfig;

#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

#[cfg(target_family = "windows")]
use std::fs::OpenOptions;
#[cfg(target_family = "windows")]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(target_family = "windows")]
use std::os::windows::io::{FromRawHandle, IntoRawHandle};
#[cfg(target_family = "windows")]
use winapi::um::fileapi::CreateFileA;
#[cfg(target_family = "windows")]
use winapi::um::winbase::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX};
#[cfg(target_family = "windows")]
use winapi::um::winnt::{GENERIC_READ, GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE};
#[cfg(target_family = "windows")]
use std::ffi::CString;
#[cfg(target_family = "windows")]
use std::ptr;
#[cfg(target_family = "windows")]
use std::io;

/// Client for communicating with mpv via JSON IPC.
pub struct MpvIpcClient {
    #[cfg(target_family = "unix")]
    socket: UnixStream,
    
    #[cfg(target_family = "windows")]
    socket: std::fs::File,
    
    request_id: u64,
    connected: bool,
    socket_path: String,
    config: IpcConfig,
    reconnect_attempts: u32,
    last_reconnect_time: Option<Instant>,
    intentionally_closed: bool,
}

impl MpvIpcClient {
    /// Connects to the mpv JSON IPC socket.
    pub fn connect(socket_path: &str) -> Result<Self> {
        Self::connect_with_config(socket_path, IpcConfig::default())
    }
    
    /// Connects to the mpv JSON IPC socket with custom IPC configuration.
    pub fn connect_with_config(socket_path: &str, config: IpcConfig) -> Result<Self> {
        debug!("Connecting to mpv IPC socket: {}", socket_path);
        
        #[cfg(target_family = "unix")]
        {
            match UnixStream::connect(socket_path) {
                Ok(socket) => {
                    debug!("Successfully connected to mpv IPC socket");
                    Ok(Self { 
                        socket, 
                        request_id: 1, 
                        connected: true,
                        socket_path: socket_path.to_string(),
                        config,
                        reconnect_attempts: 0,
                        last_reconnect_time: None,
                        intentionally_closed: false,
                    })
                },
                Err(e) => {
                    error!("Failed to connect to mpv IPC socket: {}", e);
                    Err(Error::Io(e))
                }
            }
        }
        
        #[cfg(target_family = "windows")]
        {
            // Windows named pipe implementation
            let c_socket_path = match CString::new(socket_path) {
                Ok(path) => path,
                Err(e) => {
                    error!("Failed to convert socket path to CString: {}", e);
                    return Err(Error::MpvError(format!("Invalid socket path: {}", e)));
                }
            };
            
            let handle = unsafe {
                CreateFileA(
                    c_socket_path.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    ptr::null_mut(),
                    winapi::um::fileapi::OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED,
                    ptr::null_mut(),
                )
            };
            
            if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let err = io::Error::last_os_error();
                error!("Failed to open named pipe: {}", err);
                return Err(Error::Io(err));
            }
            
            let socket = unsafe { std::fs::File::from_raw_handle(handle as *mut _) };
            debug!("Successfully connected to MPV IPC socket");
            
            Ok(Self {
                socket,
                request_id: 1,
                connected: true,
                socket_path: socket_path.to_string(),
                config,
                reconnect_attempts: 0,
                last_reconnect_time: None,
                intentionally_closed: false,
            })
        }
    }
    
    /// Attempts to reconnect to the mpv socket if disconnected
    fn reconnect(&mut self) -> Result<()> {
        if self.intentionally_closed {
            debug!("Not reconnecting because client was intentionally closed");
            return Err(Error::MpvError("Client was intentionally closed".to_string()));
        }

        // Log the reconnection attempt and current state
        debug!("Attempting to reconnect to mpv IPC socket. Attempt: {}/{}, intentionally_closed: {}", 
               self.reconnect_attempts + 1, 
               self.config.max_reconnect_attempts,
               self.intentionally_closed);

        if self.connected {
            return Ok(());
        }
        
        // Check if we've reached the maximum number of reconnection attempts
        if self.reconnect_attempts >= self.config.max_reconnect_attempts {
            return Err(Error::MpvError(format!(
                "Max reconnection attempts ({}) reached", 
                self.config.max_reconnect_attempts
            )));
        }
        
        // Increment reconnection attempts
        self.reconnect_attempts += 1;
        
        let now = Instant::now();
        
        // If we recently tried to reconnect, wait a bit to avoid hammering the socket
        if let Some(last_time) = self.last_reconnect_time {
            let elapsed = now.duration_since(last_time);
            if elapsed < Duration::from_millis(self.config.reconnect_delay_ms) {
                std::thread::sleep(Duration::from_millis(self.config.reconnect_delay_ms) - elapsed);
            }
        }
        
        self.last_reconnect_time = Some(now);
        
        debug!("Attempting to reconnect to mpv IPC socket (attempt {}/{})", 
              self.reconnect_attempts, self.config.max_reconnect_attempts);
        
        #[cfg(target_family = "unix")]
        {
            match UnixStream::connect(&self.socket_path) {
                Ok(new_socket) => {
                    self.socket = new_socket;
                    self.connected = true;
                    debug!("Successfully reconnected to mpv IPC socket");
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to reconnect to mpv IPC socket: {}", e);
                    Err(Error::Io(e))
                }
            }
        }
        
        #[cfg(target_family = "windows")]
        {
            let socket_path_cstring = std::ffi::CString::new(self.socket_path.clone())
                .map_err(|e| Error::MpvError(format!("Invalid socket path: {}", e)))?;
            
            let handle = unsafe {
                CreateFileA(
                    socket_path_cstring.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null_mut(),
                    winapi::um::fileapi::OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                )
            };
            
            if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let err = std::io::Error::last_os_error();
                error!("Failed to reconnect to mpv IPC socket: {}", err);
                return Err(Error::Io(err));
            }
            
            let new_socket = unsafe { std::fs::File::from_raw_handle(handle as *mut _) };
            self.socket = new_socket;
            self.connected = true;
            debug!("Successfully reconnected to mpv IPC socket");
            Ok(())
        }
    }
    
    /// Resets the reconnection attempts counter after a successful operation
    fn reset_reconnect_attempts(&mut self) {
        if self.reconnect_attempts > 0 {
            debug!("Resetting reconnection attempts counter");
            self.reconnect_attempts = 0;
        }
    }
    
    /// Sends a command to mpv with automatic reconnection if configured.
    pub fn command(&mut self, command: &str, args: &[Value]) -> Result<Value> {
        let result = self.command_internal(command, args);
        
        if let Err(ref e) = result {
            // Check if it's an IO error and auto-reconnect is enabled
            if self.should_reconnect(e) {
                debug!("Command failed, attempting to reconnect and retry");
                match self.reconnect() {
                    Ok(_) => {
                        // Retry the command after successful reconnection
                        return self.command_internal(command, args);
                    },
                    Err(reconnect_err) => {
                        error!("Failed to reconnect: {}", reconnect_err);
                        return Err(reconnect_err);
                    }
                }
            }
        } else {
            // Reset reconnection attempts after successful command
            self.reset_reconnect_attempts();
        }
        
        result
    }
    
    /// Internal implementation of command without reconnection logic
    fn command_internal(&mut self, command: &str, args: &[Value]) -> Result<Value> {
        let id = self.request_id;
        self.request_id += 1;
        
        let mut command_args = vec![Value::String(command.to_string())];
        command_args.extend_from_slice(args);
        
        let request = json!({
            "command": command_args,
            "request_id": id
        });
        
        self.send_request(&request)?;
        self.receive_response(id)
    }
    
    /// Gets a property from mpv with automatic reconnection if configured.
    pub fn get_property(&mut self, property: &str) -> Result<Value> {
        let result = self.get_property_internal(property);
        
        if let Err(ref e) = result {
            if self.should_reconnect(e) {
                debug!("Get property failed, attempting to reconnect and retry");
                match self.reconnect() {
                    Ok(_) => {
                        return self.get_property_internal(property);
                    },
                    Err(reconnect_err) => {
                        error!("Failed to reconnect: {}", reconnect_err);
                        return Err(reconnect_err);
                    }
                }
            }
        } else {
            self.reset_reconnect_attempts();
        }
        
        result
    }
    
    /// Internal implementation of get_property without reconnection logic
    fn get_property_internal(&mut self, property: &str) -> Result<Value> {
        let id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["get_property", property],
            "request_id": id
        });
        
        self.send_request(&request)?;
        self.receive_response(id)
    }
    
    /// Sets a property in mpv with automatic reconnection if configured.
    pub fn set_property(&mut self, property: &str, value: Value) -> Result<Value> {
        let result = self.set_property_internal(property, value.clone());
        
        if let Err(ref e) = result {
            if self.should_reconnect(e) {
                debug!("Set property failed, attempting to reconnect and retry");
                match self.reconnect() {
                    Ok(_) => {
                        return self.set_property_internal(property, value);
                    },
                    Err(reconnect_err) => {
                        error!("Failed to reconnect: {}", reconnect_err);
                        return Err(reconnect_err);
                    }
                }
            }
        } else {
            self.reset_reconnect_attempts();
        }
        
        result
    }
    
    /// Internal implementation of set_property without reconnection logic
    fn set_property_internal(&mut self, property: &str, value: Value) -> Result<Value> {
        let id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["set_property", property, value],
            "request_id": id
        });
        
        self.send_request(&request)?;
        self.receive_response(id)
    }
    
    /// Observes a property in mpv with automatic reconnection if configured.
    pub fn observe_property(&mut self, property: &str) -> Result<u64> {
        let result = self.observe_property_internal(property);
        
        if let Err(ref e) = result {
            if self.should_reconnect(e) {
                debug!("Observe property failed, attempting to reconnect and retry");
                match self.reconnect() {
                    Ok(_) => {
                        return self.observe_property_internal(property);
                    },
                    Err(reconnect_err) => {
                        error!("Failed to reconnect: {}", reconnect_err);
                        return Err(reconnect_err);
                    }
                }
            }
        } else {
            self.reset_reconnect_attempts();
        }
        
        result
    }
    
    /// Internal implementation of observe_property without reconnection logic
    fn observe_property_internal(&mut self, property: &str) -> Result<u64> {
        let id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["observe_property", id, property],
            "request_id": id
        });
        
        self.send_request(&request)?;
        if let Value::Object(response) = self.receive_response(id)? {
            if let Some(Value::String(error)) = response.get("error") {
                if error != "success" {
                    return Err(Error::MpvError(error.clone()));
                }
            }
            
            return Ok(id);
        }
        
        Err(Error::MpvError("Invalid response format".to_string()))
    }
    
    /// Unobserves a property in mpv with automatic reconnection if configured.
    pub fn unobserve_property(&mut self, observe_id: u64) -> Result<Value> {
        let result = self.unobserve_property_internal(observe_id);
        
        if let Err(ref e) = result {
            if self.should_reconnect(e) {
                debug!("Unobserve property failed, attempting to reconnect and retry");
                match self.reconnect() {
                    Ok(_) => {
                        return self.unobserve_property_internal(observe_id);
                    },
                    Err(reconnect_err) => {
                        error!("Failed to reconnect: {}", reconnect_err);
                        return Err(reconnect_err);
                    }
                }
            }
        } else {
            self.reset_reconnect_attempts();
        }
        
        result
    }
    
    /// Internal implementation of unobserve_property without reconnection logic
    fn unobserve_property_internal(&mut self, observe_id: u64) -> Result<Value> {
        let id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["unobserve_property", observe_id],
            "request_id": id
        });
        
        self.send_request(&request)?;
        self.receive_response(id)
    }
    
    /// Checks if we should attempt to reconnect based on the error
    fn should_reconnect(&self, error: &Error) -> bool {
        if self.intentionally_closed {
            debug!("Not reconnecting because client was intentionally closed");
            return false;
        }

        // Check for EOF-related errors that indicate intentional closure
        let is_eof_error = match error {
            Error::MpvError(msg) if msg.contains("End of file") => true,
            Error::MpvError(msg) if msg.contains("property unavailable") && self.connected => {
                // This often happens when mpv is shutting down
                debug!("Detected property unavailable error during connected state, treating as EOF");
                true
            }
            _ => false,
        };

        if is_eof_error {
            debug!("Not reconnecting because EOF-related error detected: {}", error);
            return false;
        }

        if self.reconnect_attempts >= self.config.max_reconnect_attempts {
            debug!("Not reconnecting because max reconnect attempts ({}) reached", 
                   self.config.max_reconnect_attempts);
            return false;
        }

        // Check if auto-reconnect is enabled
        if !self.config.auto_reconnect {
            debug!("Not reconnecting because auto-reconnect is disabled");
            return false;
        }

        // Check if the error is reconnectable
        let reconnectable = match error {
            Error::Io(_) => true,
            Error::MpvError(msg) if msg.contains("Connection refused") => true,
            Error::MpvError(msg) if msg.contains("Broken pipe") => true,
            Error::MpvError(msg) if msg.contains("Connection reset") => true,
            _ => false,
        };

        debug!("Should reconnect for error: {}? {}", error, reconnectable);
        reconnectable
    }
    
    /// Sends a request to mpv with improved error handling
    fn send_request(&mut self, request: &Value) -> Result<()> {
        if !self.connected {
            if self.config.auto_reconnect {
                self.reconnect()?;
            } else {
                return Err(Error::MpvError("Not connected to mpv".to_string()));
            }
        }
        
        let request_str = request.to_string();
        debug!("Sending request: {}", request_str);
        
        #[cfg(target_family = "unix")]
        {
            match self.socket.write_all(format!("{}\n", request_str).as_bytes()) {
                Ok(_) => {
                    debug!("Request sent successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to send request: {}", e);
                    self.connected = false;
                    Err(Error::Io(e))
                }
            }
        }
        
        #[cfg(target_family = "windows")]
        {
            match self.socket.write_all(format!("{}\n", request_str).as_bytes()) {
                Ok(_) => {
                    debug!("Request sent successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to send request: {}", e);
                    self.connected = false;
                    Err(Error::Io(e))
                }
            }
        }
    }
    
    /// Receives a response from mpv with improved error handling and timeout
    fn receive_response(&mut self, request_id: u64) -> Result<Value> {
        if !self.connected {
            if self.config.auto_reconnect {
                self.reconnect()?;
            } else {
                return Err(Error::MpvError("Not connected to mpv".to_string()));
            }
        }
        
        let timeout = Duration::from_millis(self.config.timeout_ms);
        let start_time = Instant::now();
        
        #[cfg(target_family = "unix")]
        let reader = BufReader::new(&self.socket);
        
        #[cfg(target_family = "windows")]
        let reader = BufReader::new(&self.socket);
        
        // Set read timeout if available
        #[cfg(target_family = "unix")]
        {
            self.socket.set_read_timeout(Some(timeout))
                .map_err(|e| Error::Io(e))?;
        }
        
        // Read and parse lines until we find the response with matching request_id
        let mut reader_lines = reader.lines();
        while let Some(line_result) = reader_lines.next() {
            // Check for timeout
            if start_time.elapsed() > timeout {
                return Err(Error::MpvError(format!("Response timeout after {} ms", self.config.timeout_ms)));
            }
            
            match line_result {
                Ok(line) => {
                    debug!("Received response: {}", line);
                    
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    match serde_json::from_str::<Value>(&line) {
                        Ok(Value::Object(resp)) => {
                            // Check if this is a response to our request
                            if let Some(Value::Number(id)) = resp.get("request_id") {
                                if id.as_u64() == Some(request_id) {
                                    return Ok(Value::Object(resp));
                                }
                            }
                            
                            // If it's an event, ignore and continue
                            if resp.contains_key("event") {
                                continue;
                            }
                        },
                        Ok(v) => {
                            debug!("Received unexpected response format: {:?}", v);
                            continue;
                        },
                        Err(e) => {
                            warn!("Failed to parse response: {} - {}", line, e);
                            continue;
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to read response: {}", e);
                    self.connected = false;
                    return Err(Error::Io(e));
                }
            }
        }
        
        // If we reach here, we've exhausted the reader without finding a matching response
        Err(Error::MpvError(format!("No response found for request ID {}", request_id)))
    }
    
    /// Checks if the mpv process is running by sending a simple command
    pub fn is_running(&mut self) -> bool {
        if !self.connected && self.config.auto_reconnect {
            if let Err(e) = self.reconnect() {
                debug!("Failed to reconnect while checking if mpv is running: {}", e);
                return false;
            }
        }
        
        match self.get_property("pid") {
            Ok(_) => true,
            Err(e) => {
                debug!("mpv is not running: {}", e);
                false
            }
        }
    }
    
    /// Returns whether the client is currently connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    /// Explicitly marks the connection as closed
    pub fn close(&mut self) {
        if self.connected {
            debug!("Closing connection to mpv IPC socket");
            self.connected = false;
        }
    }
    
    /// Gets the current playback time in seconds
    pub fn get_time_pos(&mut self) -> Result<f64> {
        match self.get_property("time-pos")? {
            Value::Number(n) => {
                if let Some(pos) = n.as_f64() {
                    Ok(pos)
                } else {
                    Err(Error::MpvError("Invalid time-pos format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid time-pos type".to_string()))
        }
    }
    
    /// Gets the duration of the current media in seconds
    pub fn get_duration(&mut self) -> Result<f64> {
        match self.get_property("duration")? {
            Value::Number(n) => {
                if let Some(duration) = n.as_f64() {
                    Ok(duration)
                } else {
                    Err(Error::MpvError("Invalid duration format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid duration type".to_string()))
        }
    }
    
    /// Gets the current playback position as a percentage (0-100)
    pub fn get_percent_pos(&mut self) -> Result<f64> {
        match self.get_property("percent-pos")? {
            Value::Number(n) => {
                if let Some(percent) = n.as_f64() {
                    Ok(percent)
                } else {
                    Err(Error::MpvError("Invalid percent-pos format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid percent-pos type".to_string()))
        }
    }
    
    /// Gets the current playback speed (1.0 is normal speed)
    pub fn get_speed(&mut self) -> Result<f64> {
        match self.get_property("speed")? {
            Value::Number(n) => {
                if let Some(speed) = n.as_f64() {
                    Ok(speed)
                } else {
                    Err(Error::MpvError("Invalid speed format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid speed type".to_string()))
        }
    }
    
    /// Sets the playback speed (1.0 is normal speed)
    pub fn set_speed(&mut self, speed: f64) -> Result<Value> {
        self.set_property("speed", json!(speed))
    }
    
    /// Gets the current volume level (0-100)
    pub fn get_volume(&mut self) -> Result<f64> {
        match self.get_property("volume")? {
            Value::Number(n) => {
                if let Some(volume) = n.as_f64() {
                    Ok(volume)
                } else {
                    Err(Error::MpvError("Invalid volume format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid volume type".to_string()))
        }
    }
    
    /// Sets the volume level (0-100)
    pub fn set_volume(&mut self, volume: f64) -> Result<Value> {
        self.set_property("volume", json!(volume))
    }
    
    /// Gets the current mute state
    pub fn get_mute(&mut self) -> Result<bool> {
        match self.get_property("mute")? {
            Value::Bool(mute) => Ok(mute),
            _ => Err(Error::MpvError("Invalid mute type".to_string()))
        }
    }
    
    /// Sets the mute state
    pub fn set_mute(&mut self, mute: bool) -> Result<Value> {
        self.set_property("mute", json!(mute))
    }
    
    /// Toggles mute state
    pub fn toggle_mute(&mut self) -> Result<Value> {
        let mute = self.get_mute()?;
        self.set_mute(!mute)
    }
    
    /// Gets the current pause state
    pub fn get_pause(&mut self) -> Result<bool> {
        match self.get_property("pause")? {
            Value::Bool(pause) => Ok(pause),
            _ => Err(Error::MpvError("Invalid pause type".to_string()))
        }
    }
    
    /// Sets the pause state
    pub fn set_pause(&mut self, pause: bool) -> Result<Value> {
        self.set_property("pause", json!(pause))
    }
    
    /// Toggles pause state
    pub fn toggle_pause(&mut self) -> Result<Value> {
        let pause = self.get_pause()?;
        self.set_pause(!pause)
    }
    
    /// Gets the current fullscreen state
    pub fn get_fullscreen(&mut self) -> Result<bool> {
        match self.get_property("fullscreen")? {
            Value::Bool(fullscreen) => Ok(fullscreen),
            _ => Err(Error::MpvError("Invalid fullscreen type".to_string()))
        }
    }
    
    /// Sets the fullscreen state
    pub fn set_fullscreen(&mut self, fullscreen: bool) -> Result<Value> {
        self.set_property("fullscreen", json!(fullscreen))
    }
    
    /// Toggles fullscreen state
    pub fn toggle_fullscreen(&mut self) -> Result<Value> {
        let fullscreen = self.get_fullscreen()?;
        self.set_fullscreen(!fullscreen)
    }
    
    /// Seeks to a specific position in seconds
    pub fn seek(&mut self, position: f64) -> Result<Value> {
        self.command("seek", &[json!(position), json!("absolute")])
    }
    
    /// Seeks to a specific percentage position (0-100)
    pub fn seek_percent(&mut self, percent: f64) -> Result<Value> {
        self.command("seek", &[json!(percent), json!("absolute-percent")])
    }
    
    /// Seeks relative to the current position (positive or negative seconds)
    pub fn seek_relative(&mut self, offset: f64) -> Result<Value> {
        self.command("seek", &[json!(offset), json!("relative")])
    }
    
    /// Gets the chapter list
    pub fn get_chapter_list(&mut self) -> Result<Vec<Value>> {
        match self.get_property("chapter-list")? {
            Value::Array(chapters) => Ok(chapters),
            _ => Err(Error::MpvError("Invalid chapter-list type".to_string()))
        }
    }
    
    /// Gets the current chapter index
    pub fn get_chapter(&mut self) -> Result<i64> {
        match self.get_property("chapter")? {
            Value::Number(n) => {
                if let Some(chapter) = n.as_i64() {
                    Ok(chapter)
                } else {
                    Err(Error::MpvError("Invalid chapter format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid chapter type".to_string()))
        }
    }
    
    /// Sets the current chapter index
    pub fn set_chapter(&mut self, chapter: i64) -> Result<Value> {
        self.set_property("chapter", json!(chapter))
    }
    
    /// Goes to the next chapter
    pub fn next_chapter(&mut self) -> Result<Value> {
        self.command("add", &[json!("chapter"), json!(1)])
    }
    
    /// Goes to the previous chapter
    pub fn prev_chapter(&mut self) -> Result<Value> {
        self.command("add", &[json!("chapter"), json!(-1)])
    }
    
    /// Gets information about the current media
    pub fn get_media_info(&mut self) -> Result<Value> {
        self.get_property("media-title")
    }
    
    /// Gets the current playlist
    pub fn get_playlist(&mut self) -> Result<Vec<Value>> {
        match self.get_property("playlist")? {
            Value::Array(playlist) => Ok(playlist),
            _ => Err(Error::MpvError("Invalid playlist type".to_string()))
        }
    }
    
    /// Gets the current playlist position
    pub fn get_playlist_pos(&mut self) -> Result<i64> {
        match self.get_property("playlist-pos")? {
            Value::Number(n) => {
                if let Some(pos) = n.as_i64() {
                    Ok(pos)
                } else {
                    Err(Error::MpvError("Invalid playlist-pos format".to_string()))
                }
            },
            _ => Err(Error::MpvError("Invalid playlist-pos type".to_string()))
        }
    }
    
    /// Sets the current playlist position
    pub fn set_playlist_pos(&mut self, pos: i64) -> Result<Value> {
        self.set_property("playlist-pos", json!(pos))
    }
    
    /// Goes to the next item in the playlist
    pub fn playlist_next(&mut self) -> Result<Value> {
        self.command("playlist-next", &[])
    }
    
    /// Goes to the previous item in the playlist
    pub fn playlist_prev(&mut self) -> Result<Value> {
        self.command("playlist-prev", &[])
    }
    
    /// Gets the number of audio tracks
    pub fn get_audio_tracks(&mut self) -> Result<Vec<Value>> {
        match self.get_property("track-list")? {
            Value::Array(tracks) => {
                let audio_tracks = tracks.into_iter()
                    .filter(|track| {
                        if let Some(Value::String(type_str)) = track.get("type") {
                            type_str == "audio"
                        } else {
                            false
                        }
                    })
                    .collect();
                Ok(audio_tracks)
            },
            _ => Err(Error::MpvError("Invalid track-list type".to_string()))
        }
    }
    
    /// Gets the number of subtitle tracks
    pub fn get_subtitle_tracks(&mut self) -> Result<Vec<Value>> {
        match self.get_property("track-list")? {
            Value::Array(tracks) => {
                let subtitle_tracks = tracks.into_iter()
                    .filter(|track| {
                        if let Some(Value::String(type_str)) = track.get("type") {
                            type_str == "sub"
                        } else {
                            false
                        }
                    })
                    .collect();
                Ok(subtitle_tracks)
            },
            _ => Err(Error::MpvError("Invalid track-list type".to_string()))
        }
    }
    
    /// Sets the current audio track
    pub fn set_audio_track(&mut self, id: i64) -> Result<Value> {
        self.set_property("aid", json!(id))
    }
    
    /// Sets the current subtitle track
    pub fn set_subtitle_track(&mut self, id: i64) -> Result<Value> {
        self.set_property("sid", json!(id))
    }
    
    /// Disables subtitles
    pub fn disable_subtitles(&mut self) -> Result<Value> {
        self.set_property("sid", json!("no"))
    }
    
    /// Takes a screenshot
    pub fn screenshot(&mut self, include_subtitles: bool) -> Result<Value> {
        let screenshot_type = if include_subtitles { "subtitles" } else { "video" };
        self.command("screenshot", &[json!(screenshot_type)])
    }
    
    /// Quits mpv
    pub fn quit(&mut self) -> Result<Value> {
        self.command("quit", &[])
    }
    
    /// Gets the current playback status (playing, paused, idle)
    pub fn get_playback_status(&mut self) -> Result<String> {
        // First check if we're paused
        match self.get_pause()? {
            true => return Ok("paused".to_string()),
            false => {
                // Check if we're idle or playing
                match self.get_property("idle-active")? {
                    Value::Bool(true) => Ok("idle".to_string()),
                    Value::Bool(false) => Ok("playing".to_string()),
                    _ => Err(Error::MpvError("Invalid idle-active type".to_string()))
                }
            }
        }
    }
    
    /// Marks the client as intentionally closed, preventing reconnection attempts
    pub fn mark_as_intentionally_closed(&mut self) {
        debug!("Marking IPC client as intentionally closed");
        self.intentionally_closed = true;
        self.connected = false;
    }
} 