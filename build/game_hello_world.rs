use windjammer_runtime::game;


fn main() {
    let mut world = game::World::new();
    let player = game::create_entity(&world);
    game::add_transform(&world, player, game::Vec3::new(0.0, 0.0, 0.0));
    game::add_velocity(&world, player, game::Vec3::new(1.0, 0.0, 0.0));
    let enemy = game::create_entity(&world);
    game::add_transform(&world, enemy, game::Vec3::new(10.0, 0.0, 0.0));
    let transforms = world.query::<game::Transform>();
    println!("Found {} entities with transforms", transforms.len());
    match world.get_component::<game::Transform>(player) {
        Some(transform) => {
            println!("Player position: ({}, {}, {})", transform.position::x, transform.position::y, transform.position::z);
        },
    }
    println!("Game world initialized successfully!")
}

