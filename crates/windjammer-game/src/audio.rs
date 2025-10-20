//! Audio playback using rodio

#[cfg(feature = "audio")]
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
#[cfg(feature = "audio")]
use std::fs::File;
#[cfg(feature = "audio")]
use std::io::BufReader;

use crate::math::Vec3;

/// Audio system
pub struct AudioSystem {
    #[cfg(feature = "audio")]
    _stream: OutputStream,
    #[cfg(feature = "audio")]
    stream_handle: OutputStreamHandle,
    #[cfg(feature = "audio")]
    music_sink: Option<Sink>,
    master_volume: f32,
}

impl AudioSystem {
    pub fn new() -> Result<Self, String> {
        #[cfg(feature = "audio")]
        {
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|e| format!("Failed to create audio output stream: {}", e))?;

            Ok(Self {
                _stream,
                stream_handle,
                music_sink: None,
                master_volume: 1.0,
            })
        }

        #[cfg(not(feature = "audio"))]
        {
            Ok(Self {
                master_volume: 1.0,
            })
        }
    }

    /// Play a sound effect
    pub fn play_sound(&mut self, path: &str) -> Result<(), String> {
        #[cfg(feature = "audio")]
        {
            let file =
                File::open(path).map_err(|e| format!("Failed to open audio file: {}", e))?;
            let source = Decoder::new(BufReader::new(file))
                .map_err(|e| format!("Failed to decode audio: {}", e))?;

            let sink = Sink::try_new(&self.stream_handle)
                .map_err(|e| format!("Failed to create sink: {}", e))?;

            sink.set_volume(self.master_volume);
            sink.append(source);
            sink.detach();

            Ok(())
        }

        #[cfg(not(feature = "audio"))]
        {
            let _ = path;
            Err("Audio feature not enabled".to_string())
        }
    }

    /// Play background music (looping)
    pub fn play_music(&mut self, path: &str, looping: bool) -> Result<(), String> {
        #[cfg(feature = "audio")]
        {
            // Stop existing music
            if let Some(sink) = self.music_sink.take() {
                sink.stop();
            }

            let file =
                File::open(path).map_err(|e| format!("Failed to open music file: {}", e))?;
            let source = Decoder::new(BufReader::new(file))
                .map_err(|e| format!("Failed to decode music: {}", e))?;

            let sink = Sink::try_new(&self.stream_handle)
                .map_err(|e| format!("Failed to create music sink: {}", e))?;

            sink.set_volume(self.master_volume);

            if looping {
                sink.append(source.repeat_infinite());
            } else {
                sink.append(source);
            }

            self.music_sink = Some(sink);
            Ok(())
        }

        #[cfg(not(feature = "audio"))]
        {
            let _ = (path, looping);
            Err("Audio feature not enabled".to_string())
        }
    }

    /// Stop music
    pub fn stop_music(&mut self) {
        #[cfg(feature = "audio")]
        {
            if let Some(sink) = self.music_sink.take() {
                sink.stop();
            }
        }
    }

    /// Set master volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);

        #[cfg(feature = "audio")]
        {
            if let Some(sink) = &self.music_sink {
                sink.set_volume(self.master_volume);
            }
        }
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            #[cfg(feature = "audio")]
            _stream: OutputStream::try_default().unwrap().0,
            #[cfg(feature = "audio")]
            stream_handle: OutputStream::try_default().unwrap().1,
            #[cfg(feature = "audio")]
            music_sink: None,
            master_volume: 1.0,
        })
    }
}

/// 3D spatial audio source
pub struct SpatialAudioSource {
    pub position: Vec3,
    pub max_distance: f32,
    pub rolloff_factor: f32,
}

impl SpatialAudioSource {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            max_distance: 100.0,
            rolloff_factor: 1.0,
        }
    }

    /// Calculate volume based on listener position
    pub fn calculate_volume(&self, listener_pos: Vec3) -> f32 {
        let distance = (self.position - listener_pos).length();
        if distance >= self.max_distance {
            return 0.0;
        }

        let attenuation = 1.0 / (1.0 + self.rolloff_factor * distance);
        attenuation.clamp(0.0, 1.0)
    }
}
