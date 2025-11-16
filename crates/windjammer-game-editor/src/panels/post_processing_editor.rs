// Post-Processing Effects Editor Panel
// Configure bloom, DOF, motion blur, color grading, etc.

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::Ui;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct PostProcessingPanel {
    // Bloom
    bloom_enabled: bool,
    bloom_intensity: f32,
    bloom_threshold: f32,
    bloom_radius: f32,
    
    // Depth of Field
    dof_enabled: bool,
    dof_focus_distance: f32,
    dof_aperture: f32,
    dof_focal_length: f32,
    
    // Motion Blur
    motion_blur_enabled: bool,
    motion_blur_intensity: f32,
    motion_blur_samples: u32,
    
    // Chromatic Aberration
    chromatic_aberration_enabled: bool,
    chromatic_aberration_intensity: f32,
    
    // Vignette
    vignette_enabled: bool,
    vignette_intensity: f32,
    vignette_smoothness: f32,
    
    // Film Grain
    film_grain_enabled: bool,
    film_grain_intensity: f32,
    
    // Color Grading
    exposure: f32,
    contrast: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    
    // Tone Mapping
    tone_mapping: ToneMapping,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Clone, Copy, PartialEq)]
enum ToneMapping {
    None,
    Reinhard,
    Filmic,
    ACES,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for PostProcessingPanel {
    fn default() -> Self {
        Self {
            bloom_enabled: false,
            bloom_intensity: 0.5,
            bloom_threshold: 1.0,
            bloom_radius: 5.0,
            dof_enabled: false,
            dof_focus_distance: 10.0,
            dof_aperture: 2.8,
            dof_focal_length: 50.0,
            motion_blur_enabled: false,
            motion_blur_intensity: 0.5,
            motion_blur_samples: 8,
            chromatic_aberration_enabled: false,
            chromatic_aberration_intensity: 0.5,
            vignette_enabled: false,
            vignette_intensity: 0.5,
            vignette_smoothness: 0.5,
            film_grain_enabled: false,
            film_grain_intensity: 0.1,
            exposure: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            tone_mapping: ToneMapping::ACES,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl PostProcessingPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("✨ Post-Processing Effects");
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Bloom
            ui.collapsing("Bloom", |ui| {
                ui.checkbox(&mut self.bloom_enabled, "Enable");
                if self.bloom_enabled {
                    ui.add(egui::Slider::new(&mut self.bloom_intensity, 0.0..=2.0).text("Intensity"));
                    ui.add(egui::Slider::new(&mut self.bloom_threshold, 0.0..=5.0).text("Threshold"));
                    ui.add(egui::Slider::new(&mut self.bloom_radius, 1.0..=10.0).text("Radius"));
                }
            });
            
            ui.add_space(8.0);
            
            // Depth of Field
            ui.collapsing("Depth of Field", |ui| {
                ui.checkbox(&mut self.dof_enabled, "Enable");
                if self.dof_enabled {
                    ui.add(egui::Slider::new(&mut self.dof_focus_distance, 0.1..=100.0)
                        .text("Focus Distance")
                        .logarithmic(true));
                    ui.add(egui::Slider::new(&mut self.dof_aperture, 0.5..=22.0).text("Aperture (f-stop)"));
                    ui.add(egui::Slider::new(&mut self.dof_focal_length, 10.0..=200.0).text("Focal Length (mm)"));
                }
            });
            
            ui.add_space(8.0);
            
            // Motion Blur
            ui.collapsing("Motion Blur", |ui| {
                ui.checkbox(&mut self.motion_blur_enabled, "Enable");
                if self.motion_blur_enabled {
                    ui.add(egui::Slider::new(&mut self.motion_blur_intensity, 0.0..=1.0).text("Intensity"));
                    ui.add(egui::Slider::new(&mut self.motion_blur_samples, 4..=32).text("Samples"));
                }
            });
            
            ui.add_space(8.0);
            
            // Chromatic Aberration
            ui.collapsing("Chromatic Aberration", |ui| {
                ui.checkbox(&mut self.chromatic_aberration_enabled, "Enable");
                if self.chromatic_aberration_enabled {
                    ui.add(egui::Slider::new(&mut self.chromatic_aberration_intensity, 0.0..=1.0)
                        .text("Intensity"));
                }
            });
            
            ui.add_space(8.0);
            
            // Vignette
            ui.collapsing("Vignette", |ui| {
                ui.checkbox(&mut self.vignette_enabled, "Enable");
                if self.vignette_enabled {
                    ui.add(egui::Slider::new(&mut self.vignette_intensity, 0.0..=1.0).text("Intensity"));
                    ui.add(egui::Slider::new(&mut self.vignette_smoothness, 0.0..=1.0).text("Smoothness"));
                }
            });
            
            ui.add_space(8.0);
            
            // Film Grain
            ui.collapsing("Film Grain", |ui| {
                ui.checkbox(&mut self.film_grain_enabled, "Enable");
                if self.film_grain_enabled {
                    ui.add(egui::Slider::new(&mut self.film_grain_intensity, 0.0..=1.0).text("Intensity"));
                }
            });
            
            ui.add_space(8.0);
            
            // Color Grading
            ui.collapsing("Color Grading", |ui| {
                ui.add(egui::Slider::new(&mut self.exposure, 0.1..=10.0)
                    .text("Exposure")
                    .logarithmic(true));
                ui.add(egui::Slider::new(&mut self.contrast, 0.0..=2.0).text("Contrast"));
                ui.add(egui::Slider::new(&mut self.saturation, 0.0..=2.0).text("Saturation"));
                ui.add(egui::Slider::new(&mut self.temperature, -1.0..=1.0).text("Temperature"));
                ui.add(egui::Slider::new(&mut self.tint, -1.0..=1.0).text("Tint"));
            });
            
            ui.add_space(8.0);
            
            // Tone Mapping
            ui.collapsing("Tone Mapping", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.radio_value(&mut self.tone_mapping, ToneMapping::None, "None");
                    ui.radio_value(&mut self.tone_mapping, ToneMapping::Reinhard, "Reinhard");
                    ui.radio_value(&mut self.tone_mapping, ToneMapping::Filmic, "Filmic");
                    ui.radio_value(&mut self.tone_mapping, ToneMapping::ACES, "ACES");
                });
            });
            
            ui.add_space(16.0);
            
            // Actions
            ui.horizontal(|ui| {
                if ui.button("💾 Save Preset").clicked() {
                    self.save_preset();
                }
                if ui.button("📂 Load Preset").clicked() {
                    self.load_preset();
                }
                if ui.button("🔄 Reset to Default").clicked() {
                    *self = Self::default();
                }
            });
        });
    }
    
    fn save_preset(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Post-Processing Preset", &["wjpp"])
            .save_file()
        {
            println!("💾 Saving post-processing preset to: {}", path.display());
            // TODO: Serialize and save
        }
    }
    
    fn load_preset(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Post-Processing Preset", &["wjpp"])
            .pick_file()
        {
            println!("📂 Loading post-processing preset from: {}", path.display());
            // TODO: Load and deserialize
        }
    }
}

