# macOS Platform Configuration

This directory contains macOS-specific configurations for the mpv video player with uosc.

## Current Implementation

The macOS implementation is the primary focus of this project. The main configuration files are located in the parent directory:

1. `mpv.conf` - Main mpv configuration file
2. `input.conf` - Keyboard and mouse input configuration
3. `scripts/uosc.lua` - Main uosc script loader
4. `scripts/uosc/` - uosc script files
5. `script-opts/uosc.conf` - uosc configuration
6. `fonts/` - uosc font files

## Notes

- The macOS implementation uses the system-installed mpv binary
- Font rendering uses the system font 'SF Pro' by default
- Hardware acceleration is enabled by default with `hwdec=auto-safe`
- Performance settings are optimized for macOS 