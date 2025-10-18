//! Audio playback

/// Audio source (sound file)
pub struct AudioSource {
    _placeholder: (),
}

/// Audio player
pub struct AudioPlayer {
    _placeholder: (),
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    pub fn play(&mut self, _source: &AudioSource) {
        // TODO: Implement with rodio/kira
    }

    pub fn stop(&mut self) {
        // TODO: Implement
    }

    pub fn set_volume(&mut self, _volume: f32) {
        // TODO: Implement
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}
