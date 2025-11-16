// Terrain Editor Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Rect, Sense, Ui, Vec2};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::path::PathBuf;

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug)]
pub struct TerrainData {
    pub width: usize,
    pub height: usize,
    pub heightmap: Vec<f32>,
    pub max_height: f32,
}

impl TerrainData {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            heightmap: vec![0.0; width * height],
            max_height: 100.0,
        }
    }

    pub fn get_height(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.heightmap[y * self.width + x]
        } else {
            0.0
        }
    }

    pub fn set_height(&mut self, x: usize, y: usize, height: f32) {
        if x < self.width && y < self.height {
            self.heightmap[y * self.width + x] = height.clamp(0.0, self.max_height);
        }
    }

    pub fn apply_brush(&mut self, center_x: f32, center_y: f32, radius: f32, strength: f32, mode: BrushMode) {
        let cx = center_x as i32;
        let cy = center_y as i32;
        let r = radius as i32;

        for dy in -r..=r {
            for dx in -r..=r {
                let x = (cx + dx).max(0).min(self.width as i32 - 1) as usize;
                let y = (cy + dy).max(0).min(self.height as i32 - 1) as usize;

                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist <= radius {
                    let falloff = 1.0 - (dist / radius);
                    let current_height = self.get_height(x, y);
                    
                    let new_height = match mode {
                        BrushMode::Raise => current_height + strength * falloff,
                        BrushMode::Lower => current_height - strength * falloff,
                        BrushMode::Flatten => {
                            let target = self.get_height(cx as usize, cy as usize);
                            current_height + (target - current_height) * strength * falloff
                        }
                        BrushMode::Smooth => {
                            // Average with neighbors
                            let mut sum = current_height;
                            let mut count = 1.0;
                            for ny in -1..=1 {
                                for nx in -1..=1 {
                                    if nx == 0 && ny == 0 { continue; }
                                    let sx = (x as i32 + nx).max(0).min(self.width as i32 - 1) as usize;
                                    let sy = (y as i32 + ny).max(0).min(self.height as i32 - 1) as usize;
                                    sum += self.get_height(sx, sy);
                                    count += 1.0;
                                }
                            }
                            let avg = sum / count;
                            current_height + (avg - current_height) * strength * falloff
                        }
                    };
                    
                    self.set_height(x, y, new_height);
                }
            }
        }
    }

    pub fn generate_perlin(&mut self, scale: f32, octaves: u32, persistence: f32) {
        for y in 0..self.height {
            for x in 0..self.width {
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                let mut noise_value = 0.0;

                for _ in 0..octaves {
                    let sample_x = x as f32 / self.width as f32 * scale * frequency;
                    let sample_y = y as f32 / self.height as f32 * scale * frequency;
                    
                    // Simple pseudo-noise (replace with real Perlin in production)
                    let noise = ((sample_x * 12.9898 + sample_y * 78.233).sin() * 43758.5453).fract();
                    noise_value += noise * amplitude;

                    amplitude *= persistence;
                    frequency *= 2.0;
                }

                let height = (noise_value / 2.0 + 0.5) * self.max_height;
                self.set_height(x, y, height);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BrushMode {
    Raise,
    Lower,
    Flatten,
    Smooth,
}

#[derive(Clone, Debug)]
pub struct TerrainLayer {
    pub name: String,
    pub texture_path: PathBuf,
    pub tiling: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub min_slope: f32,
    pub max_slope: f32,
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct TerrainEditor {
    terrain: TerrainData,
    brush_mode: BrushMode,
    brush_radius: f32,
    brush_strength: f32,
    layers: Vec<TerrainLayer>,
    selected_layer: Option<usize>,
    is_painting: bool,
    view_scale: f32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl TerrainEditor {
    pub fn new() -> Self {
        let mut terrain = TerrainData::new(64, 64);
        terrain.generate_perlin(4.0, 4, 0.5);

        let mut layers = Vec::new();
        layers.push(TerrainLayer {
            name: "Grass".to_string(),
            texture_path: PathBuf::from("textures/grass.png"),
            tiling: 10.0,
            min_height: 0.0,
            max_height: 50.0,
            min_slope: 0.0,
            max_slope: 45.0,
        });
        layers.push(TerrainLayer {
            name: "Rock".to_string(),
            texture_path: PathBuf::from("textures/rock.png"),
            tiling: 5.0,
            min_height: 40.0,
            max_height: 100.0,
            min_slope: 30.0,
            max_slope: 90.0,
        });

        Self {
            terrain,
            brush_mode: BrushMode::Raise,
            brush_radius: 5.0,
            brush_strength: 2.0,
            layers,
            selected_layer: Some(0),
            is_painting: false,
            view_scale: 4.0,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🏔️ Terrain Editor");
            ui.separator();
            if ui.button("🎲 Generate").clicked() {
                self.terrain.generate_perlin(4.0, 4, 0.5);
            }
            if ui.button("🗑️ Clear").clicked() {
                self.terrain = TerrainData::new(self.terrain.width, self.terrain.height);
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Left: Terrain view
            ui.vertical(|ui| {
                ui.set_min_width(500.0);
                self.render_terrain_view(ui);
            });

            ui.separator();

            // Right: Tools and properties
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                self.render_tools(ui);
            });
        });
    }

    fn render_terrain_view(&mut self, ui: &mut Ui) {
        ui.label("Terrain Heightmap");
        
        let size = Vec2::new(
            self.terrain.width as f32 * self.view_scale,
            self.terrain.height as f32 * self.view_scale,
        );

        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = response.rect;

        // Draw heightmap
        for y in 0..self.terrain.height {
            for x in 0..self.terrain.width {
                let height = self.terrain.get_height(x, y);
                let normalized = (height / self.terrain.max_height * 255.0) as u8;
                let color = Color32::from_gray(normalized);

                let pixel_rect = Rect::from_min_size(
                    rect.min + Vec2::new(x as f32 * self.view_scale, y as f32 * self.view_scale),
                    Vec2::splat(self.view_scale),
                );

                painter.rect_filled(pixel_rect, 0.0, color);
            }
        }

        // Handle painting
        if response.dragged() || response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local_pos = pos - rect.min;
                let terrain_x = (local_pos.x / self.view_scale).max(0.0).min(self.terrain.width as f32 - 1.0);
                let terrain_y = (local_pos.y / self.view_scale).max(0.0).min(self.terrain.height as f32 - 1.0);

                self.terrain.apply_brush(
                    terrain_x,
                    terrain_y,
                    self.brush_radius,
                    self.brush_strength,
                    self.brush_mode,
                );
            }
        }

        // Draw brush preview
        if let Some(pos) = response.hover_pos() {
            let local_pos = pos - rect.min;
            let terrain_x = local_pos.x / self.view_scale;
            let terrain_y = local_pos.y / self.view_scale;

            let brush_center = rect.min + Vec2::new(terrain_x * self.view_scale, terrain_y * self.view_scale);
            let brush_radius_pixels = self.brush_radius * self.view_scale;

            painter.circle_stroke(
                brush_center,
                brush_radius_pixels,
                egui::Stroke::new(2.0, Color32::from_rgb(255, 255, 0)),
            );
        }
    }

    fn render_tools(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing("Brush Settings", |ui| {
                ui.label("Brush Mode:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.brush_mode, BrushMode::Raise, "⬆️ Raise");
                    ui.selectable_value(&mut self.brush_mode, BrushMode::Lower, "⬇️ Lower");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.brush_mode, BrushMode::Flatten, "➖ Flatten");
                    ui.selectable_value(&mut self.brush_mode, BrushMode::Smooth, "〰️ Smooth");
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(egui::Slider::new(&mut self.brush_radius, 1.0..=20.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Strength:");
                    ui.add(egui::Slider::new(&mut self.brush_strength, 0.1..=10.0));
                });
            });

            ui.separator();

            ui.collapsing("Terrain Generation", |ui| {
                ui.label("Generate procedural terrain using Perlin noise");
                
                let mut scale = 4.0;
                let mut octaves = 4;
                let mut persistence = 0.5;

                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.add(egui::Slider::new(&mut scale, 1.0..=10.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Octaves:");
                    ui.add(egui::Slider::new(&mut octaves, 1..=8));
                });

                ui.horizontal(|ui| {
                    ui.label("Persistence:");
                    ui.add(egui::Slider::new(&mut persistence, 0.1..=1.0));
                });

                if ui.button("🎲 Generate Terrain").clicked() {
                    self.terrain.generate_perlin(scale, octaves, persistence);
                }
            });

            ui.separator();

            ui.collapsing("Terrain Properties", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.label(format!("{}x{}", self.terrain.width, self.terrain.height));
                });

                ui.horizontal(|ui| {
                    ui.label("Max Height:");
                    ui.add(egui::DragValue::new(&mut self.terrain.max_height).range(1.0..=1000.0));
                });

                ui.horizontal(|ui| {
                    ui.label("View Scale:");
                    ui.add(egui::Slider::new(&mut self.view_scale, 1.0..=10.0));
                });
            });

            ui.separator();

            ui.collapsing("Texture Layers", |ui| {
                let mut to_remove = None;
                for (idx, layer) in self.layers.iter_mut().enumerate() {
                    let is_selected = Some(idx) == self.selected_layer;
                    ui.horizontal(|ui| {
                        if ui.selectable_label(is_selected, &layer.name).clicked() {
                            self.selected_layer = Some(idx);
                        }
                        if ui.button("🗑️").clicked() {
                            to_remove = Some(idx);
                        }
                    });

                    if is_selected {
                        ui.indent(idx, |ui| {
                            ui.text_edit_singleline(&mut layer.name);
                            ui.horizontal(|ui| {
                                ui.label("Tiling:");
                                ui.add(egui::DragValue::new(&mut layer.tiling).range(0.1..=100.0).speed(0.1));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Height Range:");
                                ui.add(egui::DragValue::new(&mut layer.min_height).range(0.0..=self.terrain.max_height));
                                ui.label("-");
                                ui.add(egui::DragValue::new(&mut layer.max_height).range(0.0..=self.terrain.max_height));
                            });
                        });
                    }
                }

                if let Some(idx) = to_remove {
                    self.layers.remove(idx);
                    if self.selected_layer == Some(idx) {
                        self.selected_layer = None;
                    }
                }

                if ui.button("➕ Add Layer").clicked() {
                    self.layers.push(TerrainLayer {
                        name: format!("Layer {}", self.layers.len() + 1),
                        texture_path: PathBuf::new(),
                        tiling: 10.0,
                        min_height: 0.0,
                        max_height: self.terrain.max_height,
                        min_slope: 0.0,
                        max_slope: 90.0,
                    });
                }
            });
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for TerrainEditor {
    fn default() -> Self {
        Self::new()
    }
}
