// PBR Material Editor Panel
// Visual editor for Physically-Based Rendering materials

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Ui};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct PBRMaterialEditorPanel {
    // Material properties
    albedo_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    emissive_color: [f32; 3],
    emissive_strength: f32,
    normal_strength: f32,
    ao_strength: f32,
    
    // Texture paths
    albedo_texture: Option<String>,
    metallic_texture: Option<String>,
    roughness_texture: Option<String>,
    normal_texture: Option<String>,
    ao_texture: Option<String>,
    emissive_texture: Option<String>,
    
    // Alpha mode
    alpha_mode: AlphaMode,
    alpha_cutoff: f32,
    
    // Material preview
    show_preview: bool,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Clone, Copy, PartialEq)]
enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for PBRMaterialEditorPanel {
    fn default() -> Self {
        Self {
            albedo_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive_color: [0.0, 0.0, 0.0],
            emissive_strength: 0.0,
            normal_strength: 1.0,
            ao_strength: 1.0,
            albedo_texture: None,
            metallic_texture: None,
            roughness_texture: None,
            normal_texture: None,
            ao_texture: None,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            show_preview: true,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl PBRMaterialEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("🎨 PBR Material Editor");
        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Base Color
            ui.collapsing("Base Color", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Albedo:");
                    let mut color = Color32::from_rgba_premultiplied(
                        (self.albedo_color[0] * 255.0) as u8,
                        (self.albedo_color[1] * 255.0) as u8,
                        (self.albedo_color[2] * 255.0) as u8,
                        (self.albedo_color[3] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        let [r, g, b, a] = color.to_array();
                        self.albedo_color = [
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                            a as f32 / 255.0,
                        ];
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Texture:");
                    if let Some(path) = &self.albedo_texture {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.albedo_texture = None;
                        }
                    } else {
                        if ui.button("📁 Load...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg", "tga", "bmp"])
                                .pick_file()
                            {
                                self.albedo_texture = Some(path.display().to_string());
                            }
                        }
                    }
                });
            });
            
            ui.add_space(8.0);
            
            // Metallic & Roughness
            ui.collapsing("Metallic & Roughness", |ui| {
                ui.add(egui::Slider::new(&mut self.metallic, 0.0..=1.0).text("Metallic"));
                ui.add(egui::Slider::new(&mut self.roughness, 0.0..=1.0).text("Roughness"));
                
                ui.horizontal(|ui| {
                    ui.label("Metallic Texture:");
                    if let Some(path) = &self.metallic_texture {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.metallic_texture = None;
                        }
                    } else {
                        if ui.button("📁 Load...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.metallic_texture = Some(path.display().to_string());
                            }
                        }
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Roughness Texture:");
                    if let Some(path) = &self.roughness_texture {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.roughness_texture = None;
                        }
                    } else {
                        if ui.button("📁 Load...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.roughness_texture = Some(path.display().to_string());
                            }
                        }
                    }
                });
            });
            
            ui.add_space(8.0);
            
            // Normal Map
            ui.collapsing("Normal Map", |ui| {
                ui.add(egui::Slider::new(&mut self.normal_strength, 0.0..=2.0).text("Strength"));
                
                ui.horizontal(|ui| {
                    ui.label("Normal Texture:");
                    if let Some(path) = &self.normal_texture {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.normal_texture = None;
                        }
                    } else {
                        if ui.button("📁 Load...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.normal_texture = Some(path.display().to_string());
                            }
                        }
                    }
                });
            });
            
            ui.add_space(8.0);
            
            // Ambient Occlusion
            ui.collapsing("Ambient Occlusion", |ui| {
                ui.add(egui::Slider::new(&mut self.ao_strength, 0.0..=1.0).text("Strength"));
                
                ui.horizontal(|ui| {
                    ui.label("AO Texture:");
                    if let Some(path) = &self.ao_texture {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.ao_texture = None;
                        }
                    } else {
                        if ui.button("📁 Load...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.ao_texture = Some(path.display().to_string());
                            }
                        }
                    }
                });
            });
            
            ui.add_space(8.0);
            
            // Emissive
            ui.collapsing("Emissive", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let mut color = Color32::from_rgb(
                        (self.emissive_color[0] * 255.0) as u8,
                        (self.emissive_color[1] * 255.0) as u8,
                        (self.emissive_color[2] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        let [r, g, b, _] = color.to_array();
                        self.emissive_color = [
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                        ];
                    }
                });
                
                ui.add(egui::Slider::new(&mut self.emissive_strength, 0.0..=10.0).text("Strength"));
                
                ui.horizontal(|ui| {
                    ui.label("Emissive Texture:");
                    if let Some(path) = &self.emissive_texture {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.emissive_texture = None;
                        }
                    } else {
                        if ui.button("📁 Load...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.emissive_texture = Some(path.display().to_string());
                            }
                        }
                    }
                });
            });
            
            ui.add_space(8.0);
            
            // Alpha Mode
            ui.collapsing("Transparency", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Alpha Mode:");
                    ui.radio_value(&mut self.alpha_mode, AlphaMode::Opaque, "Opaque");
                    ui.radio_value(&mut self.alpha_mode, AlphaMode::Mask, "Mask");
                    ui.radio_value(&mut self.alpha_mode, AlphaMode::Blend, "Blend");
                });
                
                if self.alpha_mode == AlphaMode::Mask {
                    ui.add(egui::Slider::new(&mut self.alpha_cutoff, 0.0..=1.0).text("Cutoff"));
                }
            });
            
            ui.add_space(16.0);
            
            // Preview
            ui.checkbox(&mut self.show_preview, "Show Preview");
            
            if self.show_preview {
                ui.separator();
                ui.label("Material Preview:");
                // TODO: Render 3D preview sphere with material
                ui.label("(3D preview coming soon)");
            }
            
            ui.add_space(16.0);
            
            // Actions
            ui.horizontal(|ui| {
                if ui.button("💾 Save Material").clicked() {
                    self.save_material();
                }
                if ui.button("📂 Load Material").clicked() {
                    self.load_material();
                }
                if ui.button("🔄 Reset").clicked() {
                    *self = Self::default();
                }
            });
        });
    }
    
    fn save_material(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Material", &["wjmat"])
            .save_file()
        {
            // TODO: Serialize and save material
            println!("💾 Saving material to: {}", path.display());
        }
    }
    
    fn load_material(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Material", &["wjmat"])
            .pick_file()
        {
            // TODO: Load and deserialize material
            println!("📂 Loading material from: {}", path.display());
        }
    }
}

