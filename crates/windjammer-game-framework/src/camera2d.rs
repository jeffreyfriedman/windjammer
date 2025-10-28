//! 2D Camera system for view transformation and viewport management

use crate::math::Vec2;
use crate::transform::Transform2D;

/// 2D Camera for orthographic projection
#[derive(Debug, Clone)]
pub struct Camera2D {
    /// Camera position in world space
    pub position: Vec2,
    /// Camera rotation (in radians)
    pub rotation: f32,
    /// Camera zoom (1.0 = normal, 2.0 = 2x zoom in, 0.5 = 2x zoom out)
    pub zoom: f32,
    /// Viewport size (in pixels)
    pub viewport_size: Vec2,
}

impl Camera2D {
    /// Create a new camera at the origin
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            zoom: 1.0,
            viewport_size: Vec2::new(viewport_width, viewport_height),
        }
    }

    /// Create a camera at a specific position
    pub fn at(position: Vec2, viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            position,
            rotation: 0.0,
            zoom: 1.0,
            viewport_size: Vec2::new(viewport_width, viewport_height),
        }
    }

    /// Set the camera position
    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// Set the camera zoom
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Set the camera rotation (in radians)
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the camera rotation (in degrees)
    pub fn with_rotation_degrees(mut self, degrees: f32) -> Self {
        self.rotation = degrees.to_radians();
        self
    }

    /// Move the camera by a delta
    pub fn translate(&mut self, delta: Vec2) {
        self.position += delta;
    }

    /// Rotate the camera by an angle (in radians)
    pub fn rotate(&mut self, delta: f32) {
        self.rotation += delta;
    }

    /// Rotate the camera by an angle (in degrees)
    pub fn rotate_degrees(&mut self, degrees: f32) {
        self.rotation += degrees.to_radians();
    }

    /// Zoom in or out by a factor
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom *= factor;
        self.zoom = self.zoom.max(0.01); // Prevent negative or zero zoom
    }

    /// Set zoom to a specific value
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.max(0.01);
    }

    /// Get the view bounds (min and max world coordinates visible)
    pub fn view_bounds(&self) -> (Vec2, Vec2) {
        let half_width = (self.viewport_size.x / 2.0) / self.zoom;
        let half_height = (self.viewport_size.y / 2.0) / self.zoom;

        let min = Vec2::new(self.position.x - half_width, self.position.y - half_height);
        let max = Vec2::new(self.position.x + half_width, self.position.y + half_height);

        (min, max)
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // Center the screen position
        let centered = Vec2::new(
            screen_pos.x - self.viewport_size.x / 2.0,
            screen_pos.y - self.viewport_size.y / 2.0,
        );

        // Apply inverse zoom
        let scaled = Vec2::new(centered.x / self.zoom, centered.y / self.zoom);

        // Apply inverse rotation
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let rotated = Vec2::new(
            scaled.x * cos + scaled.y * sin,
            -scaled.x * sin + scaled.y * cos,
        );

        // Apply camera position
        rotated + self.position
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        // Translate relative to camera
        let relative = world_pos - self.position;

        // Apply rotation
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let rotated = Vec2::new(
            relative.x * cos - relative.y * sin,
            relative.x * sin + relative.y * cos,
        );

        // Apply zoom
        let scaled = Vec2::new(rotated.x * self.zoom, rotated.y * self.zoom);

        // Center on screen
        Vec2::new(
            scaled.x + self.viewport_size.x / 2.0,
            scaled.y + self.viewport_size.y / 2.0,
        )
    }

    /// Check if a point is visible in the camera view
    pub fn is_visible(&self, world_pos: Vec2, margin: f32) -> bool {
        let (min, max) = self.view_bounds();
        world_pos.x >= min.x - margin
            && world_pos.x <= max.x + margin
            && world_pos.y >= min.y - margin
            && world_pos.y <= max.y + margin
    }

    /// Check if a rectangle is visible in the camera view
    pub fn is_rect_visible(&self, center: Vec2, size: Vec2) -> bool {
        let (view_min, view_max) = self.view_bounds();
        let half_size = Vec2::new(size.x / 2.0, size.y / 2.0);
        let rect_min = center - half_size;
        let rect_max = center + half_size;

        // AABB intersection test
        rect_max.x >= view_min.x
            && rect_min.x <= view_max.x
            && rect_max.y >= view_min.y
            && rect_min.y <= view_max.y
    }

    /// Follow a target position smoothly
    pub fn follow(&mut self, target: Vec2, smoothness: f32, delta: f32) {
        let diff = target - self.position;
        self.position += diff * (smoothness * delta).min(1.0);
    }

    /// Follow a target with a dead zone
    pub fn follow_with_deadzone(
        &mut self,
        target: Vec2,
        deadzone_size: Vec2,
        smoothness: f32,
        delta: f32,
    ) {
        let diff = target - self.position;
        let abs_diff = Vec2::new(diff.x.abs(), diff.y.abs());

        let half_deadzone = Vec2::new(deadzone_size.x / 2.0, deadzone_size.y / 2.0);

        let mut move_vec = Vec2::ZERO;

        if abs_diff.x > half_deadzone.x {
            move_vec.x = diff.x - half_deadzone.x * diff.x.signum();
        }

        if abs_diff.y > half_deadzone.y {
            move_vec.y = diff.y - half_deadzone.y * diff.y.signum();
        }

        self.position += move_vec * (smoothness * delta).min(1.0);
    }

    /// Clamp camera position to bounds
    pub fn clamp_to_bounds(&mut self, min: Vec2, max: Vec2) {
        self.position.x = self.position.x.clamp(min.x, max.x);
        self.position.y = self.position.y.clamp(min.y, max.y);
    }

    /// Shake the camera (for effects like explosions)
    pub fn shake(&self, intensity: f32, time: f32) -> Vec2 {
        // Simple shake using time-based pseudo-random
        let x = (time * 50.0).sin() * intensity;
        let y = (time * 47.0).cos() * intensity;
        Vec2::new(x, y)
    }

    /// Get the camera's view matrix (3x3 for 2D)
    pub fn view_matrix(&self) -> [[f32; 3]; 3] {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();

        // Scale
        let sx = self.zoom;
        let sy = self.zoom;

        // Translation
        let tx = -self.position.x * sx;
        let ty = -self.position.y * sy;

        [
            [cos * sx, -sin * sx, tx],
            [sin * sy, cos * sy, ty],
            [0.0, 0.0, 1.0],
        ]
    }

    /// Create a camera from a Transform2D
    pub fn from_transform(
        transform: Transform2D,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self {
        Self {
            position: transform.position,
            rotation: transform.rotation,
            zoom: (transform.scale.x + transform.scale.y) / 2.0, // Average scale as zoom
            viewport_size: Vec2::new(viewport_width, viewport_height),
        }
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_creation() {
        let camera = Camera2D::new(800.0, 600.0);
        assert_eq!(camera.position, Vec2::ZERO);
        assert_eq!(camera.zoom, 1.0);
        assert_eq!(camera.viewport_size, Vec2::new(800.0, 600.0));
    }

    #[test]
    fn test_camera_builder() {
        let camera = Camera2D::new(800.0, 600.0)
            .with_position(Vec2::new(100.0, 100.0))
            .with_zoom(2.0)
            .with_rotation_degrees(45.0);

        assert_eq!(camera.position, Vec2::new(100.0, 100.0));
        assert_eq!(camera.zoom, 2.0);
        assert!((camera.rotation - std::f32::consts::PI / 4.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_translate() {
        let mut camera = Camera2D::new(800.0, 600.0);
        camera.translate(Vec2::new(10.0, 20.0));
        assert_eq!(camera.position, Vec2::new(10.0, 20.0));
    }

    #[test]
    fn test_camera_zoom() {
        let mut camera = Camera2D::new(800.0, 600.0);
        camera.zoom_by(2.0);
        assert_eq!(camera.zoom, 2.0);

        camera.zoom_by(0.5);
        assert_eq!(camera.zoom, 1.0);
    }

    #[test]
    fn test_camera_view_bounds() {
        let camera = Camera2D::new(800.0, 600.0);
        let (min, max) = camera.view_bounds();

        // At zoom 1.0, view bounds should be half viewport size
        assert_eq!(min, Vec2::new(-400.0, -300.0));
        assert_eq!(max, Vec2::new(400.0, 300.0));
    }

    #[test]
    fn test_camera_view_bounds_with_zoom() {
        let camera = Camera2D::new(800.0, 600.0).with_zoom(2.0);
        let (min, max) = camera.view_bounds();

        // At zoom 2.0, view bounds should be half the size
        assert_eq!(min, Vec2::new(-200.0, -150.0));
        assert_eq!(max, Vec2::new(200.0, 150.0));
    }

    #[test]
    fn test_screen_to_world() {
        let camera = Camera2D::new(800.0, 600.0);

        // Center of screen should map to camera position
        let world_pos = camera.screen_to_world(Vec2::new(400.0, 300.0));
        assert!((world_pos.x - camera.position.x).abs() < 0.001);
        assert!((world_pos.y - camera.position.y).abs() < 0.001);
    }

    #[test]
    fn test_world_to_screen() {
        let camera = Camera2D::new(800.0, 600.0);

        // Camera position should map to center of screen
        let screen_pos = camera.world_to_screen(camera.position);
        assert!((screen_pos.x - 400.0).abs() < 0.001);
        assert!((screen_pos.y - 300.0).abs() < 0.001);
    }

    #[test]
    fn test_is_visible() {
        let camera = Camera2D::new(800.0, 600.0);

        // Point at origin should be visible
        assert!(camera.is_visible(Vec2::ZERO, 0.0));

        // Point far away should not be visible
        assert!(!camera.is_visible(Vec2::new(1000.0, 1000.0), 0.0));
    }

    #[test]
    fn test_is_rect_visible() {
        let camera = Camera2D::new(800.0, 600.0);

        // Rectangle at origin should be visible
        assert!(camera.is_rect_visible(Vec2::ZERO, Vec2::new(10.0, 10.0)));

        // Rectangle far away should not be visible
        assert!(!camera.is_rect_visible(Vec2::new(2000.0, 2000.0), Vec2::new(10.0, 10.0)));
    }

    #[test]
    fn test_follow() {
        let mut camera = Camera2D::new(800.0, 600.0);
        let target = Vec2::new(100.0, 100.0);

        camera.follow(target, 5.0, 0.1);

        // Camera should move towards target
        assert!(camera.position.x > 0.0);
        assert!(camera.position.y > 0.0);
        assert!(camera.position.x < target.x);
        assert!(camera.position.y < target.y);
    }

    #[test]
    fn test_clamp_to_bounds() {
        let mut camera = Camera2D::new(800.0, 600.0);
        camera.position = Vec2::new(1000.0, 1000.0);

        camera.clamp_to_bounds(Vec2::ZERO, Vec2::new(500.0, 500.0));

        assert_eq!(camera.position, Vec2::new(500.0, 500.0));
    }
}
