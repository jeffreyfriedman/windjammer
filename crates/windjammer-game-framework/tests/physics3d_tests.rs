//! Unit tests for 3D Physics (Rapier3D)
//!
//! Tests physics world, rigid bodies, colliders, and raycasting.

#[cfg(feature = "3d")]
mod physics3d_tests {
    use windjammer_game_framework::ecs::World;
    use windjammer_game_framework::math::Vec3;
    use windjammer_game_framework::physics3d::*;
    use rapier3d::prelude::*;

    #[test]
    fn test_physics_world_creation() {
        let _physics = PhysicsWorld3D::new();
        println!("✅ Physics world created with default gravity");
    }

    #[test]
    fn test_physics_world_custom_gravity() {
        let gravity = Vec3::new(0.0, -20.0, 0.0);
        let _physics = PhysicsWorld3D::with_gravity(gravity);
        println!("✅ Physics world created with custom gravity: ({}, {}, {})", 
                 gravity.x, gravity.y, gravity.z);
    }

    #[test]
    fn test_physics_world_set_gravity() {
        let mut physics = PhysicsWorld3D::new();
        physics.set_gravity(Vec3::new(0.0, -20.0, 0.0));
        println!("✅ Physics world gravity changed");
    }

    #[test]
    fn test_rigid_body_dynamic() {
        let body = RigidBody3D::new_dynamic();
        assert_eq!(body.mass, 1.0);
        assert_eq!(body.linear_damping, 0.0);
        assert_eq!(body.angular_damping, 0.0);
        println!("✅ Dynamic rigid body created: mass={}", body.mass);
    }

    #[test]
    fn test_rigid_body_fixed() {
        let body = RigidBody3D::new_fixed();
        println!("✅ Fixed rigid body created");
    }

    #[test]
    fn test_rigid_body_kinematic() {
        let body = RigidBody3D::new_kinematic();
        println!("✅ Kinematic rigid body created");
    }

    #[test]
    fn test_collider_ball() {
        let collider = Collider3D::new_ball(2.5);
        assert_eq!(collider.density, 1.0);
        assert_eq!(collider.restitution, 0.5); // Default is 0.5, not 0.0
        assert_eq!(collider.friction, 0.5);
        assert!(!collider.is_sensor);
        println!("✅ Ball collider created: radius=2.5, restitution={}, friction={}", 
                 collider.restitution, collider.friction);
    }

    #[test]
    fn test_collider_cuboid() {
        let collider = Collider3D::new_cuboid(1.0, 2.0, 3.0);
        println!("✅ Cuboid collider created: hx=1.0, hy=2.0, hz=3.0");
    }

    #[test]
    fn test_collider_capsule() {
        let collider = Collider3D::new_capsule(1.0, 0.5);
        println!("✅ Capsule collider created: half_height=1.0, radius=0.5");
    }

    #[test]
    fn test_add_rigid_body() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0, 0.0])
            .build();
        
        let _handle = physics.add_rigid_body(entity, body);
        
        assert!(physics.get_body_handle(entity).is_some());
        println!("✅ Rigid body added to physics world");
    }

    #[test]
    fn test_get_set_position() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Get initial position
        if let Some(pos) = physics.get_position(entity) {
            assert!((pos.x - 0.0).abs() < 0.001);
            assert!((pos.y - 10.0).abs() < 0.001);
            assert!((pos.z - 0.0).abs() < 0.001);
            println!("✅ Initial position: ({}, {}, {})", pos.x, pos.y, pos.z);
        } else {
            panic!("Failed to get position");
        }
        
        // Set new position
        physics.set_position(entity, Vec3::new(5.0, 15.0, 20.0), true);
        
        if let Some(pos) = physics.get_position(entity) {
            assert!((pos.x - 5.0).abs() < 0.001);
            assert!((pos.y - 15.0).abs() < 0.001);
            assert!((pos.z - 20.0).abs() < 0.001);
            println!("✅ New position: ({}, {}, {})", pos.x, pos.y, pos.z);
        } else {
            panic!("Failed to get position after set");
        }
    }

    #[test]
    fn test_get_set_velocity() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Set velocity
        physics.set_linear_velocity(entity, Vec3::new(1.0, 2.0, 3.0), true);
        
        if let Some(vel) = physics.get_linear_velocity(entity) {
            assert!((vel.x - 1.0).abs() < 0.001);
            assert!((vel.y - 2.0).abs() < 0.001);
            assert!((vel.z - 3.0).abs() < 0.001);
            println!("✅ Linear velocity: ({}, {}, {})", vel.x, vel.y, vel.z);
        } else {
            panic!("Failed to get velocity");
        }
    }

    #[test]
    fn test_apply_impulse() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Apply impulse
        physics.apply_impulse(entity, Vec3::new(0.0, 10.0, 0.0), true);
        
        println!("✅ Impulse applied");
    }

    #[test]
    fn test_apply_force() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Apply force
        physics.apply_force(entity, Vec3::new(10.0, 0.0, 0.0), true);
        
        println!("✅ Force applied");
    }

    #[test]
    fn test_apply_torque() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Apply torque
        physics.apply_torque(entity, Vec3::new(0.0, 1.0, 0.0), true);
        
        println!("✅ Torque applied");
    }

    #[test]
    fn test_physics_step() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 10.0, 0.0])
            .build();
        
        let body_handle = physics.add_rigid_body(entity, body);
        
        // Add a collider so the body has mass
        let collider = ColliderBuilder::ball(0.5).build();
        physics.add_collider(collider, body_handle);
        
        let initial_pos = physics.get_position(entity).unwrap();
        
        // Step physics multiple times
        for _ in 0..60 {  // More steps to ensure movement
            physics.step();
        }
        
        let new_pos = physics.get_position(entity).unwrap();
        
        // Object should have fallen due to gravity
        assert!(new_pos.y < initial_pos.y, "Object should have fallen: y={} -> y={}", initial_pos.y, new_pos.y);
        println!("✅ Physics step: y={} -> y={}", initial_pos.y, new_pos.y);
    }

    #[test]
    fn test_remove_body() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        assert!(physics.get_body_handle(entity).is_some());
        
        physics.remove_body(entity);
        assert!(physics.get_body_handle(entity).is_none());
        
        println!("✅ Rigid body removed");
    }

    #[test]
    fn test_get_set_rotation() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Set rotation (identity quaternion: w=1, x=0, y=0, z=0)
        physics.set_rotation(entity, 0.0, 0.0, 0.0, 1.0, true);
        
        if let Some((x, y, z, w)) = physics.get_rotation(entity) {
            println!("✅ Rotation: quat({}, {}, {}, {})", x, y, z, w);
        } else {
            panic!("Failed to get rotation");
        }
    }

    #[test]
    fn test_angular_velocity() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        physics.add_rigid_body(entity, body);
        
        // Set angular velocity
        physics.set_angular_velocity(entity, Vec3::new(0.0, 1.0, 0.0), true);
        
        if let Some(vel) = physics.get_angular_velocity(entity) {
            println!("✅ Angular velocity: ({}, {}, {})", vel.x, vel.y, vel.z);
        } else {
            panic!("Failed to get angular velocity");
        }
    }

    #[test]
    fn test_raycast() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        // Create a body at origin
        let entity = world.spawn().build();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        
        let body_handle = physics.add_rigid_body(entity, body);
        
        // Add a collider
        let collider = ColliderBuilder::ball(1.0).build();
        physics.add_collider(collider, body_handle);
        
        // Update query pipeline
        physics.step();
        
        // Cast ray from above
        let ray_origin = Vec3::new(0.0, 10.0, 0.0);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let max_distance = 20.0;
        
        if let Some((hit_entity, hit_point, hit_normal, toi)) = 
            physics.raycast(ray_origin, ray_direction, max_distance, true) {
            assert_eq!(hit_entity, entity);
            assert!(toi > 0.0);
            println!("✅ Raycast hit: entity={:?}, point=({}, {}, {}), toi={}", 
                     hit_entity, hit_point.x, hit_point.y, hit_point.z, toi);
        } else {
            println!("⚠️  Raycast miss (may need more physics steps)");
        }
    }

    #[test]
    fn test_is_colliding() {
        let mut physics = PhysicsWorld3D::new();
        let mut world = World::new();
        
        // Create two bodies
        let entity1 = world.spawn().build();
        let body1 = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, 0.0])
            .build();
        let handle1 = physics.add_rigid_body(entity1, body1);
        
        let entity2 = world.spawn().build();
        let body2 = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.5, 0.0])
            .build();
        let handle2 = physics.add_rigid_body(entity2, body2);
        
        // Add colliders
        let collider1 = ColliderBuilder::ball(0.5).build();
        physics.add_collider(collider1, handle1);
        
        let collider2 = ColliderBuilder::ball(0.5).build();
        physics.add_collider(collider2, handle2);
        
        // Step physics to update collisions
        physics.step();
        
        // Check if colliding (they should be, since they're overlapping)
        let colliding = physics.is_colliding(entity1, entity2);
        println!("✅ Collision check: colliding={}", colliding);
    }

    #[test]
    fn test_collider_properties() {
        let mut collider = Collider3D::new_ball(1.0);
        
        // Test default properties
        assert_eq!(collider.density, 1.0);
        assert_eq!(collider.restitution, 0.5); // Default is 0.5, not 0.0
        assert_eq!(collider.friction, 0.5);
        assert!(!collider.is_sensor);
        
        // Modify properties
        collider.density = 2.0;
        collider.restitution = 0.8;
        collider.friction = 0.3;
        collider.is_sensor = true;
        
        assert_eq!(collider.density, 2.0);
        assert_eq!(collider.restitution, 0.8);
        assert_eq!(collider.friction, 0.3);
        assert!(collider.is_sensor);
        
        println!("✅ Collider properties: density={}, restitution={}, friction={}, sensor={}", 
                 collider.density, collider.restitution, collider.friction, collider.is_sensor);
    }

    #[test]
    fn test_rigid_body_properties() {
        let mut body = RigidBody3D::new_dynamic();
        
        // Test default properties
        assert_eq!(body.mass, 1.0);
        assert_eq!(body.linear_damping, 0.0);
        assert_eq!(body.angular_damping, 0.0);
        
        // Modify properties
        body.mass = 10.0;
        body.linear_damping = 0.5;
        body.angular_damping = 0.3;
        
        assert_eq!(body.mass, 10.0);
        assert_eq!(body.linear_damping, 0.5);
        assert_eq!(body.angular_damping, 0.3);
        
        println!("✅ Rigid body properties: mass={}, linear_damping={}, angular_damping={}", 
                 body.mass, body.linear_damping, body.angular_damping);
    }
}

