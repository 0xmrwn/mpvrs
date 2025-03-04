use neatflix_mpvrs::{self, MpvEvent};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    // Initialize logging
    neatflix_mpvrs::setup_logging();

    // Example media file or URL
    let media = "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4";
    
    // Optional extra arguments to override defaults
    let extra_args = ["--volume=50"];

    // Spawn mpv and get the IPC socket path
    let (mut process, socket_path) = match neatflix_mpvrs::spawn_mpv(media, &extra_args) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error launching video player: {}", e);
            return;
        }
    };
    
    println!("MPV process spawned with PID: {:?}", process.id());
    println!("IPC socket path: {}", socket_path);
    
    // Give mpv some time to start up
    thread::sleep(Duration::from_secs(2));
    
    // Track if mpv is running so we can shut down gracefully
    let mpv_running = Arc::new(AtomicBool::new(true));
    
    // Connect to the IPC socket
    let ipc_client = match neatflix_mpvrs::connect_ipc(&socket_path) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error connecting to mpv IPC: {}", e);
            return;
        }
    };
    
    println!("Connected to MPV IPC socket");
    
    // Create an event listener
    let mut event_listener = neatflix_mpvrs::create_event_listener(ipc_client);
    
    // Clone running flag for the event listener
    let mpv_running_clone = Arc::clone(&mpv_running);
    
    // Subscribe to process exit event
    if let Err(e) = event_listener.subscribe("process-exited", move |event| {
        if let MpvEvent::ProcessExited(_) = event {
            println!("MPV process has exited");
            mpv_running_clone.store(false, Ordering::SeqCst);
        }
    }) {
        eprintln!("Error subscribing to process exit events: {}", e);
    }
    
    // Subscribe to time position changes 
    // Note: Will only be triggered approximately once per minute
    if let Err(e) = event_listener.subscribe("time-position-changed", |event| {
        if let MpvEvent::TimePositionChanged(position) = event {
            println!("Playback progress: {:.2} seconds", position);
            
            // Here you could save the position for resume features
            // For example:
            // save_playback_position(current_media_id, position);
        }
    }) {
        eprintln!("Error subscribing to time position events: {}", e);
    }
    
    // Subscribe to playback state changes
    if let Err(e) = event_listener.subscribe("playback-paused", |_| {
        println!("Playback paused");
    }) {
        eprintln!("Error subscribing to pause events: {}", e);
    }
    
    if let Err(e) = event_listener.subscribe("playback-resumed", |_| {
        println!("Playback resumed");
    }) {
        eprintln!("Error subscribing to resume events: {}", e);
    }
    
    // Subscribe to playback completion
    if let Err(e) = event_listener.subscribe("playback-completed", |_| {
        println!("Playback completed!");
        
        // Here you could mark the media as watched
        // For example:
        // mark_media_as_watched(current_media_id);
    }) {
        eprintln!("Error subscribing to playback completion events: {}", e);
    }
    
    // Subscribe to all events
    if let Err(e) = event_listener.subscribe("all", |event| {
        println!("Event received: {:?}", event);
    }) {
        eprintln!("Error subscribing to all events: {}", e);
    }
    
    // The event listener needs to be shared with the main thread
    let event_listener = Arc::new(Mutex::new(event_listener));
    
    // Start listening for events
    {
        let mut listener = event_listener.lock().unwrap();
        if let Err(e) = listener.start_listening() {
            eprintln!("Error starting event listener: {}", e);
            return;
        }
    }
    
    println!("Event listener started");
    
    // Get a new IPC client for control
    let control_client = match neatflix_mpvrs::connect_ipc(&socket_path) {
        Ok(client) => Arc::new(Mutex::new(client)),
        Err(e) => {
            eprintln!("Error connecting to mpv IPC for control: {}", e);
            return;
        }
    };
    
    // Example: Pause playback after 5 seconds
    thread::spawn({
        let control_client = Arc::clone(&control_client);
        let mpv_running = Arc::clone(&mpv_running);
        
        move || {
            // Wait for 5 seconds
            thread::sleep(Duration::from_secs(5));
            
            // Check if mpv is still running
            if !mpv_running.load(Ordering::SeqCst) {
                println!("MPV process has already exited, not sending pause command");
                return;
            }
            
            // Pause playback
            println!("Pausing playback...");
            let mut client = match control_client.lock() {
                Ok(client) => client,
                Err(_) => return,
            };
            
            if let Err(e) = client.set_property("pause", json!(true)) {
                eprintln!("Error pausing playback: {}", e);
            }
            
            // Wait for 2 seconds
            thread::sleep(Duration::from_secs(2));
            
            // Check if mpv is still running
            if !mpv_running.load(Ordering::SeqCst) {
                println!("MPV process has already exited, not sending resume command");
                return;
            }
            
            // Resume playback
            println!("Resuming playback...");
            if let Err(e) = client.set_property("pause", json!(false)) {
                eprintln!("Error resuming playback: {}", e);
            }
        }
    });
    
    // Wait for the process to exit
    // This can happen either through our command or if the user closes mpv
    if let Ok(status) = process.wait() {
        println!("MPV process exited with status: {:?}", status);
        mpv_running.store(false, Ordering::SeqCst);
        
        // Clean up the event listener
        let mut listener = match event_listener.lock() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("Failed to get event listener lock: {:?}", e);
                return;
            }
        };
        
        // Handle the process exit in the event listener
        if let Err(e) = listener.handle_process_exit() {
            eprintln!("Error handling process exit in event listener: {}", e);
        }
    }
    
    println!("Example completed");
} 