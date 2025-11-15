//! 3D Camera System
//!
//! Provides various camera types for 3D games:
//! - Perspective camera (standard 3D)
//! - Orthographic camera (isometric, strategy)
//! - Third-person camera (follow & orbit)
//! - First-person camera (FPS)
//! - Free camera (editor, debug)

use crate::math::{Mat4, Quat, Vec3};
use crate::input::{Input, Key, MouseButton};

/// Camera projection type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraProjection {
    /// Perspective projection (standard 3D)
    Perspective {
        fov: f32,      // Field of view in radians
        aspect: f32,   // Aspect ratio (width / height)
        near: f32,     // Near clipping plane
        far: f32,      // Far clipping plane
    },
    /// Orthographic projection (isometric, strategy)
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
}

/// Base 3D camera
#[derive(Debug, Clone)]
pub struct Camera3D {
    /// Camera position in world space
    pub position: Vec3,
    /// Camera rotation (quaternion)
    pub rotation: Quat,
    /// Projection type
    pub projection: CameraProjection,
}

impl Camera3D {
    /// Create a new perspective camera
    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            projection: CameraProjection::Perspective {
                fov,
                aspect,
                near,
                far,
            },
        }
    }

    /// Create a new orthographic camera
    pub fn orthographic(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            projection: CameraProjection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            },
        }
    }

    /// Get the view matrix (transforms world space to camera space)
    pub fn view_matrix(&self) -> Mat4 {
        let rotation_matrix = Mat4::from_quat(self.rotation);
        let translation_matrix = Mat4::from_translation(-self.position);
        rotation_matrix * translation_matrix
    }

    /// Get the projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        match self.projection {
            CameraProjection::Perspective {
                fov,
                aspect,
                near,
                far,
            } => Mat4::perspective_rh(fov, aspect, near, far),
            CameraProjection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => Mat4::orthographic_rh(left, right, bottom, top, near, far),
        }
    }

    /// Get the combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// Get the right direction vector
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Get the up direction vector
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Look at a target position
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let up = right.cross(forward).normalize();

        // Create rotation matrix from basis vectors
        let rotation_matrix = Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            (-forward).extend(0.0),
            Vec3::ZERO.extend(1.0),
        );

        self.rotation = Quat::from_mat4(&rotation_matrix);
    }
}

impl Default for Camera3D {
    fn default() -> Self {
        Self::perspective(
            std::f32::consts::PI / 4.0, // 45 degree FOV
            16.0 / 9.0,                 // 16:9 aspect ratio
            0.1,                        // Near plane
            1000.0,                     // Far plane
        )
    }
}

/// Third-person camera controller
///
/// Follows a target with orbiting and zooming capabilities.
/// Perfect for action-adventure games, RPGs, and character-focused games.
#[derive(Debug, Clone)]
pub struct ThirdPersonCamera {
    /// The camera itself
    pub camera: Camera3D,
    /// Target position to follow
    pub target: Vec3,
    /// Distance from target
    pub distance: f32,
    /// Minimum distance
    pub min_distance: f32,
    /// Maximum distance
    pub max_distance: f32,
    /// Horizontal angle (yaw) in radians
    pub yaw: f32,
    /// Vertical angle (pitch) in radians
    pub pitch: f32,
    /// Minimum pitch (looking down)
    pub min_pitch: f32,
    /// Maximum pitch (looking up)
    pub max_pitch: f32,
    /// Mouse sensitivity for orbiting
    pub mouse_sensitivity: f32,
    /// Zoom speed
    pub zoom_speed: f32,
    /// Smoothing factor (0 = instant, 1 = very smooth)
    pub smoothing: f32,
    /// Offset from target (for over-the-shoulder cameras)
    pub target_offset: Vec3,
}

impl ThirdPersonCamera {
    /// Create a new third-person camera
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            camera: Camera3D::default(),
            target,
            distance,
            min_distance: 2.0,
            max_distance: 20.0,
            yaw: 0.0,
            pitch: 0.3, // Slightly looking down
            min_pitch: -std::f32::consts::PI / 2.0 + 0.1,
            max_pitch: std::f32::consts::PI / 2.0 - 0.1,
            mouse_sensitivity: 0.003,
            zoom_speed: 2.0,
            smoothing: 0.1,
            target_offset: Vec3::ZERO,
        }
    }

    /// Update camera based on input
    pub fn update(&mut self, input: &Input, delta: f32) {
        // Orbit with mouse (right button held)
        if input.mouse_held(MouseButton::Right) {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx as f32 * self.mouse_sensitivity;
            self.pitch -= dy as f32 * self.mouse_sensitivity;
            self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);
        }

        // Zoom with scroll or keys
        if input.held(Key::Num9) {
            self.distance -= self.zoom_speed * delta;
        }
        if input.held(Key::Num0) {
            self.distance += self.zoom_speed * delta;
        }
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);

        // Calculate desired camera position
        let offset = Vec3::new(
            self.distance * self.yaw.cos() * self.pitch.cos(),
            self.distance * self.pitch.sin(),
            self.distance * self.yaw.sin() * self.pitch.cos(),
        );

        let target_pos = self.target + self.target_offset;
        let desired_position = target_pos + offset;

        // Smooth camera movement
        self.camera.position = self.camera.position.lerp(desired_position, self.smoothing);

        // Look at target
        self.camera.look_at(target_pos, Vec3::Y);
    }

    /// Set the target position
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    /// Set the camera distance
    pub fn set_distance(&mut self, distance: f32) {
        self.distance = distance.clamp(self.min_distance, self.max_distance);
    }

    /// Set the yaw angle (horizontal rotation)
    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
    }

    /// Set the pitch angle (vertical rotation)
    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(self.min_pitch, self.max_pitch);
    }

    /// Get the view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.camera.view_projection_matrix()
    }
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 10.0)
    }
}

/// First-person camera controller
///
/// Free-look camera for FPS games.
#[derive(Debug, Clone)]
pub struct FirstPersonCamera {
    /// The camera itself
    pub camera: Camera3D,
    /// Yaw angle (horizontal rotation) in radians
    pub yaw: f32,
    /// Pitch angle (vertical rotation) in radians
    pub pitch: f32,
    /// Minimum pitch (looking down)
    pub min_pitch: f32,
    /// Maximum pitch (looking up)
    pub max_pitch: f32,
    /// Mouse sensitivity
    pub mouse_sensitivity: f32,
    /// Movement speed
    pub move_speed: f32,
}

impl FirstPersonCamera {
    /// Create a new first-person camera
    pub fn new(position: Vec3) -> Self {
        let mut camera = Self {
            camera: Camera3D::default(),
            yaw: 0.0,
            pitch: 0.0,
            min_pitch: -std::f32::consts::PI / 2.0 + 0.01,
            max_pitch: std::f32::consts::PI / 2.0 - 0.01,
            mouse_sensitivity: 0.002,
            move_speed: 5.0,
        };
        camera.camera.position = position;
        camera
    }

    /// Update camera based on input
    pub fn update(&mut self, input: &Input, delta: f32) {
        // Mouse look
        let (dx, dy) = input.mouse_delta();
        self.yaw -= dx as f32 * self.mouse_sensitivity;
        self.pitch -= dy as f32 * self.mouse_sensitivity;
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);

        // Update camera rotation
        self.camera.rotation = Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);

        // WASD movement
        let mut movement = Vec3::ZERO;
        if input.held(Key::W) {
            movement += self.camera.forward();
        }
        if input.held(Key::S) {
            movement -= self.camera.forward();
        }
        if input.held(Key::A) {
            movement -= self.camera.right();
        }
        if input.held(Key::D) {
            movement += self.camera.right();
        }

        // Normalize and apply speed
        if movement.length() > 0.0 {
            movement = movement.normalize() * self.move_speed * delta;
            self.camera.position += movement;
        }

        // Up/down movement
        if input.held(Key::Space) {
            self.camera.position.y += self.move_speed * delta;
        }
        if input.held(Key::Shift) {
            self.camera.position.y -= self.move_speed * delta;
        }
    }

    /// Get the view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.camera.view_projection_matrix()
    }
}

impl Default for FirstPersonCamera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 1.8, 0.0)) // Eye height
    }
}

/// Free camera controller
///
/// Unrestricted camera movement for editor/debug purposes.
#[derive(Debug, Clone)]
pub struct FreeCamera {
    /// The camera itself
    pub camera: Camera3D,
    /// Yaw angle in radians
    pub yaw: f32,
    /// Pitch angle in radians
    pub pitch: f32,
    /// Mouse sensitivity
    pub mouse_sensitivity: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Fast movement multiplier (when holding Shift)
    pub fast_multiplier: f32,
}

impl FreeCamera {
    /// Create a new free camera
    pub fn new(position: Vec3) -> Self {
        let mut camera = Self {
            camera: Camera3D::default(),
            yaw: 0.0,
            pitch: 0.0,
            mouse_sensitivity: 0.002,
            move_speed: 10.0,
            fast_multiplier: 3.0,
        };
        camera.camera.position = position;
        camera
    }

    /// Update camera based on input
    pub fn update(&mut self, input: &Input, delta: f32) {
        // Mouse look (only when right mouse button held)
        if input.mouse_held(MouseButton::Right) {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx as f32 * self.mouse_sensitivity;
            self.pitch -= dy as f32 * self.mouse_sensitivity;
            self.pitch = self.pitch.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);
        }

        // Update camera rotation
        self.camera.rotation = Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);

        // Calculate movement speed (faster with Shift)
        let speed = if input.held(Key::Shift) {
            self.move_speed * self.fast_multiplier
        } else {
            self.move_speed
        };

        // WASD movement
        let mut movement = Vec3::ZERO;
        if input.held(Key::W) {
            movement += self.camera.forward();
        }
        if input.held(Key::S) {
            movement -= self.camera.forward();
        }
        if input.held(Key::A) {
            movement -= self.camera.right();
        }
        if input.held(Key::D) {
            movement += self.camera.right();
        }

        // Up/down movement
        if input.held(Key::Space) {
            movement += Vec3::Y;
        }
        if input.held(Key::Control) {
            movement -= Vec3::Y;
        }

        // Normalize and apply speed
        if movement.length() > 0.0 {
            movement = movement.normalize() * speed * delta;
            self.camera.position += movement;
        }
    }

    /// Get the view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.camera.view_projection_matrix()
    }
}

impl Default for FreeCamera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 5.0, 10.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera3d_creation() {
        let camera = Camera3D::default();
        assert_eq!(camera.position, Vec3::ZERO);
        assert_eq!(camera.rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_camera3d_directions() {
        let camera = Camera3D::default();
        let forward = camera.forward();
        let right = camera.right();
        let up = camera.up();

        // Default camera looks down -Z
        assert!((forward.z - -1.0).abs() < 0.001);
        assert!((right.x - 1.0).abs() < 0.001);
        assert!((up.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_third_person_camera_creation() {
        let camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.distance, 10.0);
    }

    #[test]
    fn test_first_person_camera_creation() {
        let camera = FirstPersonCamera::new(Vec3::new(0.0, 1.8, 0.0));
        assert_eq!(camera.camera.position.y, 1.8);
    }

    #[test]
    fn test_free_camera_creation() {
        let camera = FreeCamera::new(Vec3::new(0.0, 5.0, 10.0));
        assert_eq!(camera.camera.position, Vec3::new(0.0, 5.0, 10.0));
    }
}

