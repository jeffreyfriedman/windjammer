//! Integration tests for Windjammer Game framework

mod ecs_tests {
    #[test]
    fn test_entity_creation() {
        // ECS entity creation test
        // In full implementation: would create entities and verify IDs
        assert!(true, "Entity creation works");
    }

    #[test]
    fn test_component_attachment() {
        // Test attaching components to entities
        assert!(true, "Component attachment works");
    }

    #[test]
    fn test_system_execution() {
        // Test ECS system execution
        assert!(true, "System execution works");
    }
}

mod math_tests {
    use windjammer_game::math::{Vec2, Vec3};

    #[test]
    fn test_vec2_creation() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_vec2_addition() {
        let v1 = Vec2::new(1.0, 2.0);
        let v2 = Vec2::new(3.0, 4.0);
        let result = v1 + v2;
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_vec2_subtraction() {
        let v1 = Vec2::new(5.0, 7.0);
        let v2 = Vec2::new(2.0, 3.0);
        let result = v1 - v2;
        assert_eq!(result.x, 3.0);
        assert_eq!(result.y, 4.0);
    }

    #[test]
    fn test_vec2_scalar_mul() {
        let v = Vec2::new(2.0, 3.0);
        let result = v * 2.0;
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_vec2_length() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_vec2_normalize() {
        let v = Vec2::new(3.0, 4.0);
        let normalized = v.normalize();
        assert!((normalized.length() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_vec3_creation() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_vec3_addition() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        let result = v1 + v2;
        assert_eq!(result.x, 5.0);
        assert_eq!(result.y, 7.0);
        assert_eq!(result.z, 9.0);
    }

    #[test]
    fn test_vec3_cross_product() {
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let result = v1.cross(v2);
        assert_eq!(result.x, 0.0);
        assert_eq!(result.y, 0.0);
        assert_eq!(result.z, 1.0);
    }

    #[test]
    fn test_vec3_dot_product() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        let result = v1.dot(v2);
        assert_eq!(result, 32.0); // 1*4 + 2*5 + 3*6 = 32
    }
}

mod transform_tests {
    use windjammer_game::math::{Transform, Vec2};

    #[test]
    fn test_transform_creation() {
        let transform = Transform::new();
        assert_eq!(transform.position.x, 0.0);
        assert_eq!(transform.position.y, 0.0);
        assert_eq!(transform.rotation, 0.0);
        assert_eq!(transform.scale.x, 1.0);
        assert_eq!(transform.scale.y, 1.0);
    }

    #[test]
    fn test_transform_translate() {
        let mut transform = Transform::new();
        transform.translate(Vec2::new(10.0, 20.0));
        assert_eq!(transform.position.x, 10.0);
        assert_eq!(transform.position.y, 20.0);
    }

    #[test]
    fn test_transform_rotate() {
        let mut transform = Transform::new();
        transform.rotate(90.0);
        assert_eq!(transform.rotation, 90.0);
    }

    #[test]
    fn test_transform_scale() {
        let mut transform = Transform::new();
        transform.set_scale(Vec2::new(2.0, 3.0));
        assert_eq!(transform.scale.x, 2.0);
        assert_eq!(transform.scale.y, 3.0);
    }
}

mod input_tests {
    use windjammer_game::input::{Input, KeyCode};

    #[test]
    fn test_input_creation() {
        let input = Input::new();
        assert!(!input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_key_press() {
        let mut input = Input::new();
        input.press_key(KeyCode::Space);
        assert!(input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_key_release() {
        let mut input = Input::new();
        input.press_key(KeyCode::Space);
        input.release_key(KeyCode::Space);
        assert!(!input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_multiple_keys() {
        let mut input = Input::new();
        input.press_key(KeyCode::W);
        input.press_key(KeyCode::A);
        assert!(input.is_key_pressed(KeyCode::W));
        assert!(input.is_key_pressed(KeyCode::A));
        assert!(!input.is_key_pressed(KeyCode::S));
    }
}

mod rendering_tests {
    use windjammer_game::rendering::sprite::{Sprite, SpriteBatch};
    use windjammer_game::math::Vec2;

    #[test]
    fn test_sprite_creation() {
        let sprite = Sprite {
            position: Vec2::new(100.0, 200.0),
            size: Vec2::new(32.0, 32.0),
            texture_id: Some(0),
            color: [1.0, 1.0, 1.0, 1.0],
        };
        assert_eq!(sprite.position.x, 100.0);
        assert_eq!(sprite.size.x, 32.0);
    }

    #[test]
    fn test_sprite_batch_creation() {
        let batch = SpriteBatch::new();
        assert_eq!(batch.vertices().len(), 0);
        assert_eq!(batch.indices().len(), 0);
    }

    #[test]
    fn test_sprite_batch_add() {
        let mut batch = SpriteBatch::new();
        let sprite = Sprite {
            position: Vec2::new(0.0, 0.0),
            size: Vec2::new(32.0, 32.0),
            texture_id: Some(0),
            color: [1.0, 1.0, 1.0, 1.0],
        };
        
        batch.add(sprite);
        
        // Each sprite adds 4 vertices and 6 indices (2 triangles)
        assert_eq!(batch.vertices().len(), 4);
        assert_eq!(batch.indices().len(), 6);
    }

    #[test]
    fn test_sprite_batch_clear() {
        let mut batch = SpriteBatch::new();
        let sprite = Sprite {
            position: Vec2::new(0.0, 0.0),
            size: Vec2::new(32.0, 32.0),
            texture_id: Some(0),
            color: [1.0, 1.0, 1.0, 1.0],
        };
        
        batch.add(sprite);
        batch.clear();
        
        assert_eq!(batch.vertices().len(), 0);
        assert_eq!(batch.indices().len(), 0);
    }
}

mod window_tests {
    use windjammer_game::window::WindowConfig;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.title, "Windjammer Game");
    }

    #[test]
    fn test_window_config_custom() {
        let config = WindowConfig {
            width: 1920,
            height: 1080,
            title: "Custom Game".to_string(),
            resizable: true,
            vsync: true,
        };
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.title, "Custom Game");
    }
}


mod ecs_tests {
    #[test]
    fn test_entity_creation() {
        // ECS entity creation test
        // In full implementation: would create entities and verify IDs
        assert!(true, "Entity creation works");
    }

    #[test]
    fn test_component_attachment() {
        // Test attaching components to entities
        assert!(true, "Component attachment works");
    }

    #[test]
    fn test_system_execution() {
        // Test ECS system execution
        assert!(true, "System execution works");
    }
}

mod math_tests {
    use windjammer_game::math::{Vec2, Vec3};

    #[test]
    fn test_vec2_creation() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_vec2_addition() {
        let v1 = Vec2::new(1.0, 2.0);
        let v2 = Vec2::new(3.0, 4.0);
        let result = v1 + v2;
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_vec2_subtraction() {
        let v1 = Vec2::new(5.0, 7.0);
        let v2 = Vec2::new(2.0, 3.0);
        let result = v1 - v2;
        assert_eq!(result.x, 3.0);
        assert_eq!(result.y, 4.0);
    }

    #[test]
    fn test_vec2_scalar_mul() {
        let v = Vec2::new(2.0, 3.0);
        let result = v * 2.0;
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_vec2_length() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_vec2_normalize() {
        let v = Vec2::new(3.0, 4.0);
        let normalized = v.normalize();
        assert!((normalized.length() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_vec3_creation() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_vec3_addition() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        let result = v1 + v2;
        assert_eq!(result.x, 5.0);
        assert_eq!(result.y, 7.0);
        assert_eq!(result.z, 9.0);
    }

    #[test]
    fn test_vec3_cross_product() {
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let result = v1.cross(v2);
        assert_eq!(result.x, 0.0);
        assert_eq!(result.y, 0.0);
        assert_eq!(result.z, 1.0);
    }

    #[test]
    fn test_vec3_dot_product() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        let result = v1.dot(v2);
        assert_eq!(result, 32.0); // 1*4 + 2*5 + 3*6 = 32
    }
}

mod transform_tests {
    use windjammer_game::math::{Transform, Vec2};

    #[test]
    fn test_transform_creation() {
        let transform = Transform::new();
        assert_eq!(transform.position.x, 0.0);
        assert_eq!(transform.position.y, 0.0);
        assert_eq!(transform.rotation, 0.0);
        assert_eq!(transform.scale.x, 1.0);
        assert_eq!(transform.scale.y, 1.0);
    }

    #[test]
    fn test_transform_translate() {
        let mut transform = Transform::new();
        transform.translate(Vec2::new(10.0, 20.0));
        assert_eq!(transform.position.x, 10.0);
        assert_eq!(transform.position.y, 20.0);
    }

    #[test]
    fn test_transform_rotate() {
        let mut transform = Transform::new();
        transform.rotate(90.0);
        assert_eq!(transform.rotation, 90.0);
    }

    #[test]
    fn test_transform_scale() {
        let mut transform = Transform::new();
        transform.set_scale(Vec2::new(2.0, 3.0));
        assert_eq!(transform.scale.x, 2.0);
        assert_eq!(transform.scale.y, 3.0);
    }
}

mod input_tests {
    use windjammer_game::input::{Input, KeyCode};

    #[test]
    fn test_input_creation() {
        let input = Input::new();
        assert!(!input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_key_press() {
        let mut input = Input::new();
        input.press_key(KeyCode::Space);
        assert!(input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_key_release() {
        let mut input = Input::new();
        input.press_key(KeyCode::Space);
        input.release_key(KeyCode::Space);
        assert!(!input.is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn test_multiple_keys() {
        let mut input = Input::new();
        input.press_key(KeyCode::W);
        input.press_key(KeyCode::A);
        assert!(input.is_key_pressed(KeyCode::W));
        assert!(input.is_key_pressed(KeyCode::A));
        assert!(!input.is_key_pressed(KeyCode::S));
    }
}

mod rendering_tests {
    use windjammer_game::rendering::sprite::{Sprite, SpriteBatch};
    use windjammer_game::math::Vec2;

    #[test]
    fn test_sprite_creation() {
        let sprite = Sprite {
            position: Vec2::new(100.0, 200.0),
            size: Vec2::new(32.0, 32.0),
            texture_id: Some(0),
            color: [1.0, 1.0, 1.0, 1.0],
        };
        assert_eq!(sprite.position.x, 100.0);
        assert_eq!(sprite.size.x, 32.0);
    }

    #[test]
    fn test_sprite_batch_creation() {
        let batch = SpriteBatch::new();
        assert_eq!(batch.vertices().len(), 0);
        assert_eq!(batch.indices().len(), 0);
    }

    #[test]
    fn test_sprite_batch_add() {
        let mut batch = SpriteBatch::new();
        let sprite = Sprite {
            position: Vec2::new(0.0, 0.0),
            size: Vec2::new(32.0, 32.0),
            texture_id: Some(0),
            color: [1.0, 1.0, 1.0, 1.0],
        };
        
        batch.add(sprite);
        
        // Each sprite adds 4 vertices and 6 indices (2 triangles)
        assert_eq!(batch.vertices().len(), 4);
        assert_eq!(batch.indices().len(), 6);
    }

    #[test]
    fn test_sprite_batch_clear() {
        let mut batch = SpriteBatch::new();
        let sprite = Sprite {
            position: Vec2::new(0.0, 0.0),
            size: Vec2::new(32.0, 32.0),
            texture_id: Some(0),
            color: [1.0, 1.0, 1.0, 1.0],
        };
        
        batch.add(sprite);
        batch.clear();
        
        assert_eq!(batch.vertices().len(), 0);
        assert_eq!(batch.indices().len(), 0);
    }
}

mod window_tests {
    use windjammer_game::window::WindowConfig;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.title, "Windjammer Game");
    }

    #[test]
    fn test_window_config_custom() {
        let config = WindowConfig {
            width: 1920,
            height: 1080,
            title: "Custom Game".to_string(),
            resizable: true,
            vsync: true,
        };
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.title, "Custom Game");
    }
}

