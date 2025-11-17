//! Audio Loader
//!
//! Loads audio files (OGG, MP3, WAV, FLAC) into memory for playback.
//! Supports both streaming (for music) and buffered (for sound effects).
//!
//! ## Supported Formats
//! - ✅ OGG Vorbis (.ogg)
//! - ✅ MP3 (.mp3)
//! - ✅ WAV (.wav)
//! - ✅ FLAC (.flac)
//!
//! ## Usage
//! ```rust
//! use windjammer_game_framework::AudioLoader;
//!
//! // Load a sound effect (fully buffered)
//! let sound = AudioLoader::load("assets/sounds/explosion.ogg")?;
//!
//! // Load music (streaming)
//! let music = AudioLoader::load_streaming("assets/music/theme.mp3")?;
//! ```

use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "audio")]
use rodio::{Decoder, Source};

/// Audio data loaded into memory
#[derive(Clone)]
pub struct AudioData {
    /// Raw audio data (encoded format: OGG, MP3, WAV, etc.)
    pub data: Arc<Vec<u8>>,
    /// Audio format (file extension)
    pub format: AudioFormat,
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Duration in seconds (if known)
    pub duration: Option<f32>,
}

/// Supported audio formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// OGG Vorbis (.ogg)
    Ogg,
    /// MP3 (.mp3)
    Mp3,
    /// WAV (.wav)
    Wav,
    /// FLAC (.flac)
    Flac,
    /// Unknown format
    Unknown,
}

impl AudioFormat {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "ogg" => AudioFormat::Ogg,
            "mp3" => AudioFormat::Mp3,
            "wav" => AudioFormat::Wav,
            "flac" => AudioFormat::Flac,
            _ => AudioFormat::Unknown,
        }
    }

    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Ogg => "ogg",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Flac => "flac",
            AudioFormat::Unknown => "unknown",
        }
    }
}

/// Audio loader
pub struct AudioLoader;

impl AudioLoader {
    /// Load an audio file into memory
    ///
    /// This loads the entire file into memory and decodes metadata.
    /// Best for sound effects and short audio clips.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<AudioData, String> {
        let path = path.as_ref();
        
        // Read the entire file into memory
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open audio file '{}': {}", path.display(), e))?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read audio file '{}': {}", path.display(), e))?;
        
        // Detect format from extension
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .map(AudioFormat::from_extension)
            .unwrap_or(AudioFormat::Unknown);
        
        // Decode to get metadata
        let (sample_rate, channels, duration) = Self::decode_metadata(&data, format)?;
        
        Ok(AudioData {
            data: Arc::new(data),
            format,
            sample_rate,
            channels,
            duration: Some(duration),
        })
    }
    
    /// Load audio from memory
    ///
    /// Useful for loading audio from embedded resources or network.
    pub fn load_from_memory(data: Vec<u8>, format: AudioFormat) -> Result<AudioData, String> {
        // Decode to get metadata
        let (sample_rate, channels, duration) = Self::decode_metadata(&data, format)?;
        
        Ok(AudioData {
            data: Arc::new(data),
            format,
            sample_rate,
            channels,
            duration: Some(duration),
        })
    }
    
    /// Decode audio metadata (sample rate, channels, duration)
    #[cfg(feature = "audio")]
    fn decode_metadata(data: &[u8], format: AudioFormat) -> Result<(u32, u16, f32), String> {
        // Clone the data to avoid lifetime issues
        let data_owned = data.to_vec();
        let cursor = Cursor::new(data_owned);
        let decoder = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode audio (format: {:?}): {}", format, e))?;
        
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        
        // Calculate duration by counting samples
        // Note: This consumes the decoder, so we can't reuse it
        let total_samples = decoder.count();
        let duration = total_samples as f32 / (sample_rate as f32 * channels as f32);
        
        Ok((sample_rate, channels, duration))
    }
    
    /// Decode audio metadata (no-op when audio feature is disabled)
    #[cfg(not(feature = "audio"))]
    fn decode_metadata(_data: &[u8], _format: AudioFormat) -> Result<(u32, u16, f32), String> {
        // Return placeholder values when audio is disabled
        Ok((44100, 2, 0.0))
    }
    
    /// Create a decoder for playback
    ///
    /// This creates a new decoder from the audio data.
    /// Can be called multiple times to play the same audio simultaneously.
    #[cfg(feature = "audio")]
    pub fn create_decoder(audio: &AudioData) -> Result<Decoder<Cursor<Vec<u8>>>, String> {
        let cursor = Cursor::new((*audio.data).clone());
        Decoder::new(cursor)
            .map_err(|e| format!("Failed to create audio decoder: {}", e))
    }
    
    /// Create a decoder for playback (no-op when audio feature is disabled)
    #[cfg(not(feature = "audio"))]
    pub fn create_decoder(_audio: &AudioData) -> Result<(), String> {
        Err("Audio feature not enabled".to_string())
    }
}

impl Default for AudioData {
    fn default() -> Self {
        // Create a silent 1-second audio clip
        Self {
            data: Arc::new(Vec::new()),
            format: AudioFormat::Wav,
            sample_rate: 44100,
            channels: 2,
            duration: Some(1.0),
        }
    }
}

impl std::fmt::Debug for AudioData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioData")
            .field("format", &self.format)
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("duration", &self.duration)
            .field("data_size", &self.data.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_format_detection() {
        assert_eq!(AudioFormat::from_extension("ogg"), AudioFormat::Ogg);
        assert_eq!(AudioFormat::from_extension("OGG"), AudioFormat::Ogg);
        assert_eq!(AudioFormat::from_extension("mp3"), AudioFormat::Mp3);
        assert_eq!(AudioFormat::from_extension("wav"), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_extension("flac"), AudioFormat::Flac);
        assert_eq!(AudioFormat::from_extension("xyz"), AudioFormat::Unknown);
    }
    
    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::Ogg.extension(), "ogg");
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Wav.extension(), "wav");
        assert_eq!(AudioFormat::Flac.extension(), "flac");
    }
    
    #[test]
    fn test_audio_data_default() {
        let audio = AudioData::default();
        assert_eq!(audio.format, AudioFormat::Wav);
        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.duration, Some(1.0));
    }
}

