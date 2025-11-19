/// Audio source component
pub struct AudioSource {
    /// Volume (0-1)
    pub volume: f32,
    /// Pitch multiplier
    pub pitch: f32,
    /// Whether to loop
    pub looping: bool,
}

impl AudioSource {
    pub fn new() -> AudioSource {
        todo!()
    }

    /// Plays an audio file
    pub fn play(&mut self, audio_path: String) {
        todo!()
    }

    /// Stops playback
    pub fn stop(&mut self) {
        todo!()
    }

    /// Pauses playback
    pub fn pause(&mut self) {
        todo!()
    }

}
