use crate::Result;
use log::{debug, error, warn};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::ipc::MpvIpcClient;

/// Types of events that can be emitted by mpv.
#[derive(Debug, Clone)]
pub enum MpvEvent {
    // Playback state events
    PlaybackStarted,
    PlaybackPaused,
    PlaybackResumed,
    PlaybackCompleted,
    
    // Progress events
    TimePositionChanged(f64),
    PercentPositionChanged(f64),
    
    // Player state events
    VolumeChanged(i32),
    MuteChanged(bool),
    
    // Error events
    PlaybackError(String),
    
    // Process events
    ProcessExited(i32),
    
    // Property change events
    PropertyChanged(String, Value),
}

/// Callback type for mpv events.
pub type EventCallback = Arc<dyn Fn(MpvEvent) + Send + Sync + 'static>;

/// Event listener for mpv events.
pub struct MpvEventListener {
    ipc_client: Arc<Mutex<MpvIpcClient>>,
    callbacks: Arc<Mutex<HashMap<String, Vec<EventCallback>>>>,
    property_observers: Arc<Mutex<HashMap<String, u64>>>,
    running: Arc<Mutex<bool>>,
    poll_thread: Option<JoinHandle<()>>,
}

impl MpvEventListener {
    /// Creates a new event listener.
    pub fn new(ipc_client: MpvIpcClient) -> Self {
        Self {
            ipc_client: Arc::new(Mutex::new(ipc_client)),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            property_observers: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
            poll_thread: None,
        }
    }
    
    /// Subscribes to an event.
    pub fn subscribe<F>(&mut self, event_type: &str, callback: F) -> Result<()>
    where
        F: Fn(MpvEvent) + Send + Sync + 'static,
    {
        debug!("Subscribing to MPV event: {}", event_type);
        
        // Add the callback to the list
        {
            let mut callbacks = self.callbacks.lock().unwrap();
            let event_callbacks = callbacks.entry(event_type.to_string()).or_insert_with(Vec::new);
            event_callbacks.push(Arc::new(callback));
        }
        
        // If this is a property event, set up an observer
        if event_type.starts_with("property:") {
            let property_name = event_type.trim_start_matches("property:");
            self.observe_property(property_name)?;
        }
        
        Ok(())
    }
    
    /// Observes a property in mpv.
    fn observe_property(&mut self, property: &str) -> Result<()> {
        // Check if we're already observing this property
        {
            let property_observers = self.property_observers.lock().unwrap();
            if property_observers.contains_key(property) {
                debug!("Already observing property: {}", property);
                return Ok(());
            }
        }
        
        // Start observing the property
        let observer_id = {
            let mut client = self.ipc_client.lock().unwrap();
            client.observe_property(property)?
        };
        
        {
            let mut property_observers = self.property_observers.lock().unwrap();
            property_observers.insert(property.to_string(), observer_id);
        }
        
        debug!("Started observing property: {} with ID: {}", property, observer_id);
        
        Ok(())
    }
    
    /// Starts listening for events in a background thread.
    pub fn start_listening(&mut self) -> Result<()> {
        if *self.running.lock().unwrap() {
            debug!("Event listener is already running");
            return Ok(());
        }
        
        // Set the running flag
        *self.running.lock().unwrap() = true;
        
        // Clone the necessary data for the thread
        let ipc_client = Arc::clone(&self.ipc_client);
        let callbacks = Arc::clone(&self.callbacks);
        let property_observers = Arc::clone(&self.property_observers);
        let running = Arc::clone(&self.running);
        
        // Start the polling thread
        let handle = thread::spawn(move || {
            debug!("Starting MPV event polling thread");
            
            while *running.lock().unwrap() {
                // Check if mpv is still running
                let is_running = {
                    let mut client = ipc_client.lock().unwrap();
                    client.is_running()
                };
                
                if !is_running {
                    warn!("MPV process has exited, stopping event listener");
                    break;
                }
                
                // Poll for events
                Self::poll_events(&ipc_client, &callbacks, &property_observers);
                
                // Sleep for a short time to avoid busy-waiting
                thread::sleep(Duration::from_millis(100));
            }
            
            debug!("MPV event polling thread stopped");
        });
        
        self.poll_thread = Some(handle);
        debug!("Started MPV event listener");
        
        Ok(())
    }
    
    /// Stops listening for events.
    pub fn stop_listening(&mut self) -> Result<()> {
        if !*self.running.lock().unwrap() {
            debug!("Event listener is not running");
            return Ok(());
        }
        
        // Clear the running flag
        *self.running.lock().unwrap() = false;
        
        // Wait for the polling thread to exit
        if let Some(handle) = self.poll_thread.take() {
            debug!("Waiting for MPV event polling thread to exit");
            if let Err(e) = handle.join() {
                error!("Error joining MPV event polling thread: {:?}", e);
            }
        }
        
        // Unobserve all properties
        let mut client = self.ipc_client.lock().unwrap();
        let property_observers = self.property_observers.lock().unwrap();
        for (property, observer_id) in property_observers.iter() {
            debug!("Unobserving property: {} with ID: {}", property, observer_id);
            if let Err(e) = client.unobserve_property(*observer_id) {
                warn!("Error unobserving property: {}: {}", property, e);
            }
        }
        
        // Clear the property observers
        drop(property_observers);
        let mut property_observers = self.property_observers.lock().unwrap();
        property_observers.clear();
        
        debug!("Stopped MPV event listener");
        
        Ok(())
    }
    
    /// Polls for events from mpv.
    fn poll_events(
        ipc_client: &Arc<Mutex<MpvIpcClient>>,
        callbacks: &Arc<Mutex<HashMap<String, Vec<EventCallback>>>>,
        property_observers: &Arc<Mutex<HashMap<String, u64>>>,
    ) {
        // Check if mpv is still connected and running
        let is_running = {
            let mut client = match ipc_client.lock() {
                Ok(client) => client,
                Err(e) => {
                    error!("Failed to lock IPC client: {:?}", e);
                    return;
                }
            };
            
            client.is_running()
        };
        
        if !is_running {
            debug!("MPV process has exited, notifying subscribers");
            
            // Create a process exited event
            let event = MpvEvent::ProcessExited(0); // We don't have the actual exit code
            
            // Call the callbacks for the process exit event
            let callbacks_lock = match callbacks.lock() {
                Ok(lock) => lock,
                Err(e) => {
                    error!("Failed to lock callbacks: {:?}", e);
                    return;
                }
            };
            
            // Notify subscribers to the process-exited event
            if let Some(callbacks) = callbacks_lock.get("process-exited") {
                for callback in callbacks {
                    callback(event.clone());
                }
            }
            
            // Also notify any general event subscribers
            if let Some(callbacks) = callbacks_lock.get("all") {
                for callback in callbacks {
                    callback(event.clone());
                }
            }
            
            return;
        }
        
        // Continue with property change polling
        let property_observers_lock = match property_observers.lock() {
            Ok(lock) => lock,
            Err(e) => {
                error!("Failed to lock property observers: {:?}", e);
                return;
            }
        };
        
        for (property, _) in property_observers_lock.iter() {
            let property_value = {
                let mut client = match ipc_client.lock() {
                    Ok(client) => client,
                    Err(e) => {
                        error!("Failed to lock IPC client: {:?}", e);
                        continue;
                    }
                };
                
                match client.get_property(property) {
                    Ok(value) => value,
                    Err(e) => {
                        // Only log at debug level since this could happen during normal shutdown
                        debug!("Error getting property {}: {}", property, e);
                        
                        // If this is a connection error, check if mpv is still running
                        if !client.is_connected() {
                            debug!("MPV IPC client disconnected while polling events");
                            return;
                        }
                        
                        continue;
                    }
                }
            };
            
            // Create a property changed event
            let event = MpvEvent::PropertyChanged(property.clone(), property_value.clone());
            
            // Call the callbacks for this property
            let event_type = format!("property:{}", property);
            let callbacks_lock = match callbacks.lock() {
                Ok(lock) => lock,
                Err(e) => {
                    error!("Failed to lock callbacks: {:?}", e);
                    continue;
                }
            };
            
            if let Some(callbacks) = callbacks_lock.get(&event_type) {
                for callback in callbacks {
                    callback(event.clone());
                }
            }
            
            // Also check for specific property events
            match property.as_str() {
                "time-pos" => {
                    if let Some(pos) = property_value.as_f64() {
                        let event = MpvEvent::TimePositionChanged(pos);
                        if let Some(callbacks) = callbacks_lock.get("time-position-changed") {
                            for callback in callbacks {
                                callback(event.clone());
                            }
                        }
                    }
                },
                "percent-pos" => {
                    if let Some(pos) = property_value.as_f64() {
                        let event = MpvEvent::PercentPositionChanged(pos);
                        if let Some(callbacks) = callbacks_lock.get("percent-position-changed") {
                            for callback in callbacks {
                                callback(event.clone());
                            }
                        }
                    }
                },
                "pause" => {
                    if let Some(paused) = property_value.as_bool() {
                        let event = if paused {
                            MpvEvent::PlaybackPaused
                        } else {
                            MpvEvent::PlaybackResumed
                        };
                        
                        let event_name = if paused { "playback-paused" } else { "playback-resumed" };
                        if let Some(callbacks) = callbacks_lock.get(event_name) {
                            for callback in callbacks {
                                callback(event.clone());
                            }
                        }
                    }
                },
                _ => {}
            }
        }
    }
    
    /// Returns whether the event listener is running.
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }
    
    /// Handles the case when mpv has exited
    pub fn handle_process_exit(&mut self) -> Result<()> {
        debug!("Handling MPV process exit");
        
        // Set connected state to false on the IPC client
        {
            let mut client = self.ipc_client.lock().unwrap();
            client.close();
        }
        
        // Stop the event listener if it's running
        if self.is_running() {
            self.stop_listening()?;
        }
        
        // Create a process exited event
        let event = MpvEvent::ProcessExited(0);
        
        // Call the callbacks for the process exit event
        {
            let callbacks = self.callbacks.lock().unwrap();
            if let Some(callbacks) = callbacks.get("process-exited") {
                for callback in callbacks {
                    callback(event.clone());
                }
            }
        }
        
        Ok(())
    }
} 