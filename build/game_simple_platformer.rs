use windjammer_runtime::game;


fn main() {
    println!("=== Windjammer Game Engine Demo ===");
    println!("Creating a simple platformer...");
    let mut world = game::World::new();
    let player = world.create_entity();
    world.add_component(player, game::Transform::new(game::Vec3::new(0.0, 0.0, 0.0)));
    world.add_component(player, game::Velocity::new(game::Vec3::new(0.0, 0.0, 0.0)));
    println!("✓ Created player entity");
    let platform1 = world.create_entity();
    world.add_component(platform1, game::Transform::new(game::Vec3::new(0.0, -2.0, 0.0)));
    world.add_component(platform1, game::Mesh::cube(4.0));
    let platform2 = world.create_entity();
    world.add_component(platform2, game::Transform::new(game::Vec3::new(5.0, -1.0, 0.0)));
    world.add_component(platform2, game::Mesh::cube(3.0));
    println!("✓ Created {} platforms", 2);
    let enemy1 = world.create_entity();
    world.add_component(enemy1, game::Transform::new(game::Vec3::new(3.0, 0.0, 0.0)));
    world.add_component(enemy1, game::Velocity::new(game::Vec3::new(-1.0, 0.0, 0.0)));
    let enemy2 = world.create_entity();
    world.add_component(enemy2, game::Transform::new(game::Vec3::new(-3.0, 0.0, 0.0)));
    world.add_component(enemy2, game::Velocity::new(game::Vec3::new(1.0, 0.0, 0.0)));
    println!("✓ Created {} enemies", 2);
    let transforms = world.query::<game::Transform>();
    println!("
=== World State ===");
    println!("Total entities with transforms: {}", transforms.len());
    match world.get_component::<game::Transform>(player) {
        Some(transform) => {
            println!("Player position: ({}, {}, {})", transform.position.x, transform.position.y, transform.position.z);
        },
        _ => {
        },
    }
    println!("
=== Querying Entities ===");
    let velocities = world.query::<game::Velocity>();
    println!("Found {} entities with velocity", velocities.len());
    for (entity_id, velocity) in velocities {
        println!("Entity {:?} has velocity: ({}, {}, {})", entity_id, velocity.linear.x, velocity.linear.y, velocity.linear.z);
    }
    println!("
✓ Game simulation complete!")
}

