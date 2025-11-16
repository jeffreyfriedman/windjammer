// Animation State Machine Editor Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AnimationEditorPanel {
    // TODO: Implement animation state machine editor
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AnimationEditorPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AnimationEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🎬 Animation State Machine");
        ui.separator();
        ui.label("Animation editor coming soon...");
    }
}

