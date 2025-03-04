use crate::{Error, Result};
use log::{debug, error, warn};
use serde_json::{Value, json};
use std::io::{Write, BufRead, BufReader};

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
}

impl MpvIpcClient {
    /// Connects to the mpv JSON IPC socket.
    pub fn connect(socket_path: &str) -> Result<Self> {
        debug!("Connecting to MPV IPC socket at: {}", socket_path);
        
        #[cfg(target_family = "unix")]
        {
            match UnixStream::connect(socket_path) {
                Ok(socket) => {
                    debug!("Successfully connected to MPV IPC socket");
                    Ok(Self { 
                        socket,
                        request_id: 1,
                        connected: true,
                    })
                },
                Err(e) => {
                    error!("Failed to connect to MPV IPC socket: {}", e);
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
            })
        }
    }
    
    /// Sends a command to mpv.
    pub fn command(&mut self, command: &str, args: &[Value]) -> Result<Value> {
        let request_id = self.request_id;
        self.request_id += 1;
        
        let mut command_args = vec![Value::String(command.to_string())];
        command_args.extend_from_slice(args);
        
        let request = json!({
            "command": command_args,
            "request_id": request_id,
        });
        
        self.send_request(&request)?;
        self.receive_response(request_id)
    }
    
    /// Gets a property from mpv.
    pub fn get_property(&mut self, property: &str) -> Result<Value> {
        debug!("Getting MPV property: {}", property);
        let request_id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["get_property", property],
            "request_id": request_id,
        });
        
        self.send_request(&request)?;
        self.receive_response(request_id)
    }
    
    /// Sets a property in mpv.
    pub fn set_property(&mut self, property: &str, value: Value) -> Result<Value> {
        debug!("Setting MPV property: {} to {:?}", property, value);
        let request_id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["set_property", property, value],
            "request_id": request_id,
        });
        
        self.send_request(&request)?;
        self.receive_response(request_id)
    }
    
    /// Observes a property in mpv.
    pub fn observe_property(&mut self, property: &str) -> Result<u64> {
        debug!("Observing MPV property: {}", property);
        let request_id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["observe_property", request_id, property],
            "request_id": request_id,
        });
        
        self.send_request(&request)?;
        let response = self.receive_response(request_id)?;
        
        // Check if the response indicates success
        if response.get("error").and_then(Value::as_str) == Some("success") {
            Ok(request_id)
        } else {
            Err(Error::MpvError(format!("Failed to observe property: {:?}", response)))
        }
    }
    
    /// Unobserves a property in mpv.
    pub fn unobserve_property(&mut self, observe_id: u64) -> Result<Value> {
        debug!("Unobserving MPV property with ID: {}", observe_id);
        let request_id = self.request_id;
        self.request_id += 1;
        
        let request = json!({
            "command": ["unobserve_property", observe_id],
            "request_id": request_id,
        });
        
        self.send_request(&request)?;
        self.receive_response(request_id)
    }
    
    /// Sends a raw request to mpv.
    fn send_request(&mut self, request: &Value) -> Result<()> {
        if !self.connected {
            return Err(Error::MpvError("Not connected to MPV".to_string()));
        }
        
        let request_str = format!("{}\n", serde_json::to_string(request)?);
        debug!("Sending MPV IPC request: {}", request_str.trim());
        
        #[cfg(target_family = "unix")]
        {
            match self.socket.write_all(request_str.as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to write to MPV IPC socket: {}", e);
                    self.connected = false;
                    Err(Error::Io(e))
                }
            }
        }
        
        #[cfg(target_family = "windows")]
        {
            match self.socket.write_all(request_str.as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to write to MPV IPC socket: {}", e);
                    self.connected = false;
                    Err(Error::Io(e))
                }
            }
        }
    }
    
    /// Receives a response from mpv.
    fn receive_response(&mut self, request_id: u64) -> Result<Value> {
        if !self.connected {
            return Err(Error::MpvError("Not connected to MPV".to_string()));
        }
        
        #[cfg(target_family = "unix")]
        {
            // Create a BufReader that borrows the socket mutably
            let mut reader = BufReader::new(&mut self.socket);
            let mut response_str = String::new();
            
            // Read a line from the socket
            match reader.read_line(&mut response_str) {
                Ok(0) => {
                    // EOF reached, mpv has disconnected
                    debug!("MPV socket closed (EOF)");
                    self.connected = false;
                    return Err(Error::MpvError("MPV socket closed".to_string()));
                },
                Ok(_) => {
                    debug!("Received MPV IPC response: {}", response_str.trim());
                },
                Err(e) => {
                    error!("Failed to read from MPV IPC socket: {}", e);
                    self.connected = false;
                    return Err(Error::Io(e));
                }
            }
            
            // Parse the response
            let response: Value = serde_json::from_str(&response_str)
                .map_err(|e| {
                    error!("Failed to parse MPV IPC response: {}", e);
                    Error::MpvError(format!("Invalid JSON response: {}", e))
                })?;
            
            // Check if the response is for the correct request
            if let Some(resp_id) = response.get("request_id").and_then(Value::as_u64) {
                if resp_id != request_id {
                    warn!("Received response for different request ID: expected {}, got {}", request_id, resp_id);
                }
            }
            
            Ok(response)
        }
        
        #[cfg(target_family = "windows")]
        {
            // Windows implementation similar to Unix
            // (omitted for brevity, but would follow the same pattern)
            unimplemented!("Windows implementation omitted for brevity");
        }
    }
    
    /// Checks if mpv is still running by sending a simple command.
    pub fn is_running(&mut self) -> bool {
        if !self.connected {
            return false;
        }
        
        match self.get_property("pid") {
            Ok(_) => true,
            Err(_) => {
                self.connected = false;
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
        self.connected = false;
    }
} 