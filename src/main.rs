use log::{error, info, debug};
use neatflix_mpvrs::{setup_logging, MpvEvent, VideoEvent, PlaybackOptions, VideoManager};
use std::env;
use std::process::Command;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

fn check_mpv_installed() -> bool {
    match Command::new("which").arg("mpv").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Monitors the mpv process and returns when it exits
fn monitor_process(process: Arc<Mutex<Child>>, event_listener: Option<Arc<Mutex<neatflix_mpvrs::MpvEventListener>>>) {
    let check_interval = Duration::from_millis(500);
    
    // Create a flag to track if process has exited
    let process_exited = Arc::new(Mutex::new(false));
    let process_exited_clone = Arc::clone(&process_exited);
    let process_clone = Arc::clone(&process);
    
    // Spawn a thread to wait for the process to exit
    let wait_thread = thread::spawn(move || {
        debug!("Process monitor thread started");
        if let Ok(mut process_guard) = process_clone.lock() {
            match process_guard.wait() {
                Ok(exit_status) => {
                    info!("MPV process exited with status: {}", exit_status);
                    // Set the exited flag
                    if let Ok(mut exited) = process_exited.lock() {
                        *exited = true;
                    }
                },
                Err(e) => {
                    error!("Error waiting for mpv process: {}", e);
                }
            }
        }
        debug!("Process monitor thread completed");
    });
    
    // If we have an event listener, handle process exit events
    if let Some(event_listener) = event_listener {
        // Subscribe to process exit events
        if let Ok(mut listener) = event_listener.lock() {
            let process_exited_clone2 = Arc::clone(&process_exited_clone);
            if let Err(e) = listener.subscribe("process", move |event| {
                if let MpvEvent::ProcessExited(_) = event {
                    info!("Received process exit event from IPC");
                    // Update the exited flag
                    if let Ok(mut exited) = process_exited_clone2.lock() {
                        *exited = true;
                    }
                }
            }) {
                error!("Error subscribing to process exit events: {}", e);
            }
        }
    }
    
    // Wait for the process to exit or be manually closed
    loop {
        // Check if the process has exited
        if let Ok(exited) = process_exited_clone.lock() {
            if *exited {
                debug!("Process exited flag is set, breaking monitoring loop");
                break;
            }
        }
        
        // Check if the process is still running
        let is_running = if let Ok(mut process_guard) = process.lock() {
            match process_guard.try_wait() {
                Ok(None) => true,      // Process still running
                Ok(Some(_)) => false,  // Process exited
                Err(_) => false,       // Error checking process
            }
        } else {
            false
        };
        
        if !is_running {
            debug!("Process is no longer running, breaking monitoring loop");
            break;
        }
        
        // Sleep before checking again
        thread::sleep(check_interval);
    }
    
    // Wait for the wait thread to complete
    let _ = wait_thread.join();
    
    info!("Process monitoring completed");
}

fn main() {
    // Setup logging
    setup_logging();

    // Check if mpv is installed
    if !check_mpv_installed() {
        error!("mpv is not installed. Please install it to use this application.");
        std::process::exit(1);
    }

    // Set RUST_LOG if not already set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    let mut media_path = None;
    let mut preset_name = None;
    let mut with_ipc = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--list-presets" => {
                // List available presets
                println!("Available presets:");
                for preset in neatflix_mpvrs::list_available_presets() {
                    println!("  - {}", preset);
                }
                return;
            }
            "--preset" => {
                if i + 1 < args.len() {
                    preset_name = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--ipc" => {
                with_ipc = true;
            }
            _ => {
                if media_path.is_none() {
                    media_path = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    if media_path.is_none() {
        error!("No media path provided. Usage: {} <media_path> [--preset <preset_name>] [--ipc]", args[0]);
        std::process::exit(1);
    }

    // Launch mpv with or without preset
    let media_path = media_path.unwrap();
    
    if with_ipc {
        info!("Launching media with IPC control: {}", media_path);
        
        // Create a tokio runtime for async operations
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create tokio runtime: {}", e);
                std::process::exit(1);
            }
        };
        
        // Create video manager with IPC
        let video_manager = VideoManager::new();
        
        // Set up event listener for video events
        let mut subscription = rt.block_on(async {
            video_manager.subscribe().await
        });
        
        // Start thread to monitor video events
        let video_closed = Arc::new(Mutex::new(false));
        let video_closed_clone = Arc::clone(&video_closed);
        
        thread::spawn(move || {
            info!("Event monitoring thread started");
            
            // Create a runtime for this thread
            let thread_rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Failed to create thread runtime: {}", e);
                    return;
                }
            };
            
            // Process events in a loop
            thread_rt.block_on(async {
                while let Some(event) = subscription.recv().await {
                    match event {
                        VideoEvent::Started { id } => info!("Video started: {:?}", id),
                        VideoEvent::Paused { id } => info!("Video paused: {:?}", id),
                        VideoEvent::Resumed { id } => info!("Video resumed: {:?}", id),
                        VideoEvent::Ended { id } => {
                            info!("Video ended: {:?}", id);
                            // Set the closed flag
                            if let Ok(mut closed) = video_closed.lock() {
                                *closed = true;
                            }
                        },
                        VideoEvent::Closed { id } => {
                            info!("Video closed: {:?}", id);
                            // Set the closed flag
                            if let Ok(mut closed) = video_closed.lock() {
                                *closed = true;
                            }
                        },
                        VideoEvent::Error { id, message } => error!("Video error for {:?}: {}", id, message),
                        VideoEvent::Progress { id, position, duration, percent } => {
                            debug!("Video progress: {:?} - {:.1}/{:.1} ({:.1}%)", 
                                id, position, duration, percent * 100.0);
                        }
                    }
                }
            });
            
            info!("Event monitoring thread completed");
        });
        
        // Start playback
        let mut playback_options = PlaybackOptions::default();
        
        if let Some(preset) = preset_name {
            info!("Using preset: {}", preset);
            playback_options.preset = Some(preset);
        }
        
        // Start playback and get process
        let video_id = match rt.block_on(async {
            video_manager.play(media_path.to_string(), playback_options).await
        }) {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to play media: {}", e);
                std::process::exit(1);
            }
        };
        
        info!("Started playback with video ID: {:?}", video_id);
        
        // Monitor for video closure or process exit
        let mut should_exit = false;
        while !should_exit {
            // Check if video was closed by event
            if let Ok(closed) = video_closed_clone.lock() {
                if *closed {
                    info!("Video closed detected from events");
                    should_exit = true;
                }
            }
            
            // Sleep before checking again
            thread::sleep(Duration::from_millis(500));
        }
        
        // Clean up
        info!("Cleaning up resources");
        rt.block_on(async {
            if let Err(e) = video_manager.close(video_id).await {
                error!("Error closing video: {}", e);
            }
            
            if let Err(e) = video_manager.close_all().await {
                error!("Error closing video manager: {}", e);
            }
        });
    } else {
        // Just launch mpv without IPC control
        info!("Launching media without IPC control: {}", media_path);
        
        let result = if let Some(preset) = preset_name {
            info!("Using preset: {}", preset);
            neatflix_mpvrs::spawn_mpv_with_preset(&media_path, Some(&preset), &[])
        } else {
            neatflix_mpvrs::spawn_mpv(&media_path, &[])
        };
        
        match result {
            Ok((process, socket_path)) => {
                info!("MPV process spawned with socket: {}", socket_path);
                
                let process = Arc::new(Mutex::new(process));
                
                // Create IPC client for monitoring
                let ipc_client = match neatflix_mpvrs::connect_ipc(&socket_path) {
                    Ok(client) => {
                        info!("Connected to MPV IPC socket");
                        Some(client)
                    },
                    Err(e) => {
                        error!("Error connecting to IPC socket: {}", e);
                        None
                    }
                };
                
                // Create event listener if IPC client is available
                let event_listener = ipc_client.map(|client| {
                    let listener = neatflix_mpvrs::create_event_listener(client);
                    Arc::new(Mutex::new(listener))
                });
                
                // Monitor the process and wait for it to exit
                monitor_process(process, event_listener);
            },
            Err(e) => {
                error!("Failed to launch mpv: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    info!("Application exiting");
}
