use windjammer_runtime::game;


fn main() {
    println!("=== Windjammer Game Engine Demo ===");
    println!("Creating a simple platformer...");
    let mut world = game::World::new();
    let player = game::create_entity(&world);
    game::add_transform(&world, player, game::Vec3::new(0.0, 0.0, 0.0));
    game::add_velocity(&world, player, game::Vec3::new(0.0, 0.0, 0.0));
    println!("✓ Created player entity");
    let platform1 = game::create_entity(&world);
    game::add_transform(&world, platform1, game::Vec3::new(0.0, -2.0, 0.0));
    game::add_mesh(&world, platform1, game::Mesh::cube(4.0));
    let platform2 = game::create_entity(&world);
    game::add_transform(&world, platform2, game::Vec3::new(5.0, -1.0, 0.0));
    game::add_mesh(&world, platform2, game::Mesh::cube(3.0));
    println!("✓ Created {} platforms", 2);
    let enemy1 = game::create_entity(&world);
    game::add_transform(&world, enemy1, game::Vec3::new(3.0, 0.0, 0.0));
    game::add_velocity(&world, enemy1, game::Vec3::new(-1.0, 0.0, 0.0));
    let enemy2 = game::create_entity(&world);
    game::add_transform(&world, enemy2, game::Vec3::new(-3.0, 0.0, 0.0));
    game::add_velocity(&world, enemy2, game::Vec3::new(1.0, 0.0, 0.0));
    println!("✓ Created {} enemies", 2);
    let transforms = world.query::<game::Transform>();
    println!("
=== World State ===");
    println!("Total entities with transforms: {}", transforms.len());
    match world.get_component::<game::Transform>(player) {
        Some(transform) => {
            println!("Player position: ({:.1}, {:.1}, {:.1})", transform.position::x, transform.position::y, transform.position::z);
        },
    }
    println!("
=== Simulating Game Loop ===");
    let delta_time = 0.016;
    for frame in 0..5 {
        println!("
Frame {}:", frame);
        let entities_with_velocity = world.query::<game::Velocity>();
        for (entity_id, velocity) in entities_with_velocity {
            match world.get_component_mut::<game::Transform>(entity_id) {
                Some(transform) => {
                    transform.position::x = transform.position::x + velocity.linear::x * delta_time;
                    transform.position::y = transform.position::y + velocity.linear::y * delta_time;
                    transform.position::z = transform.position::z + velocity.linear::z * delta_time;
                    println!("  Entity {:?} moved to ({:.2}, {:.2}, {:.2})", entity_id, transform.position::x, transform.position::y, transform.position::z);
                },
            }
        }
    }
    println!("
=== Final State ===");
    match world.get_component::<game::Transform>(player) {
        Some(transform) => {
            println!("Player final position: ({:.2}, {:.2}, {:.2})", transform.position::x, transform.position::y, transform.position::z);
        },
    }
    println!("
✓ Game simulation complete!");
    println!("Note: Add wgpu rendering to see graphics on screen")
}

