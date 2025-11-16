// Profiler Visualization Panel

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct ProfilerPanel {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for ProfilerPanel {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl ProfilerPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("📊 Performance Profiler");
        ui.separator();
        ui.label("Profiler visualization coming soon...");
    }
}

