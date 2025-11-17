//! Character Controller for 3D Games
//!
//! Provides a high-level character controller for third-person and first-person games.
//! Handles movement, jumping, crouching, and collision with the physics world.

use crate::ecs::Entity;
use crate::math::Vec3;
use crate::physics3d::PhysicsWorld3D;
// TODO: Add RigidBody3D, Collider3D, ColliderShape3D when implementing character controller

/// Character controller component
#[derive(Debug, Clone)]
pub struct CharacterController {
    /// Movement speed (m/s)
    pub move_speed: f32,
    /// Sprint speed multiplier
    pub sprint_multiplier: f32,
    /// Crouch speed multiplier
    pub crouch_multiplier: f32,
    /// Jump force
    pub jump_force: f32,
    /// Gravity multiplier (applied on top of physics world gravity)
    pub gravity_multiplier: f32,
    /// Maximum slope angle (degrees) that can be walked on
    pub max_slope_angle: f32,
    /// Step height for climbing stairs
    pub step_height: f32,
    /// Character height (for capsule collider)
    pub height: f32,
    /// Character radius (for capsule collider)
    pub radius: f32,
    /// Is the character on the ground?
    pub is_grounded: bool,
    /// Is the character crouching?
    pub is_crouching: bool,
    /// Is the character sprinting?
    pub is_sprinting: bool,
    /// Current vertical velocity (for jumping/falling)
    pub vertical_velocity: f32,
    /// Time since last jump (for jump cooldown)
    pub time_since_jump: f32,
    /// Jump cooldown duration
    pub jump_cooldown: f32,
    /// Air control factor (0.0 = no air control, 1.0 = full control)
    pub air_control: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            sprint_multiplier: 2.0,
            crouch_multiplier: 0.5,
            jump_force: 10.0,
            gravity_multiplier: 1.0,
            max_slope_angle: 45.0,
            step_height: 0.3,
            height: 1.8,
            radius: 0.4,
            is_grounded: false,
            is_crouching: false,
            is_sprinting: false,
            vertical_velocity: 0.0,
            time_since_jump: 0.0,
            jump_cooldown: 0.2,
            air_control: 0.3,
        }
    }
}

impl CharacterController {
    /// Create a new character controller with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a character controller with custom dimensions
    pub fn with_dimensions(height: f32, radius: f32) -> Self {
        Self {
            height,
            radius,
            ..Default::default()
        }
    }
    
    /// Create a character controller with custom movement speeds
    pub fn with_speeds(move_speed: f32, sprint_multiplier: f32, crouch_multiplier: f32) -> Self {
        Self {
            move_speed,
            sprint_multiplier,
            crouch_multiplier,
            ..Default::default()
        }
    }
    
    /// Get the current effective move speed based on state
    pub fn get_effective_speed(&self) -> f32 {
        let mut speed = self.move_speed;
        
        if self.is_sprinting && !self.is_crouching {
            speed *= self.sprint_multiplier;
        } else if self.is_crouching {
            speed *= self.crouch_multiplier;
        }
        
        // Apply air control if not grounded
        if !self.is_grounded {
            speed *= self.air_control;
        }
        
        speed
    }
    
    /// Get the current character height (adjusted for crouching)
    pub fn get_effective_height(&self) -> f32 {
        if self.is_crouching {
            self.height * 0.6 // Crouch to 60% of normal height
        } else {
            self.height
        }
    }
    
    /// Check if the character can jump
    pub fn can_jump(&self) -> bool {
        self.is_grounded && self.time_since_jump >= self.jump_cooldown
    }
    
    /// Attempt to jump
    pub fn jump(&mut self) -> bool {
        if self.can_jump() {
            self.vertical_velocity = self.jump_force;
            self.is_grounded = false;
            self.time_since_jump = 0.0;
            true
        } else {
            false
        }
    }
    
    /// Set crouching state
    pub fn set_crouching(&mut self, crouching: bool) {
        self.is_crouching = crouching;
    }
    
    /// Set sprinting state
    pub fn set_sprinting(&mut self, sprinting: bool) {
        self.is_sprinting = sprinting;
    }
    
    /// Update the character controller (call every frame)
    pub fn update(&mut self, delta: f32) {
        self.time_since_jump += delta;
    }
    
    // TODO: Implement create_rigid_body and create_collider when implementing character controller
    // These will use PhysicsWorld3D methods directly
    
    // /// Create a rigid body for this character controller
    // pub fn create_rigid_body(&self, physics: &mut PhysicsWorld3D, entity_id: u64, position: Vec3) -> RigidBodyHandle {
    //     physics.create_kinematic_body(entity_id, position, Quat::IDENTITY)
    // }
    
    // /// Create a collider for this character controller
    // pub fn create_collider(&self, physics: &mut PhysicsWorld3D, entity_id: u64, body_handle: RigidBodyHandle) -> ColliderHandle {
    //     let half_height = self.get_effective_height() / 2.0 - self.radius;
    //     physics.add_capsule_collider(entity_id, body_handle, half_height, self.radius)
    // }
}

/// Character movement input
#[derive(Debug, Clone, Copy, Default)]
pub struct CharacterMovementInput {
    /// Forward/backward movement (-1.0 to 1.0)
    pub forward: f32,
    /// Right/left movement (-1.0 to 1.0)
    pub right: f32,
    /// Jump input
    pub jump: bool,
    /// Sprint input
    pub sprint: bool,
    /// Crouch input
    pub crouch: bool,
}

impl CharacterMovementInput {
    /// Create a new movement input
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get the movement direction (normalized)
    pub fn get_direction(&self) -> Vec3 {
        let mut dir = Vec3::new(self.right, 0.0, -self.forward);
        let len = (dir.x * dir.x + dir.z * dir.z).sqrt();
        if len > 0.0 {
            dir.x /= len;
            dir.z /= len;
        }
        dir
    }
    
    /// Get the movement magnitude (0.0 to 1.0)
    pub fn get_magnitude(&self) -> f32 {
        let mag = (self.forward * self.forward + self.right * self.right).sqrt();
        mag.min(1.0)
    }
}

/// Character controller system for updating character movement
pub struct CharacterControllerSystem;

impl CharacterControllerSystem {
    /// Update a character's movement based on input
    pub fn update_movement(
        physics: &mut PhysicsWorld3D,
        entity: Entity,
        controller: &mut CharacterController,
        input: &CharacterMovementInput,
        camera_forward: Vec3,
        delta: f32,
    ) {
        // Update controller state
        controller.update(delta);
        controller.set_sprinting(input.sprint);
        controller.set_crouching(input.crouch);
        
        // Handle jumping
        if input.jump {
            controller.jump();
        }
        
        // Get movement direction relative to camera
        let camera_right = Vec3::new(camera_forward.z, 0.0, -camera_forward.x);
        let camera_forward_flat = Vec3::new(camera_forward.x, 0.0, camera_forward.z);
        
        // Normalize camera directions
        let camera_forward_flat = Self::normalize(camera_forward_flat);
        let camera_right = Self::normalize(camera_right);
        
        // Calculate movement direction
        let move_dir = Vec3::new(
            camera_forward_flat.x * input.forward + camera_right.x * input.right,
            0.0,
            camera_forward_flat.z * input.forward + camera_right.z * input.right,
        );
        
        let move_dir = Self::normalize(move_dir);
        
        // Get effective speed
        let speed = controller.get_effective_speed();
        
        // Calculate velocity
        let velocity = Vec3::new(
            move_dir.x * speed,
            controller.vertical_velocity,
            move_dir.z * speed,
        );
        
        // Apply velocity to physics body
        physics.set_linear_velocity(entity, velocity, true);
        
        // Check if grounded (simplified - in production, use raycasting)
        if let Some(_pos) = physics.get_position(entity) {
            if let Some(vel) = physics.get_linear_velocity(entity) {
                // Simple ground check: if vertical velocity is near zero and we're moving down or stationary
                if vel.y.abs() < 0.1 && controller.vertical_velocity <= 0.0 {
                    controller.is_grounded = true;
                    controller.vertical_velocity = 0.0;
                } else {
                    controller.is_grounded = false;
                }
            }
        }
    }
    
    /// Normalize a Vec3 (helper function)
    fn normalize(v: Vec3) -> Vec3 {
        let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        if len > 0.0 {
            Vec3::new(v.x / len, v.y / len, v.z / len)
        } else {
            v
        }
    }
    
    /// Perform a ground check using raycasting
    pub fn check_ground(
        physics: &PhysicsWorld3D,
        entity: Entity,
        controller: &CharacterController,
    ) -> bool {
        if let Some(pos) = physics.get_position(entity) {
            // Cast a ray downward from the character's center
            let ray_start = pos;
            let ray_dir = Vec3::new(0.0, -1.0, 0.0);
            let ray_length = controller.radius + 0.1; // Slightly longer than radius
            
            if let Some((hit_entity, _, _, _)) = physics.raycast(ray_start, ray_dir, ray_length, true) {
                // Hit something below us (and it's not ourselves)
                return hit_entity != entity;
            }
        }
        false
    }
}

/// First-person camera controller
#[derive(Debug, Clone)]
pub struct FirstPersonCamera {
    /// Camera pitch (up/down rotation in degrees)
    pub pitch: f32,
    /// Camera yaw (left/right rotation in degrees)
    pub yaw: f32,
    /// Mouse sensitivity
    pub sensitivity: f32,
    /// Minimum pitch angle (looking down)
    pub min_pitch: f32,
    /// Maximum pitch angle (looking up)
    pub max_pitch: f32,
    /// Camera offset from character position
    pub eye_height: f32,
}

impl Default for FirstPersonCamera {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            yaw: 0.0,
            sensitivity: 0.1,
            min_pitch: -89.0,
            max_pitch: 89.0,
            eye_height: 1.6,
        }
    }
}

impl FirstPersonCamera {
    /// Create a new first-person camera
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Update camera rotation based on mouse input
    pub fn update_rotation(&mut self, mouse_delta_x: f32, mouse_delta_y: f32) {
        self.yaw += mouse_delta_x * self.sensitivity;
        self.pitch -= mouse_delta_y * self.sensitivity;
        
        // Clamp pitch
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);
        
        // Wrap yaw
        if self.yaw > 360.0 {
            self.yaw -= 360.0;
        } else if self.yaw < 0.0 {
            self.yaw += 360.0;
        }
    }
    
    /// Get the camera's forward direction
    pub fn get_forward(&self) -> Vec3 {
        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();
        
        Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
    }
    
    /// Get the camera's right direction
    pub fn get_right(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        Vec3::new(yaw_rad.cos(), 0.0, -yaw_rad.sin())
    }
    
    /// Get the camera's up direction
    pub fn get_up(&self) -> Vec3 {
        let forward = self.get_forward();
        let right = self.get_right();
        
        // Cross product: right × forward = up
        Vec3::new(
            right.y * forward.z - right.z * forward.y,
            right.z * forward.x - right.x * forward.z,
            right.x * forward.y - right.y * forward.x,
        )
    }
    
    /// Get the camera position relative to character position
    pub fn get_camera_offset(&self) -> Vec3 {
        Vec3::new(0.0, self.eye_height, 0.0)
    }
}

/// Third-person camera controller
#[derive(Debug, Clone)]
pub struct ThirdPersonCamera {
    /// Camera pitch (up/down rotation in degrees)
    pub pitch: f32,
    /// Camera yaw (left/right rotation in degrees)
    pub yaw: f32,
    /// Distance from character
    pub distance: f32,
    /// Mouse sensitivity
    pub sensitivity: f32,
    /// Minimum pitch angle
    pub min_pitch: f32,
    /// Maximum pitch angle
    pub max_pitch: f32,
    /// Minimum distance
    pub min_distance: f32,
    /// Maximum distance
    pub max_distance: f32,
    /// Camera height offset
    pub height_offset: f32,
    /// Smoothing factor (0.0 = no smoothing, 1.0 = instant)
    pub smoothing: f32,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            pitch: 20.0,
            yaw: 0.0,
            distance: 5.0,
            sensitivity: 0.1,
            min_pitch: -30.0,
            max_pitch: 70.0,
            min_distance: 2.0,
            max_distance: 10.0,
            height_offset: 1.5,
            smoothing: 0.1,
        }
    }
}

impl ThirdPersonCamera {
    /// Create a new third-person camera
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Update camera rotation based on mouse input
    pub fn update_rotation(&mut self, mouse_delta_x: f32, mouse_delta_y: f32) {
        self.yaw += mouse_delta_x * self.sensitivity;
        self.pitch -= mouse_delta_y * self.sensitivity;
        
        // Clamp pitch
        self.pitch = self.pitch.clamp(self.min_pitch, self.max_pitch);
        
        // Wrap yaw
        if self.yaw > 360.0 {
            self.yaw -= 360.0;
        } else if self.yaw < 0.0 {
            self.yaw += 360.0;
        }
    }
    
    /// Update camera distance (zoom)
    pub fn update_distance(&mut self, scroll_delta: f32) {
        self.distance -= scroll_delta;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }
    
    /// Get the camera position relative to character position
    pub fn get_camera_offset(&self) -> Vec3 {
        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();
        
        Vec3::new(
            yaw_rad.sin() * pitch_rad.cos() * self.distance,
            pitch_rad.sin() * self.distance + self.height_offset,
            yaw_rad.cos() * pitch_rad.cos() * self.distance,
        )
    }
    
    /// Get the camera's forward direction (toward character)
    pub fn get_forward(&self) -> Vec3 {
        let offset = self.get_camera_offset();
        let len = (offset.x * offset.x + offset.y * offset.y + offset.z * offset.z).sqrt();
        if len > 0.0 {
            Vec3::new(-offset.x / len, -offset.y / len, -offset.z / len)
        } else {
            Vec3::new(0.0, 0.0, -1.0)
        }
    }
}

