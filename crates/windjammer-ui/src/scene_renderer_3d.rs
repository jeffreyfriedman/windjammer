// 3D Scene Renderer using wgpu
// Renders scene objects in the Scene View panel

#![cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]

use crate::scene_manager::{Color, ObjectType, Scene, SceneObject, Vec3};
use std::sync::{Arc, Mutex};

/// Basic 3D renderer for scene viewport
pub struct SceneRenderer3D {
    // For now, we'll use egui's built-in painting
    // In the future, this will use wgpu directly
}

impl SceneRenderer3D {
    pub fn new() -> Self {
        Self {}
    }

    /// Render the scene in the given UI area
    pub fn render(&mut self, ui: &mut egui::Ui, scene: &Arc<Mutex<Scene>>) {
        let scene = scene.lock().unwrap();
        let available_size = ui.available_size();
        let (rect, _response) =
            ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

        // Background (sky/skybox)
        let bg_color = match &scene.skybox {
            crate::scene_manager::Skybox::SolidColor(color) => color_to_egui(color),
            crate::scene_manager::Skybox::Gradient { top, bottom: _ } => {
                // For gradient, use top color (simplified)
                color_to_egui(top)
            }
            crate::scene_manager::Skybox::Cubemap { path: _ } => {
                // Fallback for cubemap
                egui::Color32::from_rgb(100, 150, 200)
            }
        };
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Draw grid
        self.draw_grid(ui, rect);

        // Simple 3D projection (orthographic for now)
        let center_x = rect.center().x;
        let center_y = rect.center().y;
        let scale = 20.0; // pixels per unit

        // Camera position (use scene camera)
        let camera_pos = &scene.camera.position;

        // Draw all objects
        for (_id, object) in &scene.objects {
            if !object.visible {
                continue;
            }

            self.draw_object(ui, rect, object, camera_pos, center_x, center_y, scale);
        }

        // Draw camera preview (picture-in-picture)
        self.draw_camera_preview(ui, rect, &scene);
    }

    fn draw_grid(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let grid_spacing = 50.0;
        let grid_color = egui::Color32::from_rgba_premultiplied(80, 80, 80, 100);

        // Vertical lines
        let mut x = rect.left();
        while x <= rect.right() {
            ui.painter().line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, grid_color),
            );
            x += grid_spacing;
        }

        // Horizontal lines
        let mut y = rect.top();
        while y <= rect.bottom() {
            ui.painter().line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, grid_color),
            );
            y += grid_spacing;
        }

        // Origin axes (X=red, Y=green, Z=blue)
        let origin_x = rect.center().x;
        let origin_y = rect.center().y;
        let axis_length = 100.0;

        // X axis (red, horizontal)
        ui.painter().line_segment(
            [
                egui::pos2(origin_x, origin_y),
                egui::pos2(origin_x + axis_length, origin_y),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 0, 0)),
        );

        // Y axis (green, vertical up)
        ui.painter().line_segment(
            [
                egui::pos2(origin_x, origin_y),
                egui::pos2(origin_x, origin_y - axis_length),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
        );

        // Z axis (blue, diagonal for depth)
        ui.painter().line_segment(
            [
                egui::pos2(origin_x, origin_y),
                egui::pos2(origin_x - axis_length * 0.5, origin_y + axis_length * 0.5),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 0, 255)),
        );
    }

    fn draw_object(
        &self,
        ui: &mut egui::Ui,
        _rect: egui::Rect,
        object: &SceneObject,
        _camera_pos: &Vec3,
        center_x: f32,
        center_y: f32,
        scale: f32,
    ) {
        // Simple orthographic projection
        let pos = &object.transform.position;
        let screen_x = center_x + pos.x * scale;
        let screen_y = center_y - pos.y * scale; // Flip Y for screen coordinates

        match &object.object_type {
            ObjectType::Cube { size } => {
                let s = size * scale * object.transform.scale.x;
                let rect =
                    egui::Rect::from_center_size(egui::pos2(screen_x, screen_y), egui::vec2(s, s));
                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::from_rgb(180, 180, 180));
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                );
            }
            ObjectType::Sphere { radius } => {
                let r = radius * scale * object.transform.scale.x;
                ui.painter().circle_filled(
                    egui::pos2(screen_x, screen_y),
                    r,
                    egui::Color32::from_rgb(180, 180, 180),
                );
                ui.painter().circle_stroke(
                    egui::pos2(screen_x, screen_y),
                    r,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                );
            }
            ObjectType::Plane { width, height } => {
                let w = width * scale * object.transform.scale.x;
                let h = height * scale * object.transform.scale.z;
                let rect =
                    egui::Rect::from_center_size(egui::pos2(screen_x, screen_y), egui::vec2(w, h));
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_rgb(150, 150, 150));
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                );
            }
            ObjectType::DirectionalLight { color, .. } => {
                // Draw sun icon
                let r = 15.0;
                ui.painter()
                    .circle_filled(egui::pos2(screen_x, screen_y), r, color_to_egui(color));
                // Draw rays
                for i in 0..8 {
                    let angle = (i as f32) * std::f32::consts::PI / 4.0;
                    let start_x = screen_x + angle.cos() * r;
                    let start_y = screen_y + angle.sin() * r;
                    let end_x = screen_x + angle.cos() * (r + 10.0);
                    let end_y = screen_y + angle.sin() * (r + 10.0);
                    ui.painter().line_segment(
                        [egui::pos2(start_x, start_y), egui::pos2(end_x, end_y)],
                        egui::Stroke::new(2.0, color_to_egui(color)),
                    );
                }
            }
            ObjectType::PointLight { color, .. } => {
                // Draw bulb icon
                let r = 10.0;
                ui.painter()
                    .circle_filled(egui::pos2(screen_x, screen_y), r, color_to_egui(color));
                // Draw glow
                ui.painter().circle_stroke(
                    egui::pos2(screen_x, screen_y),
                    r + 5.0,
                    egui::Stroke::new(2.0, color_to_egui_alpha(color, 0.5)),
                );
            }
            ObjectType::Camera => {
                // Draw camera icon
                let size = 20.0;
                let rect = egui::Rect::from_center_size(
                    egui::pos2(screen_x, screen_y),
                    egui::vec2(size, size * 0.7),
                );
                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::from_rgb(100, 150, 255));
                // Lens
                ui.painter().circle_filled(
                    egui::pos2(screen_x, screen_y),
                    size * 0.3,
                    egui::Color32::from_rgb(50, 50, 50),
                );
            }
            _ => {
                // Default: draw a small marker
                ui.painter().circle_filled(
                    egui::pos2(screen_x, screen_y),
                    5.0,
                    egui::Color32::from_rgb(200, 200, 200),
                );
            }
        }

        // Draw object name
        ui.painter().text(
            egui::pos2(screen_x, screen_y - 25.0),
            egui::Align2::CENTER_BOTTOM,
            &object.name,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 255, 255),
        );
    }

    fn draw_camera_preview(&self, ui: &mut egui::Ui, rect: egui::Rect, scene: &Scene) {
        // Picture-in-picture camera preview (Godot-inspired)
        let preview_width = 200.0;
        let preview_height = 150.0;
        let margin = 10.0;

        let preview_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.right() - preview_width - margin,
                rect.bottom() - preview_height - margin,
            ),
            egui::vec2(preview_width, preview_height),
        );

        // Semi-transparent background
        ui.painter().rect_filled(
            preview_rect,
            4.0,
            egui::Color32::from_rgba_premultiplied(30, 30, 30, 230),
        );

        // Border
        ui.painter().rect_stroke(
            preview_rect,
            4.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
        );

        // Checkerboard pattern (to show it's a preview)
        let checker_size = 20.0;
        for y in 0..((preview_height / checker_size) as i32) {
            for x in 0..((preview_width / checker_size) as i32) {
                if (x + y) % 2 == 0 {
                    let checker_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            preview_rect.left() + x as f32 * checker_size,
                            preview_rect.top() + y as f32 * checker_size,
                        ),
                        egui::vec2(checker_size, checker_size),
                    );
                    ui.painter().rect_filled(
                        checker_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(50, 50, 50, 100),
                    );
                }
            }
        }

        // Camera icon
        ui.painter().text(
            egui::pos2(preview_rect.left() + 10.0, preview_rect.top() + 10.0),
            egui::Align2::LEFT_TOP,
            "📷 Camera Preview",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(200, 200, 200),
        );

        // Camera info
        let info_y = preview_rect.top() + 30.0;
        ui.painter().text(
            egui::pos2(preview_rect.left() + 10.0, info_y),
            egui::Align2::LEFT_TOP,
            format!("FOV: {:.0}°", scene.camera.fov),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(180, 180, 180),
        );
        ui.painter().text(
            egui::pos2(preview_rect.left() + 10.0, info_y + 15.0),
            egui::Align2::LEFT_TOP,
            format!(
                "Pos: ({:.1}, {:.1}, {:.1})",
                scene.camera.position.x, scene.camera.position.y, scene.camera.position.z
            ),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(180, 180, 180),
        );
    }
}

fn color_to_egui(color: &Color) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    )
}

fn color_to_egui_alpha(color: &Color, alpha: f32) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (alpha * 255.0) as u8,
    )
}
