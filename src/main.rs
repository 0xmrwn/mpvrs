use log::{error, info};
use neatflix_mpvrs::{config, setup_logging, MpvEvent};
use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn check_mpv_installed() -> bool {
    match Command::new("which").arg("mpv").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn main() {
    // Initialize logging
    setup_logging();
    info!("neatflix-mpvrs v{}", neatflix_mpvrs::version());
    
    // Set RUST_LOG environment variable if not already set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    
    // Check if mpv is installed
    if !check_mpv_installed() {
        eprintln!("Error: mpv is not installed or not in your PATH.");
        eprintln!("Please install mpv before using neatflix-mpvrs.");
        eprintln!("On macOS, you can install it with: brew install mpv");
        std::process::exit(1);
    }
    
    // Initialize default configuration
    if let Err(e) = config::initialize_default_config() {
        error!("Failed to initialize configuration: {}", e);
        std::process::exit(1);
    }
    
    // Get media file from command line arguments or use a default
    let args: Vec<String> = env::args().collect();
    
    // Check for special commands that don't require a media file
    if args.len() > 1 && args[1] == "--list-presets" {
        // List all available presets
        println!("Available presets:");
        for preset in neatflix_mpvrs::list_available_presets() {
            if let Some(details) = neatflix_mpvrs::get_preset_details(&preset) {
                println!("  {} - {}", preset, details.description);
            } else {
                println!("  {}", preset);
            }
        }
        std::process::exit(0);
    }
    
    let media = if args.len() > 1 {
        &args[1]
    } else {
        println!("Usage: neatflix-mpvrs <media_file_or_url> [--preset=<preset_name>] [other mpv options]");
        println!("       neatflix-mpvrs --list-presets");
        println!("No media file specified. Please provide a media file path or URL.");
        std::process::exit(1);
    };
    
    // Check if a preset is specified
    let mut preset_name = None;
    let mut extra_args = Vec::new();
    let mut enable_ipc_control = false;
    
    for arg in args.iter().skip(2) {
        if arg.starts_with("--preset=") {
            preset_name = Some(arg.trim_start_matches("--preset=").to_string());
        } else if arg == "--auto-preset" {
            // Use the recommended preset based on system detection
            info!("Detecting system for auto-preset...");
            let recommended = neatflix_mpvrs::get_recommended_preset();
            preset_name = Some(recommended);
            info!("Using recommended preset: {}", preset_name.as_ref().unwrap());
        } else if arg == "--ipc-control" {
            enable_ipc_control = true;
        } else {
            extra_args.push(arg.as_str());
        }
    }
    
    info!("Playing media: {}", media);
    
    // Launch mpv with or without a preset
    let result = if let Some(preset) = preset_name {
        info!("Using preset: {}", preset);
        neatflix_mpvrs::spawn_mpv_with_preset(media, Some(&preset), &extra_args)
    } else {
        neatflix_mpvrs::spawn_mpv(media, &extra_args)
    };
    
    // Handle the result and set up IPC if requested
    match result {
        Ok((process, socket_path)) => {
            info!("MPV process spawned with PID: {:?}", process.id());
            
            if enable_ipc_control {
                info!("IPC control enabled, socket path: {}", socket_path);
                
                // Give mpv some time to start up
                thread::sleep(Duration::from_secs(1));
                
                // Connect to the IPC socket
                match neatflix_mpvrs::connect_ipc(&socket_path) {
                    Ok(ipc_client) => {
                        info!("Connected to MPV IPC socket");
                        
                        // Create an event listener
                        let mut event_listener = neatflix_mpvrs::create_event_listener(ipc_client);
                        
                        // Subscribe to time position changes
                        if let Err(e) = event_listener.subscribe("property:time-pos", |event| {
                            if let MpvEvent::PropertyChanged(property, value) = &event {
                                info!("Playback position: {} = {:?}", property, value);
                            }
                        }) {
                            error!("Error subscribing to time position events: {}", e);
                        }
                        
                        // Subscribe to playback state changes
                        if let Err(e) = event_listener.subscribe("property:pause", |event| {
                            match event {
                                MpvEvent::PlaybackPaused => info!("Playback paused"),
                                MpvEvent::PlaybackResumed => info!("Playback resumed"),
                                _ => {}
                            }
                        }) {
                            error!("Error subscribing to pause events: {}", e);
                        }
                        
                        // Start listening for events
                        if let Err(e) = event_listener.start_listening() {
                            error!("Error starting event listener: {}", e);
                        } else {
                            info!("Event listener started");
                            
                            // Wait for the process to exit
                            let _ = process.wait_with_output();
                        }
                    },
                    Err(e) => {
                        error!("Error connecting to mpv IPC: {}", e);
                    }
                }
            } else {
                // Just wait for the process to exit
                let _ = process.wait_with_output();
            }
        },
        Err(e) => {
            error!("Error launching video player: {}", e);
            std::process::exit(1);
        }
    }
}
