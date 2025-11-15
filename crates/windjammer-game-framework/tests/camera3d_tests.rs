//! Unit tests for 3D Camera System
//!
//! Tests Camera3D, ThirdPersonCamera, FirstPersonCamera, and FreeCamera.

#[cfg(feature = "3d")]
mod camera3d_tests {
    use windjammer_game_framework::camera3d::*;
    use windjammer_game_framework::input::{Input, Key, MouseButton};
    use windjammer_game_framework::math::{Quat, Vec3};

    // ============================================================================
    // Camera3D Tests
    // ============================================================================

    #[test]
    fn test_camera3d_creation() {
        let camera = Camera3D::default();
        assert_eq!(camera.position, Vec3::ZERO);
        assert_eq!(camera.rotation, Quat::IDENTITY);
        println!("✅ Camera3D created");
    }

    #[test]
    fn test_camera3d_perspective() {
        let camera = Camera3D::perspective(
            std::f32::consts::PI / 4.0, // 45 degree FOV
            16.0 / 9.0,                 // 16:9 aspect
            0.1,                        // Near
            1000.0,                     // Far
        );
        
        match camera.projection {
            CameraProjection::Perspective { fov, aspect, near, far } => {
                assert_eq!(fov, std::f32::consts::PI / 4.0);
                assert_eq!(aspect, 16.0 / 9.0);
                assert_eq!(near, 0.1);
                assert_eq!(far, 1000.0);
            }
            _ => panic!("Expected perspective projection"),
        }
        println!("✅ Perspective camera created");
    }

    #[test]
    fn test_camera3d_orthographic() {
        let camera = Camera3D::orthographic(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        
        match camera.projection {
            CameraProjection::Orthographic { left, right, bottom, top, near, far } => {
                assert_eq!(left, -10.0);
                assert_eq!(right, 10.0);
                assert_eq!(bottom, -10.0);
                assert_eq!(top, 10.0);
                assert_eq!(near, 0.1);
                assert_eq!(far, 100.0);
            }
            _ => panic!("Expected orthographic projection"),
        }
        println!("✅ Orthographic camera created");
    }

    #[test]
    fn test_camera3d_directions() {
        let camera = Camera3D::default();
        
        let forward = camera.forward();
        let right = camera.right();
        let up = camera.up();
        
        // Default camera looks down -Z (forward), right is +X, up is +Y
        assert!((forward.z - -1.0).abs() < 0.001, "Forward should be -Z");
        assert!((right.x - 1.0).abs() < 0.001, "Right should be +X");
        assert!((up.y - 1.0).abs() < 0.001, "Up should be +Y");
        
        println!("✅ Camera directions: forward={:?}, right={:?}, up={:?}", forward, right, up);
    }

    #[test]
    fn test_camera3d_view_matrix() {
        let camera = Camera3D::default();
        let view_matrix = camera.view_matrix();
        
        // View matrix should be identity for camera at origin with no rotation
        assert!(view_matrix.is_finite());
        println!("✅ View matrix generated");
    }

    #[test]
    fn test_camera3d_projection_matrix() {
        let camera = Camera3D::default();
        let proj_matrix = camera.projection_matrix();
        
        assert!(proj_matrix.is_finite());
        println!("✅ Projection matrix generated");
    }

    #[test]
    fn test_camera3d_view_projection_matrix() {
        let camera = Camera3D::default();
        let vp_matrix = camera.view_projection_matrix();
        
        assert!(vp_matrix.is_finite());
        println!("✅ View-projection matrix generated");
    }

    // ============================================================================
    // ThirdPersonCamera Tests
    // ============================================================================

    #[test]
    fn test_third_person_camera_creation() {
        let camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.distance, 10.0);
        assert!(camera.distance >= camera.min_distance);
        assert!(camera.distance <= camera.max_distance);
        
        println!("✅ ThirdPersonCamera created: target={:?}, distance={}", camera.target, camera.distance);
    }

    #[test]
    fn test_third_person_camera_default() {
        let camera = ThirdPersonCamera::default();
        
        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.distance, 10.0);
        
        println!("✅ ThirdPersonCamera default");
    }

    #[test]
    fn test_third_person_camera_set_target() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        
        camera.set_target(Vec3::new(5.0, 0.0, 5.0));
        assert_eq!(camera.target, Vec3::new(5.0, 0.0, 5.0));
        
        println!("✅ ThirdPersonCamera set_target");
    }

    #[test]
    fn test_third_person_camera_set_distance() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        
        camera.set_distance(15.0);
        assert_eq!(camera.distance, 15.0);
        
        // Test clamping
        camera.set_distance(1.0); // Below min
        assert_eq!(camera.distance, camera.min_distance);
        
        camera.set_distance(100.0); // Above max
        assert_eq!(camera.distance, camera.max_distance);
        
        println!("✅ ThirdPersonCamera set_distance with clamping");
    }

    #[test]
    fn test_third_person_camera_set_angles() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        
        camera.set_yaw(std::f32::consts::PI / 4.0);
        assert_eq!(camera.yaw, std::f32::consts::PI / 4.0);
        
        camera.set_pitch(0.5);
        assert_eq!(camera.pitch, 0.5);
        
        // Test pitch clamping
        camera.set_pitch(std::f32::consts::PI); // Too high
        assert!(camera.pitch <= camera.max_pitch);
        
        camera.set_pitch(-std::f32::consts::PI); // Too low
        assert!(camera.pitch >= camera.min_pitch);
        
        println!("✅ ThirdPersonCamera set_yaw and set_pitch with clamping");
    }

    #[test]
    fn test_third_person_camera_update() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        let input = Input::new();
        
        // Update without input
        camera.update(&input, 0.016); // 60 FPS
        
        // Camera should remain stable
        assert_eq!(camera.target, Vec3::ZERO);
        
        println!("✅ ThirdPersonCamera update");
    }

    #[test]
    fn test_third_person_camera_zoom() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        let mut input = Input::new();
        
        let initial_distance = camera.distance;
        
        // Simulate zoom in
        input.simulate_key_press(Key::Num9);
        camera.update(&input, 0.1);
        
        assert!(camera.distance < initial_distance, "Distance should decrease when zooming in");
        
        println!("✅ ThirdPersonCamera zoom");
    }

    #[test]
    fn test_third_person_camera_orbit() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        let mut input = Input::new();
        
        let initial_yaw = camera.yaw;
        
        // Simulate mouse orbit
        input.simulate_mouse_press(MouseButton::Right);
        input.simulate_mouse_move(100.0, 100.0);
        input.clear_frame_state();
        input.simulate_mouse_move(150.0, 100.0); // Move right
        
        camera.update(&input, 0.016);
        
        assert_ne!(camera.yaw, initial_yaw, "Yaw should change when orbiting");
        
        println!("✅ ThirdPersonCamera orbit");
    }

    // ============================================================================
    // FirstPersonCamera Tests
    // ============================================================================

    #[test]
    fn test_first_person_camera_creation() {
        let camera = FirstPersonCamera::new(Vec3::new(0.0, 1.8, 0.0));
        
        assert_eq!(camera.camera.position, Vec3::new(0.0, 1.8, 0.0));
        assert_eq!(camera.yaw, 0.0);
        assert_eq!(camera.pitch, 0.0);
        
        println!("✅ FirstPersonCamera created at eye height");
    }

    #[test]
    fn test_first_person_camera_default() {
        let camera = FirstPersonCamera::default();
        
        assert_eq!(camera.camera.position.y, 1.8); // Eye height
        
        println!("✅ FirstPersonCamera default");
    }

    #[test]
    fn test_first_person_camera_mouse_look() {
        let mut camera = FirstPersonCamera::new(Vec3::ZERO);
        let mut input = Input::new();
        
        let initial_yaw = camera.yaw;
        let initial_pitch = camera.pitch;
        
        // Simulate mouse movement
        input.simulate_mouse_move(100.0, 100.0);
        input.clear_frame_state();
        input.simulate_mouse_move(150.0, 120.0); // Move right and down
        
        camera.update(&input, 0.016);
        
        assert_ne!(camera.yaw, initial_yaw, "Yaw should change");
        assert_ne!(camera.pitch, initial_pitch, "Pitch should change");
        
        println!("✅ FirstPersonCamera mouse look");
    }

    #[test]
    fn test_first_person_camera_movement() {
        let mut camera = FirstPersonCamera::new(Vec3::ZERO);
        let mut input = Input::new();
        
        let initial_pos = camera.camera.position;
        
        // Simulate WASD movement
        input.simulate_key_press(Key::W);
        camera.update(&input, 0.1);
        
        assert_ne!(camera.camera.position, initial_pos, "Position should change when moving");
        
        println!("✅ FirstPersonCamera movement");
    }

    #[test]
    fn test_first_person_camera_pitch_clamping() {
        let mut camera = FirstPersonCamera::new(Vec3::ZERO);
        
        // Try to look straight up (should be clamped)
        camera.pitch = std::f32::consts::PI;
        camera.update(&Input::new(), 0.016);
        
        assert!(camera.pitch <= camera.max_pitch, "Pitch should be clamped to max");
        
        // Try to look straight down (should be clamped)
        camera.pitch = -std::f32::consts::PI;
        camera.update(&Input::new(), 0.016);
        
        assert!(camera.pitch >= camera.min_pitch, "Pitch should be clamped to min");
        
        println!("✅ FirstPersonCamera pitch clamping");
    }

    // ============================================================================
    // FreeCamera Tests
    // ============================================================================

    #[test]
    fn test_free_camera_creation() {
        let camera = FreeCamera::new(Vec3::new(0.0, 5.0, 10.0));
        
        assert_eq!(camera.camera.position, Vec3::new(0.0, 5.0, 10.0));
        assert_eq!(camera.yaw, 0.0);
        assert_eq!(camera.pitch, 0.0);
        
        println!("✅ FreeCamera created");
    }

    #[test]
    fn test_free_camera_default() {
        let camera = FreeCamera::default();
        
        assert_eq!(camera.camera.position, Vec3::new(0.0, 5.0, 10.0));
        
        println!("✅ FreeCamera default");
    }

    #[test]
    fn test_free_camera_mouse_look() {
        let mut camera = FreeCamera::new(Vec3::ZERO);
        let mut input = Input::new();
        
        let initial_yaw = camera.yaw;
        
        // Simulate mouse look (only works with right mouse button)
        input.simulate_mouse_press(MouseButton::Right);
        input.simulate_mouse_move(100.0, 100.0);
        input.clear_frame_state();
        input.simulate_mouse_move(150.0, 100.0);
        
        camera.update(&input, 0.016);
        
        assert_ne!(camera.yaw, initial_yaw, "Yaw should change when right mouse is held");
        
        println!("✅ FreeCamera mouse look");
    }

    #[test]
    fn test_free_camera_movement() {
        let mut camera = FreeCamera::new(Vec3::ZERO);
        let mut input = Input::new();
        
        let initial_pos = camera.camera.position;
        
        // Simulate movement
        input.simulate_key_press(Key::W);
        camera.update(&input, 0.1);
        
        assert_ne!(camera.camera.position, initial_pos, "Position should change");
        
        println!("✅ FreeCamera movement");
    }

    #[test]
    fn test_free_camera_fast_movement() {
        let mut camera = FreeCamera::new(Vec3::ZERO);
        let mut input = Input::new();
        
        // Normal movement
        input.simulate_key_press(Key::W);
        camera.update(&input, 0.1);
        let normal_distance = camera.camera.position.length();
        
        // Reset
        camera.camera.position = Vec3::ZERO;
        input.clear_frame_state();
        
        // Fast movement (with Shift)
        input.simulate_key_press(Key::W);
        input.simulate_key_press(Key::Shift);
        camera.update(&input, 0.1);
        let fast_distance = camera.camera.position.length();
        
        assert!(fast_distance > normal_distance, "Fast movement should be faster");
        
        println!("✅ FreeCamera fast movement: normal={}, fast={}", normal_distance, fast_distance);
    }

    #[test]
    fn test_free_camera_vertical_movement() {
        let mut camera = FreeCamera::new(Vec3::ZERO);
        let mut input = Input::new();
        
        // Move up
        input.simulate_key_press(Key::Space);
        camera.update(&input, 0.1);
        
        assert!(camera.camera.position.y > 0.0, "Should move up: y={}", camera.camera.position.y);
        
        // Reset camera and input for down movement
        camera.camera.position = Vec3::ZERO;
        let mut input2 = Input::new();
        input2.simulate_key_press(Key::Control);
        camera.update(&input2, 0.1);
        
        assert!(camera.camera.position.y < 0.0, "Should move down: y={}", camera.camera.position.y);
        
        println!("✅ FreeCamera vertical movement");
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_camera_types_interoperability() {
        // All camera types should produce valid view-projection matrices
        let third_person = ThirdPersonCamera::default();
        let first_person = FirstPersonCamera::default();
        let free_camera = FreeCamera::default();
        
        assert!(third_person.view_projection_matrix().is_finite());
        assert!(first_person.view_projection_matrix().is_finite());
        assert!(free_camera.view_projection_matrix().is_finite());
        
        println!("✅ All camera types produce valid matrices");
    }

    #[test]
    fn test_camera_smoothing() {
        let mut camera = ThirdPersonCamera::new(Vec3::ZERO, 10.0);
        
        // Set high smoothing
        camera.smoothing = 0.9;
        
        // Move target far away
        camera.set_target(Vec3::new(100.0, 0.0, 0.0));
        
        // Update once
        camera.update(&Input::new(), 0.016);
        
        // Camera should not have reached target yet (due to smoothing)
        let distance_to_target = (camera.camera.position - camera.target).length();
        assert!(distance_to_target > 1.0, "Camera should lag behind target with high smoothing");
        
        println!("✅ Camera smoothing works");
    }
}

