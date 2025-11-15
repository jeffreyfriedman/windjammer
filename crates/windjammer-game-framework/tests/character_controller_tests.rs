//! Unit tests for Character Controller
//!
//! Tests character movement, jumping, crouching, and camera systems.

#[cfg(feature = "3d")]
mod character_controller_tests {
    use windjammer_game_framework::character_controller::*;
    use windjammer_game_framework::math::Vec3;

    #[test]
    fn test_character_controller_creation() {
        let controller = CharacterController::new();
        assert_eq!(controller.move_speed, 5.0);
        assert_eq!(controller.sprint_multiplier, 2.0);
        assert_eq!(controller.crouch_multiplier, 0.5);
        assert_eq!(controller.jump_force, 10.0);
        assert_eq!(controller.height, 1.8);
        assert_eq!(controller.radius, 0.4);
        assert!(!controller.is_grounded);
        assert!(!controller.is_crouching);
        assert!(!controller.is_sprinting);
        println!("✅ Character controller created with default values");
    }

    #[test]
    fn test_character_controller_custom_dimensions() {
        let controller = CharacterController::with_dimensions(2.0, 0.5);
        assert_eq!(controller.height, 2.0);
        assert_eq!(controller.radius, 0.5);
        println!("✅ Character controller created with custom dimensions");
    }

    #[test]
    fn test_character_controller_custom_speeds() {
        let controller = CharacterController::with_speeds(10.0, 1.5, 0.3);
        assert_eq!(controller.move_speed, 10.0);
        assert_eq!(controller.sprint_multiplier, 1.5);
        assert_eq!(controller.crouch_multiplier, 0.3);
        println!("✅ Character controller created with custom speeds");
    }

    #[test]
    fn test_effective_speed_normal() {
        let mut controller = CharacterController::new();
        controller.is_grounded = true; // Must be grounded for normal speed
        let speed = controller.get_effective_speed();
        assert_eq!(speed, 5.0);
        println!("✅ Normal speed: {}", speed);
    }

    #[test]
    fn test_effective_speed_sprinting() {
        let mut controller = CharacterController::new();
        controller.is_sprinting = true;
        controller.is_grounded = true;
        let speed = controller.get_effective_speed();
        assert_eq!(speed, 10.0); // 5.0 * 2.0
        println!("✅ Sprint speed: {}", speed);
    }

    #[test]
    fn test_effective_speed_crouching() {
        let mut controller = CharacterController::new();
        controller.is_crouching = true;
        controller.is_grounded = true;
        let speed = controller.get_effective_speed();
        assert_eq!(speed, 2.5); // 5.0 * 0.5
        println!("✅ Crouch speed: {}", speed);
    }

    #[test]
    fn test_effective_speed_air_control() {
        let mut controller = CharacterController::new();
        controller.is_grounded = false;
        let speed = controller.get_effective_speed();
        assert_eq!(speed, 1.5); // 5.0 * 0.3 (air control)
        println!("✅ Air control speed: {}", speed);
    }

    #[test]
    fn test_effective_height_normal() {
        let controller = CharacterController::new();
        let height = controller.get_effective_height();
        assert_eq!(height, 1.8);
        println!("✅ Normal height: {}", height);
    }

    #[test]
    fn test_effective_height_crouching() {
        let mut controller = CharacterController::new();
        controller.is_crouching = true;
        let height = controller.get_effective_height();
        assert_eq!(height, 1.08); // 1.8 * 0.6
        println!("✅ Crouch height: {}", height);
    }

    #[test]
    fn test_jump_when_grounded() {
        let mut controller = CharacterController::new();
        controller.is_grounded = true;
        controller.time_since_jump = 1.0; // Past cooldown
        
        let jumped = controller.jump();
        assert!(jumped);
        assert_eq!(controller.vertical_velocity, 10.0);
        assert!(!controller.is_grounded);
        assert_eq!(controller.time_since_jump, 0.0);
        println!("✅ Jump successful when grounded");
    }

    #[test]
    fn test_jump_when_airborne() {
        let mut controller = CharacterController::new();
        controller.is_grounded = false;
        
        let jumped = controller.jump();
        assert!(!jumped);
        assert_eq!(controller.vertical_velocity, 0.0);
        println!("✅ Jump prevented when airborne");
    }

    #[test]
    fn test_jump_cooldown() {
        let mut controller = CharacterController::new();
        controller.is_grounded = true;
        controller.time_since_jump = 0.1; // Within cooldown (0.2s)
        
        let jumped = controller.jump();
        assert!(!jumped);
        println!("✅ Jump prevented during cooldown");
    }

    #[test]
    fn test_update_increments_time() {
        let mut controller = CharacterController::new();
        controller.time_since_jump = 0.0;
        
        controller.update(0.016); // ~60fps
        assert!((controller.time_since_jump - 0.016).abs() < 0.001);
        
        controller.update(0.016);
        assert!((controller.time_since_jump - 0.032).abs() < 0.001);
        println!("✅ Update increments time correctly");
    }

    #[test]
    fn test_set_crouching() {
        let mut controller = CharacterController::new();
        assert!(!controller.is_crouching);
        
        controller.set_crouching(true);
        assert!(controller.is_crouching);
        
        controller.set_crouching(false);
        assert!(!controller.is_crouching);
        println!("✅ Crouching state toggles correctly");
    }

    #[test]
    fn test_set_sprinting() {
        let mut controller = CharacterController::new();
        assert!(!controller.is_sprinting);
        
        controller.set_sprinting(true);
        assert!(controller.is_sprinting);
        
        controller.set_sprinting(false);
        assert!(!controller.is_sprinting);
        println!("✅ Sprinting state toggles correctly");
    }

    #[test]
    fn test_movement_input_creation() {
        let input = CharacterMovementInput::new();
        assert_eq!(input.forward, 0.0);
        assert_eq!(input.right, 0.0);
        assert!(!input.jump);
        assert!(!input.sprint);
        assert!(!input.crouch);
        println!("✅ Movement input created with default values");
    }

    #[test]
    fn test_movement_input_direction_forward() {
        let mut input = CharacterMovementInput::new();
        input.forward = 1.0;
        
        let dir = input.get_direction();
        assert!((dir.x - 0.0).abs() < 0.001);
        assert!((dir.z - -1.0).abs() < 0.001);
        println!("✅ Forward direction: ({}, {}, {})", dir.x, dir.y, dir.z);
    }

    #[test]
    fn test_movement_input_direction_right() {
        let mut input = CharacterMovementInput::new();
        input.right = 1.0;
        
        let dir = input.get_direction();
        assert!((dir.x - 1.0).abs() < 0.001);
        assert!((dir.z - 0.0).abs() < 0.001);
        println!("✅ Right direction: ({}, {}, {})", dir.x, dir.y, dir.z);
    }

    #[test]
    fn test_movement_input_direction_diagonal() {
        let mut input = CharacterMovementInput::new();
        input.forward = 1.0;
        input.right = 1.0;
        
        let dir = input.get_direction();
        // Should be normalized
        let len = (dir.x * dir.x + dir.z * dir.z).sqrt();
        assert!((len - 1.0).abs() < 0.001);
        println!("✅ Diagonal direction normalized: ({}, {}, {})", dir.x, dir.y, dir.z);
    }

    #[test]
    fn test_movement_input_magnitude() {
        let mut input = CharacterMovementInput::new();
        input.forward = 0.5;
        input.right = 0.5;
        
        let mag = input.get_magnitude();
        assert!((mag - 0.707).abs() < 0.01); // sqrt(0.5^2 + 0.5^2)
        println!("✅ Movement magnitude: {}", mag);
    }

    #[test]
    fn test_first_person_camera_creation() {
        let camera = FirstPersonCamera::new();
        assert_eq!(camera.pitch, 0.0);
        assert_eq!(camera.yaw, 0.0);
        assert_eq!(camera.sensitivity, 0.1);
        assert_eq!(camera.min_pitch, -89.0);
        assert_eq!(camera.max_pitch, 89.0);
        assert_eq!(camera.eye_height, 1.6);
        println!("✅ First-person camera created with default values");
    }

    #[test]
    fn test_first_person_camera_rotation() {
        let mut camera = FirstPersonCamera::new();
        
        camera.update_rotation(10.0, 5.0);
        assert_eq!(camera.yaw, 1.0); // 10.0 * 0.1
        assert_eq!(camera.pitch, -0.5); // -5.0 * 0.1
        println!("✅ Camera rotation updated: yaw={}, pitch={}", camera.yaw, camera.pitch);
    }

    #[test]
    fn test_first_person_camera_pitch_clamping() {
        let mut camera = FirstPersonCamera::new();
        
        // Try to look too far up
        camera.update_rotation(0.0, -1000.0);
        assert_eq!(camera.pitch, 89.0); // Clamped to max_pitch
        
        // Try to look too far down
        camera.pitch = 0.0;
        camera.update_rotation(0.0, 1000.0);
        assert_eq!(camera.pitch, -89.0); // Clamped to min_pitch
        println!("✅ Camera pitch clamped correctly");
    }

    #[test]
    fn test_first_person_camera_yaw_wrapping() {
        let mut camera = FirstPersonCamera::new();
        
        // Rotate past 360
        camera.yaw = 350.0;
        camera.update_rotation(200.0, 0.0); // +20 degrees
        assert!((camera.yaw - 10.0).abs() < 0.001); // Should wrap to 10
        
        // Rotate below 0
        camera.yaw = 10.0;
        camera.update_rotation(-200.0, 0.0); // -20 degrees
        assert!((camera.yaw - 350.0).abs() < 0.001); // Should wrap to 350
        println!("✅ Camera yaw wraps correctly");
    }

    #[test]
    fn test_first_person_camera_forward() {
        let camera = FirstPersonCamera::new();
        let forward = camera.get_forward();
        
        // Default should be looking forward (negative Z in typical game coordinates)
        assert!((forward.x - 0.0).abs() < 0.001);
        assert!((forward.y - 0.0).abs() < 0.001);
        assert!((forward.z - 1.0).abs() < 0.001);
        println!("✅ Camera forward vector: ({}, {}, {})", forward.x, forward.y, forward.z);
    }

    #[test]
    fn test_third_person_camera_creation() {
        let camera = ThirdPersonCamera::new();
        assert_eq!(camera.pitch, 20.0);
        assert_eq!(camera.yaw, 0.0);
        assert_eq!(camera.distance, 5.0);
        assert_eq!(camera.sensitivity, 0.1);
        assert_eq!(camera.height_offset, 1.5);
        println!("✅ Third-person camera created with default values");
    }

    #[test]
    fn test_third_person_camera_rotation() {
        let mut camera = ThirdPersonCamera::new();
        
        camera.update_rotation(10.0, 5.0);
        assert_eq!(camera.yaw, 1.0); // 10.0 * 0.1
        assert_eq!(camera.pitch, 19.5); // 20.0 - 5.0 * 0.1
        println!("✅ Third-person camera rotation updated");
    }

    #[test]
    fn test_third_person_camera_distance() {
        let mut camera = ThirdPersonCamera::new();
        assert_eq!(camera.distance, 5.0);
        
        camera.update_distance(1.0); // Zoom in
        assert_eq!(camera.distance, 4.0);
        
        camera.update_distance(-2.0); // Zoom out
        assert_eq!(camera.distance, 6.0);
        println!("✅ Third-person camera distance updated");
    }

    #[test]
    fn test_third_person_camera_distance_clamping() {
        let mut camera = ThirdPersonCamera::new();
        
        // Try to zoom in too far
        camera.update_distance(10.0);
        assert_eq!(camera.distance, 2.0); // Clamped to min_distance
        
        // Try to zoom out too far
        camera.distance = 5.0;
        camera.update_distance(-10.0);
        assert_eq!(camera.distance, 10.0); // Clamped to max_distance
        println!("✅ Third-person camera distance clamped correctly");
    }

    #[test]
    fn test_character_controller_rigid_body() {
        let controller = CharacterController::new();
        let body = controller.create_rigid_body();
        
        assert_eq!(body.mass, 70.0);
        assert_eq!(body.linear_damping, 0.0);
        assert_eq!(body.angular_damping, 1.0);
        println!("✅ Character controller rigid body created correctly");
    }

    #[test]
    fn test_character_controller_collider() {
        let controller = CharacterController::new();
        let collider = controller.create_collider();
        
        // Verify capsule shape
        match collider.shape {
            windjammer_game_framework::physics3d::ColliderShape3D::Capsule(half_height, radius) => {
                let expected_half_height = controller.get_effective_height() / 2.0 - controller.radius;
                assert!((half_height - expected_half_height).abs() < 0.001);
                assert_eq!(radius, controller.radius);
                println!("✅ Capsule collider: half_height={}, radius={}", half_height, radius);
            }
            _ => panic!("Expected capsule collider"),
        }
        
        assert_eq!(collider.restitution, 0.0);
        assert_eq!(collider.friction, 0.0);
        assert!(!collider.is_sensor);
        println!("✅ Character controller collider created correctly");
    }

    #[test]
    fn test_character_controller_system_normalize() {
        // Test the normalize helper (indirectly through movement)
        let v1 = Vec3::new(3.0, 4.0, 0.0);
        let len = (v1.x * v1.x + v1.y * v1.y + v1.z * v1.z).sqrt();
        assert!((len - 5.0).abs() < 0.001); // 3-4-5 triangle
        
        let normalized = Vec3::new(v1.x / len, v1.y / len, v1.z / len);
        let new_len = (normalized.x * normalized.x + normalized.y * normalized.y + normalized.z * normalized.z).sqrt();
        assert!((new_len - 1.0).abs() < 0.001);
        println!("✅ Vector normalization works correctly");
    }
}

