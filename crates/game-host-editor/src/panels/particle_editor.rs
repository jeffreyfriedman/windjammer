// Particle System Editor Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct ParticleEditorPanel {
    // TODO: Implement particle system editor
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for ParticleEditorPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl ParticleEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("✨ Particle System Editor");
        ui.separator();
        ui.label("Particle editor coming soon...");
    }
}

