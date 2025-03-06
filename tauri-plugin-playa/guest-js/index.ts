import { invoke } from '@tauri-apps/api/core'

export interface PlaybackOptions {
  /** Optional preset name to use for playback */
  preset?: string;
  /** Starting position in seconds */
  startTime?: number;
  /** Volume level (0-100) */
  volume?: number;
  /** Whether to start in fullscreen mode */
  fullscreen?: boolean;
  /** Additional mpv arguments */
  extraArgs?: string[];
  /** Window options for mpv */
  windowOptions?: WindowOptions;
}

export interface WindowOptions {
  /** Window position x coordinate */
  x?: number;
  /** Window position y coordinate */
  y?: number;
  /** Window width */
  width?: number;
  /** Window height */
  height?: number;
  /** Whether the window should be decorated */
  decorated?: boolean;
  /** Whether the window should be always on top */
  alwaysOnTop?: boolean;
}

export interface VideoEvent {
  /** Video event type */
  type: 'started' | 'paused' | 'resumed' | 'ended' | 'closed' | 'error' | 'progress';
  /** Video ID */
  id: string;
  /** Current position in seconds (for progress events) */
  position?: number;
  /** Total duration in seconds (for progress events) */
  duration?: number;
  /** Percentage of playback (for progress events) */
  percent?: number;
  /** Error message (for error events) */
  message?: string;
}

/**
 * Play a video file or URL
 * @param path Path to the video file or URL
 * @param options Optional playback options
 * @returns Promise with the video ID
 */
export async function play(path: string, options?: PlaybackOptions): Promise<string> {
  const response = await invoke<{ videoId: string }>('plugin:playa|play', {
    request: {
      path,
      options: options || {},
    },
  });
  
  return response.videoId;
}

/**
 * Control video playback
 * @param videoId ID of the video to control
 * @param command Control command: 'pause', 'resume', 'seek', 'volume'
 * @param value Optional value for commands that require one (seek position, volume level)
 * @returns Promise with control response
 */
export async function control(
  videoId: string,
  command: string,
  value?: number,
): Promise<{
  success: boolean;
  position?: number;
  duration?: number;
  state?: string;
}> {
  return invoke('plugin:playa|control', {
    request: {
      videoId,
      command,
      value,
    },
  });
}

/**
 * Get information about a video
 * @param videoId ID of the video to get information for
 * @returns Promise with video information
 */
export async function getInfo(videoId: string): Promise<{
  videoId: string;
  path: string;
  position: number;
  duration: number;
  volume: number;
  isPaused: boolean;
  speed: number;
  isMuted: boolean;
}> {
  return invoke('plugin:playa|get_info', {
    request: {
      videoId,
    },
  });
}

/**
 * Close a video
 * @param videoId ID of the video to close
 * @returns Promise indicating success
 */
export async function close(videoId: string): Promise<{ success: boolean }> {
  return invoke('plugin:playa|close', {
    request: {
      videoId,
    },
  });
}

/**
 * List available presets
 * @returns Promise with list of available presets
 */
export async function listPresets(): Promise<{
  presets: string[];
  recommended?: string;
}> {
  return invoke('plugin:playa|list_presets', {
    request: {},
  });
}
