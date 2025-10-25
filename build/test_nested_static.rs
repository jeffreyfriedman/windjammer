use windjammer_runtime::game;


fn main() {
    let mut world = game::World::new();
    let entity = world.create_entity();
    for i in 0..3 {
        world.add_component(entity, game::Transform::new(game::Vec3::new(1.0, 2.0, 3.0)));
    }
}

