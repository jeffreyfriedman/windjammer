use windjammer_runtime::game;


fn main() {
    println!("=== Windjammer ECS Demo ===");
    let mut world = game::World::new();
    println!("✓ Created game world");
    let entity1 = world.create_entity();
    let entity2 = world.create_entity();
    let entity3 = world.create_entity();
    println!("✓ Created {} entities", 3);
    world.add_component(entity1, game::Transform::new(game::Vec3::new(0.0, 0.0, 0.0)));
    world.add_component(entity2, game::Transform::new(game::Vec3::new(5.0, 0.0, 0.0)));
    world.add_component(entity3, game::Transform::new(game::Vec3::new(-5.0, 0.0, 0.0)));
    println!("✓ Added Transform components");
    let transforms = world.query::<game::Transform>();
    println!("
Found {} entities with transforms", transforms.len());
    let v1 = game::Vec3::new(1.0, 2.0, 3.0);
    let v2 = game::Vec3::new(4.0, 5.0, 6.0);
    let dot = v1.dot(&v2);
    println!("
Math test:");
    println!("v1 = ({}, {}, {})", v1.x, v1.y, v1.z);
    println!("v2 = ({}, {}, {})", v2.x, v2.y, v2.z);
    println!("v1 · v2 = {}", dot);
    println!("
✓ ECS demo complete!")
}

