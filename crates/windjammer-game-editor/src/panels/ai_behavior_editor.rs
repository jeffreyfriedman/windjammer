// AI Behavior Tree Editor Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct AIBehaviorEditorPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for AIBehaviorEditorPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl AIBehaviorEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🤖 AI Behavior Tree");
        ui.separator();
        ui.label("AI behavior tree editor coming soon...");
    }
}

