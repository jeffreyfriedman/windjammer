// Particle System Editor Panel
// Visual editor for creating and configuring particle effects

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Ui};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct ParticleEditorPanel {
    // Emitter settings
    emit_rate: f32,
    max_particles: u32,
    lifetime_min: f32,
    lifetime_max: f32,
    
    // Spawn properties
    spawn_position: [f32; 3],
    spawn_radius: f32,
    spawn_shape: SpawnShape,
    
    // Particle properties
    start_size: f32,
    end_size: f32,
    start_color: [f32; 4],
    end_color: [f32; 4],
    
    // Physics
    gravity: [f32; 3],
    velocity_min: [f32; 3],
    velocity_max: [f32; 3],
    drag: f32,
    
    // Rendering
    blend_mode: BlendMode,
    texture_path: Option<String>,
    
    // Preview
    show_preview: bool,
    preview_playing: bool,
    
    // Presets
    selected_preset: Option<String>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Clone, Copy, PartialEq)]
enum SpawnShape {
    Point,
    Sphere,
    Box,
    Cone,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Clone, Copy, PartialEq)]
enum BlendMode {
    Alpha,
    Additive,
    Multiply,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for ParticleEditorPanel {
    fn default() -> Self {
        Self {
            emit_rate: 100.0,
            max_particles: 1000,
            lifetime_min: 1.0,
            lifetime_max: 3.0,
            spawn_position: [0.0, 0.0, 0.0],
            spawn_radius: 1.0,
            spawn_shape: SpawnShape::Sphere,
            start_size: 0.1,
            end_size: 0.5,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 0.0],
            gravity: [0.0, -9.8, 0.0],
            velocity_min: [-1.0, 0.0, -1.0],
            velocity_max: [1.0, 2.0, 1.0],
            drag: 0.1,
            blend_mode: BlendMode::Alpha,
            texture_path: None,
            show_preview: true,
            preview_playing: false,
            selected_preset: None,
        }
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
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    self.save_particle_system();
                }
                if ui.button("📂 Load").clicked() {
                    self.load_particle_system();
                }
                if ui.button("🔄 Reset").clicked() {
                    *self = Self::default();
                }
            });
            
            ui.add_space(10.0);
            
            // Presets
            ui.collapsing("Presets", |ui| {
                let presets = vec![
                    "Fire",
                    "Smoke",
                    "Explosion",
                    "Magic Sparkles",
                    "Rain",
                    "Snow",
                    "Dust",
                    "Blood Splatter",
                ];
                
                for preset in presets {
                    if ui.button(preset).clicked() {
                        self.load_preset(preset);
                    }
                }
            });
            
            ui.add_space(10.0);
            
            // Emitter Settings
            ui.collapsing("Emitter Settings", |ui| {
                ui.add(egui::Slider::new(&mut self.emit_rate, 1.0..=1000.0)
                    .text("Emit Rate")
                    .suffix(" particles/sec"));
                
                ui.add(egui::Slider::new(&mut self.max_particles, 10..=10000)
                    .text("Max Particles")
                    .logarithmic(true));
                
                ui.horizontal(|ui| {
                    ui.label("Lifetime:");
                    ui.add(egui::DragValue::new(&mut self.lifetime_min)
                        .speed(0.1)
                        .range(0.1..=100.0)
                        .suffix(" s"));
                    ui.label("to");
                    ui.add(egui::DragValue::new(&mut self.lifetime_max)
                        .speed(0.1)
                        .range(0.1..=100.0)
                        .suffix(" s"));
                });
            });
            
            ui.add_space(10.0);
            
            // Spawn Properties
            ui.collapsing("Spawn Properties", |ui| {
                ui.label("Position:");
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.spawn_position[0]).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.spawn_position[1]).speed(0.1));
                    ui.label("Z:");
                    ui.add(egui::DragValue::new(&mut self.spawn_position[2]).speed(0.1));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Shape:");
                    ui.radio_value(&mut self.spawn_shape, SpawnShape::Point, "Point");
                    ui.radio_value(&mut self.spawn_shape, SpawnShape::Sphere, "Sphere");
                    ui.radio_value(&mut self.spawn_shape, SpawnShape::Box, "Box");
                    ui.radio_value(&mut self.spawn_shape, SpawnShape::Cone, "Cone");
                });
                
                ui.add(egui::Slider::new(&mut self.spawn_radius, 0.0..=10.0)
                    .text("Spawn Radius"));
            });
            
            ui.add_space(10.0);
            
            // Particle Properties
            ui.collapsing("Particle Properties", |ui| {
                ui.label("Size:");
                ui.horizontal(|ui| {
                    ui.label("Start:");
                    ui.add(egui::DragValue::new(&mut self.start_size)
                        .speed(0.01)
                        .range(0.01..=10.0));
                    ui.label("End:");
                    ui.add(egui::DragValue::new(&mut self.end_size)
                        .speed(0.01)
                        .range(0.01..=10.0));
                });
                
                ui.label("Start Color:");
                let mut start_color = Color32::from_rgba_premultiplied(
                    (self.start_color[0] * 255.0) as u8,
                    (self.start_color[1] * 255.0) as u8,
                    (self.start_color[2] * 255.0) as u8,
                    (self.start_color[3] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut start_color).changed() {
                    let [r, g, b, a] = start_color.to_array();
                    self.start_color = [
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        a as f32 / 255.0,
                    ];
                }
                
                ui.label("End Color:");
                let mut end_color = Color32::from_rgba_premultiplied(
                    (self.end_color[0] * 255.0) as u8,
                    (self.end_color[1] * 255.0) as u8,
                    (self.end_color[2] * 255.0) as u8,
                    (self.end_color[3] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut end_color).changed() {
                    let [r, g, b, a] = end_color.to_array();
                    self.end_color = [
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        a as f32 / 255.0,
                    ];
                }
            });
            
            ui.add_space(10.0);
            
            // Physics
            ui.collapsing("Physics", |ui| {
                ui.label("Gravity:");
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.gravity[0]).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.gravity[1]).speed(0.1));
                    ui.label("Z:");
                    ui.add(egui::DragValue::new(&mut self.gravity[2]).speed(0.1));
                });
                
                ui.label("Velocity Min:");
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.velocity_min[0]).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.velocity_min[1]).speed(0.1));
                    ui.label("Z:");
                    ui.add(egui::DragValue::new(&mut self.velocity_min[2]).speed(0.1));
                });
                
                ui.label("Velocity Max:");
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.velocity_max[0]).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.velocity_max[1]).speed(0.1));
                    ui.label("Z:");
                    ui.add(egui::DragValue::new(&mut self.velocity_max[2]).speed(0.1));
                });
                
                ui.add(egui::Slider::new(&mut self.drag, 0.0..=1.0).text("Drag"));
            });
            
            ui.add_space(10.0);
            
            // Rendering
            ui.collapsing("Rendering", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Blend Mode:");
                    ui.radio_value(&mut self.blend_mode, BlendMode::Alpha, "Alpha");
                    ui.radio_value(&mut self.blend_mode, BlendMode::Additive, "Additive");
                    ui.radio_value(&mut self.blend_mode, BlendMode::Multiply, "Multiply");
                });
                
                ui.horizontal(|ui| {
                    ui.label("Texture:");
                    if let Some(path) = &self.texture_path {
                        ui.label(path);
                        if ui.small_button("✖").clicked() {
                            self.texture_path = None;
                        }
                    } else {
                        if ui.button("📁 Load Texture...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.texture_path = Some(path.display().to_string());
                            }
                        }
                    }
                });
            });
            
            ui.add_space(10.0);
            
            // Preview
            ui.checkbox(&mut self.show_preview, "Show Preview");
            
            if self.show_preview {
                ui.separator();
                ui.group(|ui| {
                    ui.heading("Preview");
                    
                    ui.horizontal(|ui| {
                        if ui.button(if self.preview_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                            self.preview_playing = !self.preview_playing;
                        }
                        if ui.button("⏹ Stop").clicked() {
                            self.preview_playing = false;
                        }
                        if ui.button("🔄 Restart").clicked() {
                            self.preview_playing = true;
                        }
                    });
                    
                    // Preview area (placeholder)
                    let available_size = ui.available_size();
                    let preview_size = egui::vec2(available_size.x - 20.0, 200.0);
                    let (response, painter) = ui.allocate_painter(preview_size, egui::Sense::hover());
                    let rect = response.rect;
                    
                    // Background
                    painter.rect_filled(rect, 2.0, Color32::from_rgb(20, 20, 20));
                    
                    // Placeholder text
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if self.preview_playing {
                            "▶ Particle Preview (Simulated)"
                        } else {
                            "⏸ Preview Paused"
                        },
                        egui::FontId::proportional(14.0),
                        Color32::GRAY,
                    );
                    
                    ui.label("(3D preview would render here)");
                });
            }
        });
    }
    
    fn save_particle_system(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Particle System", &["wjparticle"])
            .save_file()
        {
            println!("💾 Saving particle system to: {}", path.display());
            // TODO: Serialize and save
        }
    }
    
    fn load_particle_system(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Particle System", &["wjparticle"])
            .pick_file()
        {
            println!("📂 Loading particle system from: {}", path.display());
            // TODO: Load and deserialize
        }
    }
    
    fn load_preset(&mut self, preset: &str) {
        match preset {
            "Fire" => {
                self.emit_rate = 200.0;
                self.start_color = [1.0, 0.5, 0.0, 1.0];
                self.end_color = [1.0, 0.0, 0.0, 0.0];
                self.velocity_min = [-0.5, 1.0, -0.5];
                self.velocity_max = [0.5, 3.0, 0.5];
                self.blend_mode = BlendMode::Additive;
            }
            "Smoke" => {
                self.emit_rate = 50.0;
                self.start_color = [0.3, 0.3, 0.3, 0.8];
                self.end_color = [0.5, 0.5, 0.5, 0.0];
                self.start_size = 0.2;
                self.end_size = 1.0;
                self.velocity_min = [-0.2, 0.5, -0.2];
                self.velocity_max = [0.2, 1.5, 0.2];
            }
            "Explosion" => {
                self.emit_rate = 1000.0;
                self.lifetime_min = 0.5;
                self.lifetime_max = 1.5;
                self.start_color = [1.0, 0.8, 0.0, 1.0];
                self.end_color = [0.5, 0.0, 0.0, 0.0];
                self.velocity_min = [-5.0, -5.0, -5.0];
                self.velocity_max = [5.0, 5.0, 5.0];
                self.blend_mode = BlendMode::Additive;
            }
            _ => {}
        }
        
        self.selected_preset = Some(preset.to_string());
    }
}
