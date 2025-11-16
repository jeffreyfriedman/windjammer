// Scene Gizmos Module
// Provides visual manipulation tools for 3D scene editing

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

// ============================================================================
// DATA MODEL (Pure Rust - easily portable)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    XYZ, // For uniform scale
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 3], // Euler angles in degrees
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    pub fn with_rotation(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rotation = [x, y, z];
        self
    }

    pub fn with_scale(mut self, x: f32, y: f32, z: f32) -> Self {
        self.scale = [x, y, z];
        self
    }

    pub fn translate(&mut self, delta: [f32; 3]) {
        self.position[0] += delta[0];
        self.position[1] += delta[1];
        self.position[2] += delta[2];
    }

    pub fn rotate(&mut self, delta: [f32; 3]) {
        self.rotation[0] += delta[0];
        self.rotation[1] += delta[1];
        self.rotation[2] += delta[2];
    }

    pub fn scale_by(&mut self, factor: [f32; 3]) {
        self.scale[0] *= factor[0];
        self.scale[1] *= factor[1];
        self.scale[2] *= factor[2];
    }
}

// ============================================================================
// GIZMO SYSTEM (egui-specific - will be replaced with windjammer-ui)
// ============================================================================

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct GizmoSystem {
    mode: GizmoMode,
    active_axis: Option<GizmoAxis>,
    drag_start_pos: Option<Pos2>,
    drag_start_transform: Option<Transform>,
    gizmo_size: f32,
    snap_enabled: bool,
    snap_translate: f32,
    snap_rotate: f32,
    snap_scale: f32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl GizmoSystem {
    pub fn new() -> Self {
        Self {
            mode: GizmoMode::Translate,
            active_axis: None,
            drag_start_pos: None,
            drag_start_transform: None,
            gizmo_size: 100.0,
            snap_enabled: false,
            snap_translate: 0.5,
            snap_rotate: 15.0,
            snap_scale: 0.1,
        }
    }

    pub fn set_mode(&mut self, mode: GizmoMode) {
        self.mode = mode;
        self.active_axis = None;
    }

    pub fn get_mode(&self) -> GizmoMode {
        self.mode
    }

    pub fn set_snap_enabled(&mut self, enabled: bool) {
        self.snap_enabled = enabled;
    }

    pub fn is_snap_enabled(&self) -> bool {
        self.snap_enabled
    }

    /// Render gizmo and handle interaction
    /// Returns true if the transform was modified
    pub fn render(&mut self, ui: &mut Ui, transform: &mut Transform, center: Pos2) -> bool {
        let mut modified = false;

        match self.mode {
            GizmoMode::Translate => {
                modified = self.render_translate_gizmo(ui, transform, center);
            }
            GizmoMode::Rotate => {
                modified = self.render_rotate_gizmo(ui, transform, center);
            }
            GizmoMode::Scale => {
                modified = self.render_scale_gizmo(ui, transform, center);
            }
        }

        modified
    }

    fn render_translate_gizmo(&mut self, ui: &mut Ui, transform: &mut Transform, center: Pos2) -> bool {
        let mut modified = false;
        let size = self.gizmo_size;

        // X axis (red)
        if self.render_axis_arrow(ui, center, Vec2::new(size, 0.0), Color32::RED, GizmoAxis::X) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_translate } else { 0.0 };
                let movement = self.apply_snap(delta.x * 0.01, snap);
                transform.translate([movement, 0.0, 0.0]);
                modified = true;
            }
        }

        // Y axis (green)
        if self.render_axis_arrow(ui, center, Vec2::new(0.0, -size), Color32::GREEN, GizmoAxis::Y) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_translate } else { 0.0 };
                let movement = self.apply_snap(-delta.y * 0.01, snap);
                transform.translate([0.0, movement, 0.0]);
                modified = true;
            }
        }

        // Z axis (blue) - diagonal for 2D representation
        if self.render_axis_arrow(ui, center, Vec2::new(size * 0.5, size * 0.5), Color32::BLUE, GizmoAxis::Z) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_translate } else { 0.0 };
                let movement = self.apply_snap((delta.x + delta.y) * 0.01, snap);
                transform.translate([0.0, 0.0, movement]);
                modified = true;
            }
        }

        // XY plane (yellow square)
        self.render_plane_handle(ui, center, Vec2::new(size * 0.3, -size * 0.3), Color32::YELLOW, GizmoAxis::XY);

        // Center sphere for free movement
        self.render_center_sphere(ui, center);

        modified
    }

    fn render_rotate_gizmo(&mut self, ui: &mut Ui, transform: &mut Transform, center: Pos2) -> bool {
        let mut modified = false;
        let radius = self.gizmo_size;

        // X axis rotation (red circle)
        if self.render_rotation_circle(ui, center, radius, Color32::RED, GizmoAxis::X) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_rotate } else { 0.0 };
                let rotation = self.apply_snap(delta.x * 0.5, snap);
                transform.rotate([rotation, 0.0, 0.0]);
                modified = true;
            }
        }

        // Y axis rotation (green circle)
        if self.render_rotation_circle(ui, center, radius * 0.8, Color32::GREEN, GizmoAxis::Y) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_rotate } else { 0.0 };
                let rotation = self.apply_snap(delta.y * 0.5, snap);
                transform.rotate([0.0, rotation, 0.0]);
                modified = true;
            }
        }

        // Z axis rotation (blue circle)
        if self.render_rotation_circle(ui, center, radius * 0.6, Color32::BLUE, GizmoAxis::Z) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_rotate } else { 0.0 };
                let rotation = self.apply_snap((delta.x + delta.y) * 0.5, snap);
                transform.rotate([0.0, 0.0, rotation]);
                modified = true;
            }
        }

        modified
    }

    fn render_scale_gizmo(&mut self, ui: &mut Ui, transform: &mut Transform, center: Pos2) -> bool {
        let mut modified = false;
        let size = self.gizmo_size;

        // X axis (red)
        if self.render_axis_line_with_box(ui, center, Vec2::new(size, 0.0), Color32::RED, GizmoAxis::X) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_scale } else { 0.0 };
                let scale_factor = 1.0 + self.apply_snap(delta.x * 0.01, snap);
                transform.scale_by([scale_factor, 1.0, 1.0]);
                modified = true;
            }
        }

        // Y axis (green)
        if self.render_axis_line_with_box(ui, center, Vec2::new(0.0, -size), Color32::GREEN, GizmoAxis::Y) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_scale } else { 0.0 };
                let scale_factor = 1.0 + self.apply_snap(-delta.y * 0.01, snap);
                transform.scale_by([1.0, scale_factor, 1.0]);
                modified = true;
            }
        }

        // Z axis (blue)
        if self.render_axis_line_with_box(ui, center, Vec2::new(size * 0.5, size * 0.5), Color32::BLUE, GizmoAxis::Z) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_scale } else { 0.0 };
                let scale_factor = 1.0 + self.apply_snap((delta.x + delta.y) * 0.01, snap);
                transform.scale_by([1.0, 1.0, scale_factor]);
                modified = true;
            }
        }

        // Center cube for uniform scale
        if self.render_center_cube(ui, center, GizmoAxis::XYZ) {
            if let Some(delta) = self.get_drag_delta(ui) {
                let snap = if self.snap_enabled { self.snap_scale } else { 0.0 };
                let scale_factor = 1.0 + self.apply_snap((delta.x + delta.y) * 0.01, snap);
                transform.scale_by([scale_factor, scale_factor, scale_factor]);
                modified = true;
            }
        }

        modified
    }

    fn render_axis_arrow(&mut self, ui: &mut Ui, start: Pos2, direction: Vec2, color: Color32, axis: GizmoAxis) -> bool {
        let end = start + direction;
        let painter = ui.painter();

        // Draw line
        let is_active = self.active_axis == Some(axis);
        let stroke_width = if is_active { 4.0 } else { 2.0 };
        painter.line_segment([start, end], Stroke::new(stroke_width, color));

        // Draw arrowhead
        let arrow_size = 10.0;
        let dir_norm = direction.normalized();
        let perp = Vec2::new(-dir_norm.y, dir_norm.x);
        let arrow_tip = end;
        let arrow_base = end - dir_norm * arrow_size;
        let arrow_left = arrow_base + perp * (arrow_size * 0.5);
        let arrow_right = arrow_base - perp * (arrow_size * 0.5);

        painter.add(egui::Shape::convex_polygon(
            vec![arrow_tip, arrow_left, arrow_right],
            color,
            Stroke::NONE,
        ));

        // Interaction area
        let rect = Rect::from_two_pos(start, end).expand(5.0);
        let response = ui.allocate_rect(rect, Sense::click_and_drag());

        self.handle_interaction(response, axis)
    }

    fn render_axis_line_with_box(&mut self, ui: &mut Ui, start: Pos2, direction: Vec2, color: Color32, axis: GizmoAxis) -> bool {
        let end = start + direction;
        let painter = ui.painter();

        // Draw line
        let is_active = self.active_axis == Some(axis);
        let stroke_width = if is_active { 4.0 } else { 2.0 };
        painter.line_segment([start, end], Stroke::new(stroke_width, color));

        // Draw box at end
        let box_size = 10.0;
        let box_rect = Rect::from_center_size(end, Vec2::splat(box_size));
        painter.rect_filled(box_rect, 0.0, color);

        // Interaction area
        let rect = Rect::from_two_pos(start, end).expand(5.0);
        let response = ui.allocate_rect(rect, Sense::click_and_drag());

        self.handle_interaction(response, axis)
    }

    fn render_rotation_circle(&mut self, ui: &mut Ui, center: Pos2, radius: f32, color: Color32, axis: GizmoAxis) -> bool {
        let painter = ui.painter();
        let is_active = self.active_axis == Some(axis);
        let stroke_width = if is_active { 4.0 } else { 2.0 };

        painter.circle_stroke(center, radius, Stroke::new(stroke_width, color));

        let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
        let response = ui.allocate_rect(rect, Sense::click_and_drag());

        self.handle_interaction(response, axis)
    }

    fn render_plane_handle(&mut self, ui: &mut Ui, center: Pos2, offset: Vec2, color: Color32, axis: GizmoAxis) -> bool {
        let painter = ui.painter();
        let size = 15.0;
        let pos = center + offset;
        let rect = Rect::from_center_size(pos, Vec2::splat(size));

        let is_active = self.active_axis == Some(axis);
        let fill_color = if is_active {
            color
        } else {
            Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 100)
        };

        painter.rect_filled(rect, 0.0, fill_color);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, color));

        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        self.handle_interaction(response, axis)
    }

    fn render_center_sphere(&mut self, ui: &mut Ui, center: Pos2) -> bool {
        let painter = ui.painter();
        let radius = 8.0;

        painter.circle_filled(center, radius, Color32::WHITE);
        painter.circle_stroke(center, radius, Stroke::new(1.0, Color32::GRAY));

        let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
        let response = ui.allocate_rect(rect, Sense::click_and_drag());

        self.handle_interaction(response, GizmoAxis::XYZ)
    }

    fn render_center_cube(&mut self, ui: &mut Ui, center: Pos2, axis: GizmoAxis) -> bool {
        let painter = ui.painter();
        let size = 16.0;
        let rect = Rect::from_center_size(center, Vec2::splat(size));

        let is_active = self.active_axis == Some(axis);
        let fill_color = if is_active {
            Color32::YELLOW
        } else {
            Color32::WHITE
        };

        painter.rect_filled(rect, 0.0, fill_color);
        painter.rect_stroke(rect, 0.0, Stroke::new(2.0, Color32::GRAY));

        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        self.handle_interaction(response, axis)
    }

    fn handle_interaction(&mut self, response: Response, axis: GizmoAxis) -> bool {
        if response.drag_started() {
            self.active_axis = Some(axis);
            self.drag_start_pos = response.interact_pointer_pos();
            true
        } else if response.dragged() && self.active_axis == Some(axis) {
            true
        } else if response.drag_stopped() && self.active_axis == Some(axis) {
            self.active_axis = None;
            self.drag_start_pos = None;
            false
        } else {
            false
        }
    }

    fn get_drag_delta(&self, ui: &Ui) -> Option<Vec2> {
        if let Some(start_pos) = self.drag_start_pos {
            if let Some(current_pos) = ui.input(|i| i.pointer.interact_pos()) {
                return Some(current_pos - start_pos);
            }
        }
        None
    }

    fn apply_snap(&self, value: f32, snap: f32) -> f32 {
        if snap > 0.0 {
            (value / snap).round() * snap
        } else {
            value
        }
    }

    /// Render gizmo controls UI
    pub fn render_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Gizmo Mode:");
            
            if ui.selectable_label(self.mode == GizmoMode::Translate, "🔀 Translate").clicked() {
                self.set_mode(GizmoMode::Translate);
            }
            if ui.selectable_label(self.mode == GizmoMode::Rotate, "🔄 Rotate").clicked() {
                self.set_mode(GizmoMode::Rotate);
            }
            if ui.selectable_label(self.mode == GizmoMode::Scale, "📏 Scale").clicked() {
                self.set_mode(GizmoMode::Scale);
            }

            ui.separator();

            ui.checkbox(&mut self.snap_enabled, "Snap");

            if self.snap_enabled {
                ui.label("|");
                match self.mode {
                    GizmoMode::Translate => {
                        ui.add(egui::DragValue::new(&mut self.snap_translate).speed(0.1).prefix("Grid: "));
                    }
                    GizmoMode::Rotate => {
                        ui.add(egui::DragValue::new(&mut self.snap_rotate).speed(1.0).prefix("Angle: ").suffix("°"));
                    }
                    GizmoMode::Scale => {
                        ui.add(egui::DragValue::new(&mut self.snap_scale).speed(0.01).prefix("Step: "));
                    }
                }
            }
        });
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for GizmoSystem {
    fn default() -> Self {
        Self::new()
    }
}

