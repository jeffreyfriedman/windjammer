// Weapon System Editor Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct WeaponEditorPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for WeaponEditorPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl WeaponEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🔫 Weapon System Editor");
        ui.separator();
        ui.label("Weapon editor coming soon...");
    }
}

