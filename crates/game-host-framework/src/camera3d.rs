//! 3D Camera System
//!
//! Provides comprehensive camera controllers for 3D games.

use crate::math::{Mat4, Quat, Vec3};

/// 3D Camera
#[derive(Debug, Clone)]
pub struct Camera3D {
    /// Camera position
    pub position: Vec3,
    
    /// Camera rotation (quaternion)
    pub rotation: Quat,
    
    /// Field of view (degrees)
    pub fov: f32,
    
    /// Aspect ratio (width / height)
    pub aspect_ratio: f32,
    
    /// Near clipping plane
    pub near: f32,
    
    /// Far clipping plane
    pub far: f32,
    
    /// Camera shake offset
    shake_offset: Vec3,
    
    /// Camera shake rotation
    shake_rotation: Quat,
}

impl Camera3D {
    /// Create a new 3D camera
    pub fn new(position: Vec3, fov: f32, aspect_ratio: f32) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            fov,
            aspect_ratio,
            near: 0.1,
            far: 1000.0,
            shake_offset: Vec3::ZERO,
            shake_rotation: Quat::IDENTITY,
        }
    }
    
    /// Get forward direction
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::new(0.0, 0.0, -1.0)
    }
    
    /// Get right direction
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::new(1.0, 0.0, 0.0)
    }
    
    /// Get up direction
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::new(0.0, 1.0, 0.0)
    }
    
    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4 {
        let final_position = self.position + self.shake_offset;
        let final_rotation = self.rotation * self.shake_rotation;
        
        Mat4::from_rotation_translation(final_rotation, final_position).inverse()
    }
    
    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }
    
    /// Get view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
    
    /// Look at a target
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let up_corrected = right.cross(forward).normalize();
        
        // Create rotation matrix from basis vectors
        let rotation_matrix = Mat4::from_cols(
            right.extend(0.0),
            up_corrected.extend(0.0),
            (-forward).extend(0.0),
            Vec3::ZERO.extend(1.0),
        );
        
        self.rotation = Quat::from_mat4(&rotation_matrix);
    }
    
    /// Set shake offset (for camera shake effects)
    pub fn set_shake(&mut self, offset: Vec3, rotation: Quat) {
        self.shake_offset = offset;
        self.shake_rotation = rotation;
    }
    
    /// Clear camera shake
    pub fn clear_shake(&mut self) {
        self.shake_offset = Vec3::ZERO;
        self.shake_rotation = Quat::IDENTITY;
    }
}

/// First-person camera controller
#[derive(Debug, Clone)]
pub struct FirstPersonCamera {
    /// Base camera
    pub camera: Camera3D,
    
    /// Mouse sensitivity
    pub sensitivity: f32,
    
    /// Movement speed
    pub move_speed: f32,
    
    /// Sprint multiplier
    pub sprint_multiplier: f32,
    
    /// Pitch angle (radians)
    pub pitch: f32,
    
    /// Yaw angle (radians)
    pub yaw: f32,
    
    /// Maximum pitch angle (radians)
    pub max_pitch: f32,
}

impl FirstPersonCamera {
    /// Create a new first-person camera
    pub fn new(position: Vec3, fov: f32, aspect_ratio: f32) -> Self {
        Self {
            camera: Camera3D::new(position, fov, aspect_ratio),
            sensitivity: 0.002,
            move_speed: 5.0,
            sprint_multiplier: 2.0,
            pitch: 0.0,
            yaw: 0.0,
            max_pitch: std::f32::consts::FRAC_PI_2 - 0.01,
        }
    }
    
    /// Process mouse movement
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * self.sensitivity;
        self.pitch -= delta_y * self.sensitivity;
        
        // Clamp pitch
        self.pitch = self.pitch.clamp(-self.max_pitch, self.max_pitch);
        
        // Update camera rotation
        let pitch_quat = Quat::from_axis_angle(Vec3::X, self.pitch);
        let yaw_quat = Quat::from_axis_angle(Vec3::Y, self.yaw);
        self.camera.rotation = yaw_quat * pitch_quat;
    }
    
    /// Process movement input
    pub fn process_movement(&mut self, forward: f32, right: f32, up: f32, delta: f32, sprinting: bool) {
        let speed = if sprinting {
            self.move_speed * self.sprint_multiplier
        } else {
            self.move_speed
        };
        
        let forward_dir = self.camera.forward();
        let right_dir = self.camera.right();
        let up_dir = Vec3::Y; // World up
        
        self.camera.position += forward_dir * forward * speed * delta;
        self.camera.position += right_dir * right * speed * delta;
        self.camera.position += up_dir * up * speed * delta;
    }
}

/// Third-person camera controller
#[derive(Debug, Clone)]
pub struct ThirdPersonCamera {
    /// Base camera
    pub camera: Camera3D,
    
    /// Target to follow
    pub target: Vec3,
    
    /// Distance from target
    pub distance: f32,
    
    /// Minimum distance
    pub min_distance: f32,
    
    /// Maximum distance
    pub max_distance: f32,
    
    /// Pitch angle (radians)
    pub pitch: f32,
    
    /// Yaw angle (radians)
    pub yaw: f32,
    
    /// Mouse sensitivity
    pub sensitivity: f32,
    
    /// Smooth follow speed
    pub follow_speed: f32,
    
    /// Collision detection
    pub collision_enabled: bool,
    
    /// Offset from target
    pub offset: Vec3,
}

impl ThirdPersonCamera {
    /// Create a new third-person camera
    pub fn new(target: Vec3, distance: f32, fov: f32, aspect_ratio: f32) -> Self {
        let mut camera = Self {
            camera: Camera3D::new(Vec3::ZERO, fov, aspect_ratio),
            target,
            distance,
            min_distance: 2.0,
            max_distance: 20.0,
            pitch: 0.0,
            yaw: 0.0,
            sensitivity: 0.002,
            follow_speed: 10.0,
            collision_enabled: true,
            offset: Vec3::new(0.0, 1.5, 0.0), // Shoulder height
        };
        camera.update_position();
        camera
    }
    
    /// Process mouse movement
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * self.sensitivity;
        self.pitch -= delta_y * self.sensitivity;
        
        // Clamp pitch
        self.pitch = self.pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.1, std::f32::consts::FRAC_PI_2 - 0.1);
    }
    
    /// Process zoom input
    pub fn process_zoom(&mut self, delta: f32) {
        self.distance -= delta;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }
    
    /// Update camera position
    pub fn update_position(&mut self) {
        // Calculate desired position
        let offset_target = self.target + self.offset;
        
        let horizontal_distance = self.distance * self.pitch.cos();
        let vertical_distance = self.distance * self.pitch.sin();
        
        let x_offset = horizontal_distance * self.yaw.sin();
        let z_offset = horizontal_distance * self.yaw.cos();
        
        let desired_position = Vec3::new(
            offset_target.x - x_offset,
            offset_target.y + vertical_distance,
            offset_target.z - z_offset,
        );
        
        self.camera.position = desired_position;
        self.camera.look_at(offset_target, Vec3::Y);
    }
    
    /// Update with smooth follow
    pub fn update(&mut self, delta: f32) {
        self.update_position();
    }
    
    /// Set target position
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }
}

/// Camera shake effect
#[derive(Debug, Clone)]
pub struct CameraShake {
    /// Shake intensity
    pub intensity: f32,
    
    /// Shake frequency
    pub frequency: f32,
    
    /// Shake duration
    pub duration: f32,
    
    /// Current time
    current_time: f32,
    
    /// Random seed
    seed: f32,
}

impl CameraShake {
    /// Create a new camera shake
    pub fn new(intensity: f32, frequency: f32, duration: f32) -> Self {
        Self {
            intensity,
            frequency,
            duration,
            current_time: 0.0,
            seed: 0.0,
        }
    }
    
    /// Update shake
    pub fn update(&mut self, delta: f32) -> Option<(Vec3, Quat)> {
        if self.current_time >= self.duration {
            return None;
        }
        
        self.current_time += delta;
        
        // Calculate shake amount with falloff
        let progress = self.current_time / self.duration;
        let falloff = 1.0 - progress;
        
        // Generate shake offset using sine waves
        let time = self.current_time * self.frequency;
        let offset = Vec3::new(
            (time + self.seed).sin() * self.intensity * falloff,
            (time * 1.3 + self.seed + 1.0).sin() * self.intensity * falloff,
            (time * 0.7 + self.seed + 2.0).sin() * self.intensity * falloff * 0.5,
        );
        
        // Generate shake rotation
        let rotation_amount = self.intensity * 0.1 * falloff;
        let rotation = Quat::from_euler(
            glam::EulerRot::XYZ,
            (time * 2.0).sin() * rotation_amount,
            (time * 1.5).sin() * rotation_amount,
            (time * 2.5).sin() * rotation_amount,
        );
        
        Some((offset, rotation))
    }
    
    /// Check if shake is complete
    pub fn is_complete(&self) -> bool {
        self.current_time >= self.duration
    }
    
    /// Reset shake
    pub fn reset(&mut self) {
        self.current_time = 0.0;
    }
}

/// Smooth camera follow
#[derive(Debug, Clone)]
pub struct SmoothFollow {
    /// Follow speed
    pub speed: f32,
    
    /// Rotation speed
    pub rotation_speed: f32,
    
    /// Position offset
    pub offset: Vec3,
    
    /// Look-at offset
    pub look_offset: Vec3,
}

impl SmoothFollow {
    /// Create a new smooth follow
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            rotation_speed: 5.0,
            offset: Vec3::new(0.0, 2.0, -5.0),
            look_offset: Vec3::new(0.0, 1.0, 0.0),
        }
    }
    
    /// Update camera to follow target
    pub fn update(&self, camera: &mut Camera3D, target_position: Vec3, target_rotation: Quat, delta: f32) {
        // Calculate desired position
        let desired_position = target_position + target_rotation * self.offset;
        
        // Smoothly move towards desired position
        let t = 1.0 - (-self.speed * delta).exp();
        camera.position = camera.position.lerp(desired_position, t);
        
        // Look at target
        let look_target = target_position + self.look_offset;
        camera.look_at(look_target, Vec3::Y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_camera_creation() {
        let camera = Camera3D::new(Vec3::ZERO, 60.0, 16.0 / 9.0);
        assert_eq!(camera.position, Vec3::ZERO);
        assert_eq!(camera.fov, 60.0);
    }
    
    #[test]
    fn test_camera_directions() {
        let camera = Camera3D::new(Vec3::ZERO, 60.0, 16.0 / 9.0);
        let forward = camera.forward();
        let right = camera.right();
        let up = camera.up();
        
        // Check orthogonality
        assert!((forward.dot(right)).abs() < 0.01);
        assert!((forward.dot(up)).abs() < 0.01);
        assert!((right.dot(up)).abs() < 0.01);
    }
    
    #[test]
    fn test_first_person_camera() {
        let mut camera = FirstPersonCamera::new(Vec3::ZERO, 60.0, 16.0 / 9.0);
        
        // Process mouse movement
        camera.process_mouse(0.1, 0.1);
        assert_ne!(camera.yaw, 0.0);
        assert_ne!(camera.pitch, 0.0);
        
        // Process movement
        camera.process_movement(1.0, 0.0, 0.0, 0.016, false);
        assert_ne!(camera.camera.position, Vec3::ZERO);
    }
    
    #[test]
    fn test_third_person_camera() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 5.0, 60.0, 16.0 / 9.0);
        
        // Process mouse
        camera.process_mouse(0.1, 0.1);
        assert_ne!(camera.yaw, 0.0);
        
        // Process zoom
        let initial_distance = camera.distance;
        camera.process_zoom(1.0);
        assert_ne!(camera.distance, initial_distance);
    }
    
    #[test]
    fn test_camera_shake() {
        let mut shake = CameraShake::new(0.5, 10.0, 1.0);
        
        // Update shake
        let result = shake.update(0.016);
        assert!(result.is_some());
        
        // Shake should complete after duration
        for _ in 0..100 {
            shake.update(0.016);
        }
        assert!(shake.is_complete());
    }
    
    #[test]
    fn test_smooth_follow() {
        let follow = SmoothFollow::new(5.0);
        let mut camera = Camera3D::new(Vec3::ZERO, 60.0, 16.0 / 9.0);
        
        follow.update(&mut camera, Vec3::new(10.0, 0.0, 0.0), Quat::IDENTITY, 0.016);
        
        // Camera should have moved towards target
        assert!(camera.position.x > 0.0);
    }
}
