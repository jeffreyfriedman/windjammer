// Navigation Mesh Editor Panel
// Designed for easy migration to windjammer-ui component framework

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

// ============================================================================
// DATA MODEL (Pure Rust - easily portable to any UI framework)
// ============================================================================

#[derive(Clone, Debug)]
pub struct NavMeshPoly {
    pub id: u32,
    pub vertices: Vec<Pos2>,
    pub neighbors: Vec<u32>,
    pub cost: f32,
    pub walkable: bool,
}

#[derive(Clone, Debug)]
pub struct NavMeshData {
    pub polygons: Vec<NavMeshPoly>,
    pub agent_radius: f32,
    pub agent_height: f32,
    pub max_slope: f32,
    pub step_height: f32,
    next_poly_id: u32,
}

impl NavMeshData {
    pub fn new() -> Self {
        Self {
            polygons: Vec::new(),
            agent_radius: 0.5,
            agent_height: 2.0,
            max_slope: 45.0,
            step_height: 0.5,
            next_poly_id: 1,
        }
    }

    pub fn add_polygon(&mut self, vertices: Vec<Pos2>) -> u32 {
        let id = self.next_poly_id;
        self.next_poly_id += 1;

        self.polygons.push(NavMeshPoly {
            id,
            vertices,
            neighbors: Vec::new(),
            cost: 1.0,
            walkable: true,
        });

        id
    }

    pub fn remove_polygon(&mut self, id: u32) {
        self.polygons.retain(|p| p.id != id);
        // Remove from neighbors
        for poly in &mut self.polygons {
            poly.neighbors.retain(|&n| n != id);
        }
    }

    pub fn connect_polygons(&mut self, id1: u32, id2: u32) {
        if let Some(poly1) = self.polygons.iter_mut().find(|p| p.id == id1) {
            if !poly1.neighbors.contains(&id2) {
                poly1.neighbors.push(id2);
            }
        }
        if let Some(poly2) = self.polygons.iter_mut().find(|p| p.id == id2) {
            if !poly2.neighbors.contains(&id1) {
                poly2.neighbors.push(id1);
            }
        }
    }

    pub fn generate_grid(&mut self, width: usize, height: usize, cell_size: f32) {
        self.polygons.clear();
        
        for y in 0..height {
            for x in 0..width {
                let x_pos = x as f32 * cell_size;
                let y_pos = y as f32 * cell_size;

                let vertices = vec![
                    Pos2::new(x_pos, y_pos),
                    Pos2::new(x_pos + cell_size, y_pos),
                    Pos2::new(x_pos + cell_size, y_pos + cell_size),
                    Pos2::new(x_pos, y_pos + cell_size),
                ];

                let id = self.add_polygon(vertices);

                // Connect to left neighbor
                if x > 0 {
                    let left_id = id - 1;
                    self.connect_polygons(id, left_id);
                }

                // Connect to top neighbor
                if y > 0 {
                    let top_id = id - width as u32;
                    self.connect_polygons(id, top_id);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditMode {
    Select,
    Add,
    Remove,
    Paint,
}

// ============================================================================
// UI PANEL (egui-specific - will be replaced with windjammer-ui components)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct NavMeshEditor {
    navmesh: NavMeshData,
    edit_mode: EditMode,
    selected_poly: Option<u32>,
    new_poly_vertices: Vec<Pos2>,
    view_scale: f32,
    pan_offset: Vec2,
    show_connections: bool,
    show_costs: bool,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl NavMeshEditor {
    pub fn new() -> Self {
        let mut navmesh = NavMeshData::new();
        navmesh.generate_grid(10, 10, 50.0);

        Self {
            navmesh,
            edit_mode: EditMode::Select,
            selected_poly: None,
            new_poly_vertices: Vec::new(),
            view_scale: 1.0,
            pan_offset: Vec2::ZERO,
            show_connections: true,
            show_costs: false,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("🗺️ Navigation Mesh Editor");
            ui.separator();

            ui.selectable_value(&mut self.edit_mode, EditMode::Select, "🖱️ Select");
            ui.selectable_value(&mut self.edit_mode, EditMode::Add, "➕ Add");
            ui.selectable_value(&mut self.edit_mode, EditMode::Remove, "🗑️ Remove");
            ui.selectable_value(&mut self.edit_mode, EditMode::Paint, "🖌️ Paint");

            ui.separator();

            if ui.button("🎲 Generate Grid").clicked() {
                self.navmesh.generate_grid(10, 10, 50.0);
                self.selected_poly = None;
                self.new_poly_vertices.clear();
            }

            if ui.button("🗑️ Clear").clicked() {
                self.navmesh.polygons.clear();
                self.selected_poly = None;
                self.new_poly_vertices.clear();
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Left: NavMesh view
            ui.vertical(|ui| {
                ui.set_min_width(600.0);
                self.render_navmesh_view(ui);
            });

            ui.separator();

            // Right: Properties and tools
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                self.render_properties(ui);
            });
        });
    }

    fn render_navmesh_view(&mut self, ui: &mut Ui) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), 500.0),
            Sense::click_and_drag(),
        );

        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_gray(30));

        // Handle panning
        if response.dragged() && self.edit_mode == EditMode::Select {
            self.pan_offset += response.drag_delta();
        }

        // Draw connections first
        if self.show_connections {
            for poly in &self.navmesh.polygons {
                let center = self.poly_center(poly) + self.pan_offset;
                for &neighbor_id in &poly.neighbors {
                    if let Some(neighbor) = self.navmesh.polygons.iter().find(|p| p.id == neighbor_id) {
                        let neighbor_center = self.poly_center(neighbor) + self.pan_offset;
                        painter.line_segment(
                            [center, neighbor_center],
                            Stroke::new(1.0, Color32::from_rgb(100, 100, 150)),
                        );
                    }
                }
            }
        }

        // Draw polygons
        for poly in &self.navmesh.polygons {
            let is_selected = Some(poly.id) == self.selected_poly;
            
            let fill_color = if !poly.walkable {
                Color32::from_rgb(100, 50, 50)
            } else if is_selected {
                Color32::from_rgb(100, 150, 255)
            } else {
                Color32::from_rgb(50, 100, 50)
            };

            let stroke_color = if is_selected {
                Color32::WHITE
            } else {
                Color32::from_gray(100)
            };

            // Transform vertices
            let transformed_verts: Vec<Pos2> = poly
                .vertices
                .iter()
                .map(|v| *v + self.pan_offset)
                .collect();

            // Draw filled polygon
            painter.add(egui::Shape::convex_polygon(
                transformed_verts.clone(),
                fill_color,
                Stroke::new(1.0, stroke_color),
            ));

            // Draw cost if enabled
            if self.show_costs {
                let center = self.poly_center(poly) + self.pan_offset;
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("{:.1}", poly.cost),
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }

            // Handle poly interaction
            if self.edit_mode == EditMode::Select {
                let poly_rect = Rect::from_min_max(
                    transformed_verts.iter().fold(Pos2::new(f32::MAX, f32::MAX), |acc, v| {
                        Pos2::new(acc.x.min(v.x), acc.y.min(v.y))
                    }),
                    transformed_verts.iter().fold(Pos2::new(f32::MIN, f32::MIN), |acc, v| {
                        Pos2::new(acc.x.max(v.x), acc.y.max(v.y))
                    }),
                );

                let poly_response = ui.interact(poly_rect, ui.id().with(poly.id), Sense::click());
                if poly_response.clicked() {
                    self.selected_poly = Some(poly.id);
                }
            }
        }

        // Draw new polygon being created
        if self.edit_mode == EditMode::Add && !self.new_poly_vertices.is_empty() {
            let transformed_verts: Vec<Pos2> = self
                .new_poly_vertices
                .iter()
                .map(|v| *v + self.pan_offset)
                .collect();

            for vert in &transformed_verts {
                painter.circle_filled(*vert, 4.0, Color32::from_rgb(255, 255, 0));
            }

            if transformed_verts.len() > 1 {
                for i in 0..transformed_verts.len() {
                    let next = (i + 1) % transformed_verts.len();
                    painter.line_segment(
                        [transformed_verts[i], transformed_verts[next]],
                        Stroke::new(2.0, Color32::from_rgb(255, 255, 0)),
                    );
                }
            }
        }

        // Handle clicks for adding vertices
        if self.edit_mode == EditMode::Add && response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local_pos = (pos - rect.min - self.pan_offset).to_pos2();
                self.new_poly_vertices.push(local_pos);
            }
        }

        // Handle remove mode
        if self.edit_mode == EditMode::Remove && response.clicked() {
            if let Some(poly_id) = self.selected_poly {
                self.navmesh.remove_polygon(poly_id);
                self.selected_poly = None;
            }
        }
    }

    fn poly_center(&self, poly: &NavMeshPoly) -> Pos2 {
        let sum = poly.vertices.iter().fold(Vec2::ZERO, |acc, v| acc + v.to_vec2());
        (sum / poly.vertices.len() as f32).to_pos2()
    }

    fn render_properties(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing("Agent Settings", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut self.navmesh.agent_radius).range(0.1..=5.0).speed(0.1));
                });

                ui.horizontal(|ui| {
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut self.navmesh.agent_height).range(0.5..=10.0).speed(0.1));
                });

                ui.horizontal(|ui| {
                    ui.label("Max Slope (°):");
                    ui.add(egui::DragValue::new(&mut self.navmesh.max_slope).range(0.0..=90.0).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Step Height:");
                    ui.add(egui::DragValue::new(&mut self.navmesh.step_height).range(0.0..=2.0).speed(0.1));
                });
            });

            ui.separator();

            ui.collapsing("View Options", |ui| {
                ui.checkbox(&mut self.show_connections, "Show Connections");
                ui.checkbox(&mut self.show_costs, "Show Costs");
                
                ui.horizontal(|ui| {
                    ui.label("Zoom:");
                    ui.add(egui::Slider::new(&mut self.view_scale, 0.5..=2.0));
                });
            });

            ui.separator();

            if let Some(poly_id) = self.selected_poly {
                if let Some(poly) = self.navmesh.polygons.iter_mut().find(|p| p.id == poly_id) {
                    ui.label("📍 Selected Polygon");
                    ui.separator();

                    ui.label(format!("ID: {}", poly.id));
                    ui.label(format!("Vertices: {}", poly.vertices.len()));
                    ui.label(format!("Neighbors: {}", poly.neighbors.len()));

                    ui.separator();

                    ui.checkbox(&mut poly.walkable, "Walkable");

                    ui.horizontal(|ui| {
                        ui.label("Cost:");
                        ui.add(egui::DragValue::new(&mut poly.cost).range(0.1..=10.0).speed(0.1));
                    });

                    ui.separator();

                    if ui.button("🗑️ Delete Polygon").clicked() {
                        let id_to_remove = poly_id;
                        self.selected_poly = None;
                        self.navmesh.remove_polygon(id_to_remove);
                    }
                }
            } else {
                ui.label("No polygon selected");
            }

            ui.separator();

            ui.collapsing("Edit Mode Help", |ui| {
                match self.edit_mode {
                    EditMode::Select => {
                        ui.label("🖱️ Select Mode");
                        ui.label("• Click polygons to select");
                        ui.label("• Drag to pan view");
                    }
                    EditMode::Add => {
                        ui.label("➕ Add Mode");
                        ui.label("• Click to add vertices");
                        ui.label("• Press 'Complete' to finish");
                        if self.new_poly_vertices.len() >= 3 {
                            if ui.button("✅ Complete Polygon").clicked() {
                                self.navmesh.add_polygon(self.new_poly_vertices.clone());
                                self.new_poly_vertices.clear();
                            }
                        }
                        if !self.new_poly_vertices.is_empty() {
                            if ui.button("❌ Cancel").clicked() {
                                self.new_poly_vertices.clear();
                            }
                        }
                    }
                    EditMode::Remove => {
                        ui.label("🗑️ Remove Mode");
                        ui.label("• Select and click to remove");
                    }
                    EditMode::Paint => {
                        ui.label("🖌️ Paint Mode");
                        ui.label("• Click to toggle walkable");
                        ui.label("• (Not yet implemented)");
                    }
                }
            });

            ui.separator();

            ui.label(format!("Total Polygons: {}", self.navmesh.polygons.len()));
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for NavMeshEditor {
    fn default() -> Self {
        Self::new()
    }
}
