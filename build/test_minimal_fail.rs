use windjammer_runtime::game;


fn main() {
    let mut world = game::World::new();
    let entity = world.create_entity();
    for i in 0..3 {
        match world.get_component::<game::Transform>(entity) {
            Some(t) => {
                println!("found");
            },
        }
    }
}

