// Audio Mixer Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AudioMixerPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AudioMixerPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AudioMixerPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🔊 Audio Mixer");
        ui.separator();
        ui.label("Audio mixer coming soon...");
    }
}

