// Navigation Mesh Editor Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct NavMeshEditorPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for NavMeshEditorPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl NavMeshEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🗺️ Navigation Mesh Editor");
        ui.separator();
        ui.label("Navigation mesh editor coming soon...");
    }
}

