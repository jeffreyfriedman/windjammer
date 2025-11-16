// Gamepad Configuration Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct GamepadConfigPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for GamepadConfigPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl GamepadConfigPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🎮 Gamepad Configuration");
        ui.separator();
        ui.label("Gamepad configuration coming soon...");
    }
}

