// Audio Streaming System
// Provides efficient streaming for large audio files (music, ambient sounds)
// without loading entire files into memory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;

/// Audio streaming configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Size of each buffer chunk in bytes
    pub buffer_size: usize,
    /// Number of buffers to use (double/triple buffering)
    pub buffer_count: usize,
    /// Whether to loop the stream
    pub looping: bool,
    /// Volume (0.0 to 1.0)
    pub volume: f32,
    /// Whether to start playing immediately
    pub auto_play: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 65536, // 64KB chunks
            buffer_count: 3,    // Triple buffering
            looping: false,
            volume: 1.0,
            auto_play: true,
        }
    }
}

impl StreamConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn buffer_count(mut self, count: usize) -> Self {
        self.buffer_count = count;
        self
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn auto_play(mut self, auto_play: bool) -> Self {
        self.auto_play = auto_play;
        self
    }
}

/// State of an audio stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is stopped
    Stopped,
    /// Stream is playing
    Playing,
    /// Stream is paused
    Paused,
    /// Stream is buffering
    Buffering,
    /// Stream encountered an error
    Error,
}

/// Handle to a streaming audio source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamHandle(u64);

impl StreamHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Audio stream data
struct AudioStream {
    /// Path to the audio file
    path: PathBuf,
    /// Stream configuration
    config: StreamConfig,
    /// Current state
    state: StreamState,
    /// Current playback position in bytes
    position: u64,
    /// Total size in bytes
    total_size: u64,
    /// Current volume
    volume: f32,
    /// Whether the stream is looping
    looping: bool,
    /// Buffered audio data
    buffers: Vec<Vec<u8>>,
    /// Current buffer index
    current_buffer: usize,
}

impl AudioStream {
    fn new(path: PathBuf, config: StreamConfig, total_size: u64) -> Self {
        let buffer_count = config.buffer_count;
        Self {
            path,
            volume: config.volume,
            looping: config.looping,
            config,
            state: StreamState::Stopped,
            position: 0,
            total_size,
            buffers: vec![Vec::new(); buffer_count],
            current_buffer: 0,
        }
    }

    fn play(&mut self) {
        if self.state != StreamState::Playing {
            self.state = StreamState::Playing;
        }
    }

    fn pause(&mut self) {
        if self.state == StreamState::Playing {
            self.state = StreamState::Paused;
        }
    }

    fn stop(&mut self) {
        self.state = StreamState::Stopped;
        self.position = 0;
        self.current_buffer = 0;
    }

    fn seek(&mut self, position: f32) {
        let position = position.clamp(0.0, 1.0);
        self.position = (position * self.total_size as f32) as u64;
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    fn get_playback_position(&self) -> f32 {
        if self.total_size == 0 {
            0.0
        } else {
            self.position as f32 / self.total_size as f32
        }
    }

    fn is_finished(&self) -> bool {
        self.position >= self.total_size && !self.looping
    }
}

/// Audio streaming manager
pub struct AudioStreamManager {
    streams: HashMap<StreamHandle, AudioStream>,
    next_id: u64,
    /// Background thread for streaming
    _streaming_thread: Option<thread::JoinHandle<()>>,
}

impl AudioStreamManager {
    /// Create a new audio stream manager
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            next_id: 1,
            _streaming_thread: None,
        }
    }

    /// Load and start streaming an audio file
    pub fn load_stream<P: AsRef<Path>>(
        &mut self,
        path: P,
        config: StreamConfig,
    ) -> Result<StreamHandle, String> {
        let path = path.as_ref().to_path_buf();

        // Check if file exists
        if !path.exists() {
            return Err(format!("Audio file not found: {:?}", path));
        }

        // Get file size
        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;
        let total_size = metadata.len();

        // Create stream
        let handle = StreamHandle::new(self.next_id);
        self.next_id += 1;

        let mut stream = AudioStream::new(path, config.clone(), total_size);

        // Auto-play if configured
        if config.auto_play {
            stream.play();
        }

        self.streams.insert(handle, stream);

        Ok(handle)
    }

    /// Play a stream
    pub fn play(&mut self, handle: StreamHandle) -> Result<(), String> {
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        stream.play();
        Ok(())
    }

    /// Pause a stream
    pub fn pause(&mut self, handle: StreamHandle) -> Result<(), String> {
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        stream.pause();
        Ok(())
    }

    /// Stop a stream
    pub fn stop(&mut self, handle: StreamHandle) -> Result<(), String> {
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        stream.stop();
        Ok(())
    }

    /// Seek to a position in the stream (0.0 to 1.0)
    pub fn seek(&mut self, handle: StreamHandle, position: f32) -> Result<(), String> {
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        stream.seek(position);
        Ok(())
    }

    /// Set stream volume (0.0 to 1.0)
    pub fn set_volume(&mut self, handle: StreamHandle, volume: f32) -> Result<(), String> {
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        stream.set_volume(volume);
        Ok(())
    }

    /// Set whether the stream should loop
    pub fn set_looping(&mut self, handle: StreamHandle, looping: bool) -> Result<(), String> {
        let stream = self
            .streams
            .get_mut(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        stream.set_looping(looping);
        Ok(())
    }

    /// Get the current state of a stream
    pub fn get_state(&self, handle: StreamHandle) -> Option<StreamState> {
        self.streams.get(&handle).map(|s| s.state)
    }

    /// Get the current playback position (0.0 to 1.0)
    pub fn get_position(&self, handle: StreamHandle) -> Option<f32> {
        self.streams.get(&handle).map(|s| s.get_playback_position())
    }

    /// Check if a stream has finished playing
    pub fn is_finished(&self, handle: StreamHandle) -> bool {
        self.streams
            .get(&handle)
            .map(|s| s.is_finished())
            .unwrap_or(true)
    }

    /// Unload a stream and free its resources
    pub fn unload_stream(&mut self, handle: StreamHandle) -> Result<(), String> {
        self.streams
            .remove(&handle)
            .ok_or_else(|| "Invalid stream handle".to_string())?;
        Ok(())
    }

    /// Update all active streams (call this every frame)
    pub fn update(&mut self, _delta_time: f32) {
        // In a real implementation, this would:
        // 1. Check buffer status for each playing stream
        // 2. Request more data from the streaming thread if needed
        // 3. Update playback positions
        // 4. Handle loop points
        // 5. Clean up finished streams

        // For now, we'll just update positions for playing streams
        for stream in self.streams.values_mut() {
            if stream.state == StreamState::Playing {
                // Simulate playback progress
                // In a real implementation, this would be based on actual audio output
                let bytes_per_second = 44100 * 2 * 2; // 44.1kHz, stereo, 16-bit
                let bytes_this_frame = (bytes_per_second as f32 * _delta_time) as u64;
                stream.position += bytes_this_frame;

                // Handle looping
                if stream.position >= stream.total_size {
                    if stream.looping {
                        stream.position = stream.position % stream.total_size;
                    } else {
                        stream.position = stream.total_size;
                        stream.state = StreamState::Stopped;
                    }
                }
            }
        }
    }

    /// Get the number of active streams
    pub fn active_stream_count(&self) -> usize {
        self.streams
            .values()
            .filter(|s| s.state == StreamState::Playing)
            .count()
    }

    /// Get the total number of loaded streams
    pub fn total_stream_count(&self) -> usize {
        self.streams.len()
    }
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Music player - high-level interface for background music
pub struct MusicPlayer {
    manager: AudioStreamManager,
    current_track: Option<StreamHandle>,
    playlist: Vec<PathBuf>,
    current_index: usize,
    shuffle: bool,
    repeat_mode: RepeatMode,
    crossfade_duration: f32,
}

/// Repeat mode for music player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    /// Don't repeat
    Off,
    /// Repeat the current track
    One,
    /// Repeat the entire playlist
    All,
}

impl MusicPlayer {
    /// Create a new music player
    pub fn new() -> Self {
        Self {
            manager: AudioStreamManager::new(),
            current_track: None,
            playlist: Vec::new(),
            current_index: 0,
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            crossfade_duration: 0.0,
        }
    }

    /// Load and play a single track
    pub fn play_track<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let config = StreamConfig::default().looping(false).auto_play(true);
        let handle = self.manager.load_stream(path, config)?;
        self.current_track = Some(handle);
        Ok(())
    }

    /// Add a track to the playlist
    pub fn add_to_playlist<P: AsRef<Path>>(&mut self, path: P) {
        self.playlist.push(path.as_ref().to_path_buf());
    }

    /// Clear the playlist
    pub fn clear_playlist(&mut self) {
        self.playlist.clear();
        self.current_index = 0;
    }

    /// Play the playlist
    pub fn play_playlist(&mut self) -> Result<(), String> {
        if self.playlist.is_empty() {
            return Err("Playlist is empty".to_string());
        }

        self.current_index = 0;
        self.play_current_track()
    }

    /// Play the next track in the playlist
    pub fn next(&mut self) -> Result<(), String> {
        if self.playlist.is_empty() {
            return Err("Playlist is empty".to_string());
        }

        self.current_index = (self.current_index + 1) % self.playlist.len();
        self.play_current_track()
    }

    /// Play the previous track in the playlist
    pub fn previous(&mut self) -> Result<(), String> {
        if self.playlist.is_empty() {
            return Err("Playlist is empty".to_string());
        }

        if self.current_index == 0 {
            self.current_index = self.playlist.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.play_current_track()
    }

    /// Pause playback
    pub fn pause(&mut self) -> Result<(), String> {
        if let Some(handle) = self.current_track {
            self.manager.pause(handle)?;
        }
        Ok(())
    }

    /// Resume playback
    pub fn resume(&mut self) -> Result<(), String> {
        if let Some(handle) = self.current_track {
            self.manager.play(handle)?;
        }
        Ok(())
    }

    /// Stop playback
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(handle) = self.current_track {
            self.manager.stop(handle)?;
            self.manager.unload_stream(handle)?;
            self.current_track = None;
        }
        Ok(())
    }

    /// Set the volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        if let Some(handle) = self.current_track {
            self.manager.set_volume(handle, volume)?;
        }
        Ok(())
    }

    /// Set the repeat mode
    pub fn set_repeat_mode(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    /// Set shuffle mode
    pub fn set_shuffle(&mut self, shuffle: bool) {
        self.shuffle = shuffle;
    }

    /// Set crossfade duration in seconds
    pub fn set_crossfade(&mut self, duration: f32) {
        self.crossfade_duration = duration.max(0.0);
    }

    /// Update the music player (call every frame)
    pub fn update(&mut self, delta_time: f32) {
        self.manager.update(delta_time);

        // Check if current track finished
        if let Some(handle) = self.current_track {
            if self.manager.is_finished(handle) {
                // Handle repeat modes
                match self.repeat_mode {
                    RepeatMode::One => {
                        let _ = self.manager.stop(handle);
                        let _ = self.play_current_track();
                    }
                    RepeatMode::All => {
                        let _ = self.next();
                    }
                    RepeatMode::Off => {
                        if self.current_index + 1 < self.playlist.len() {
                            let _ = self.next();
                        } else {
                            let _ = self.stop();
                        }
                    }
                }
            }
        }
    }

    /// Get the current playback position (0.0 to 1.0)
    pub fn get_position(&self) -> f32 {
        if let Some(handle) = self.current_track {
            self.manager.get_position(handle).unwrap_or(0.0)
        } else {
            0.0
        }
    }

    /// Check if music is currently playing
    pub fn is_playing(&self) -> bool {
        if let Some(handle) = self.current_track {
            self.manager.get_state(handle) == Some(StreamState::Playing)
        } else {
            false
        }
    }

    fn play_current_track(&mut self) -> Result<(), String> {
        if self.playlist.is_empty() {
            return Err("Playlist is empty".to_string());
        }

        // Stop current track if any
        if let Some(handle) = self.current_track {
            let _ = self.manager.stop(handle);
            let _ = self.manager.unload_stream(handle);
        }

        // Load and play new track
        let path = &self.playlist[self.current_index];
        let config = StreamConfig::default().looping(false).auto_play(true);
        let handle = self.manager.load_stream(path, config)?;
        self.current_track = Some(handle);

        Ok(())
    }
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_config() {
        let config = StreamConfig::new()
            .buffer_size(32768)
            .buffer_count(2)
            .looping(true)
            .volume(0.5)
            .auto_play(false);

        assert_eq!(config.buffer_size, 32768);
        assert_eq!(config.buffer_count, 2);
        assert!(config.looping);
        assert_eq!(config.volume, 0.5);
        assert!(!config.auto_play);
    }

    #[test]
    fn test_stream_handle() {
        let handle1 = StreamHandle::new(1);
        let handle2 = StreamHandle::new(2);

        assert_eq!(handle1.id(), 1);
        assert_eq!(handle2.id(), 2);
        assert_ne!(handle1, handle2);
    }

    #[test]
    fn test_stream_manager_creation() {
        let manager = AudioStreamManager::new();
        assert_eq!(manager.total_stream_count(), 0);
        assert_eq!(manager.active_stream_count(), 0);
    }

    #[test]
    fn test_music_player_creation() {
        let player = MusicPlayer::new();
        assert!(!player.is_playing());
        assert_eq!(player.get_position(), 0.0);
    }

    #[test]
    fn test_music_player_playlist() {
        let mut player = MusicPlayer::new();
        player.add_to_playlist("track1.ogg");
        player.add_to_playlist("track2.ogg");
        player.add_to_playlist("track3.ogg");

        assert_eq!(player.playlist.len(), 3);

        player.clear_playlist();
        assert_eq!(player.playlist.len(), 0);
    }

    #[test]
    fn test_repeat_mode() {
        let mut player = MusicPlayer::new();
        player.set_repeat_mode(RepeatMode::All);
        assert_eq!(player.repeat_mode, RepeatMode::All);

        player.set_repeat_mode(RepeatMode::One);
        assert_eq!(player.repeat_mode, RepeatMode::One);
    }

    #[test]
    fn test_stream_state() {
        let mut stream = AudioStream::new(
            PathBuf::from("test.ogg"),
            StreamConfig::default(),
            1000,
        );

        assert_eq!(stream.state, StreamState::Stopped);

        stream.play();
        assert_eq!(stream.state, StreamState::Playing);

        stream.pause();
        assert_eq!(stream.state, StreamState::Paused);

        stream.stop();
        assert_eq!(stream.state, StreamState::Stopped);
        assert_eq!(stream.position, 0);
    }

    #[test]
    fn test_stream_seek() {
        let mut stream = AudioStream::new(
            PathBuf::from("test.ogg"),
            StreamConfig::default(),
            1000,
        );

        stream.seek(0.5);
        assert_eq!(stream.position, 500);

        stream.seek(0.0);
        assert_eq!(stream.position, 0);

        stream.seek(1.0);
        assert_eq!(stream.position, 1000);
    }

    #[test]
    fn test_stream_volume() {
        let mut stream = AudioStream::new(
            PathBuf::from("test.ogg"),
            StreamConfig::default(),
            1000,
        );

        stream.set_volume(0.5);
        assert_eq!(stream.volume, 0.5);

        // Test clamping
        stream.set_volume(1.5);
        assert_eq!(stream.volume, 1.0);

        stream.set_volume(-0.5);
        assert_eq!(stream.volume, 0.0);
    }

    #[test]
    fn test_stream_looping() {
        let mut stream = AudioStream::new(
            PathBuf::from("test.ogg"),
            StreamConfig::default().looping(false),
            1000,
        );

        stream.position = 1000;
        assert!(stream.is_finished());

        stream.set_looping(true);
        assert!(!stream.is_finished());
    }

    #[test]
    fn test_crossfade_setting() {
        let mut player = MusicPlayer::new();
        player.set_crossfade(2.0);
        assert_eq!(player.crossfade_duration, 2.0);

        // Test negative clamping
        player.set_crossfade(-1.0);
        assert_eq!(player.crossfade_duration, 0.0);
    }
}

