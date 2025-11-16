// Terrain Editor Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct TerrainEditorPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for TerrainEditorPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl TerrainEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🏔️ Terrain Editor");
        ui.separator();
        ui.label("Terrain editor coming soon...");
    }
}

