use windjammer_runtime::game::{World, EntityId, Transform, Sprite, Color, Vec3, Renderer2D, Input, Key};


#[derive(Component)]
struct Player {
    lives: i32,
}

#[derive(Component)]
struct Asteroid {
    size: i32,
}

#[derive(Component)]
struct Bullet {
    lifetime: f32,
}

#[derive(Component)]
struct Velocity {
    linear: Vec3,
    angular: f32,
}

#[inline]
fn player_speed() -> f64 {
    200.0
}

#[inline]
fn player_turn_speed() -> f64 {
    3.0
}

#[inline]
fn bullet_speed() -> f64 {
    400.0
}

#[inline]
fn bullet_lifetime() -> f32 {
    2.0
}

#[inline]
fn asteroid_spawn_count() -> i32 {
    5
}

#[inline]
fn init_game(world: &&mut World) -> EntityId {
    let player = world.create_entity();
    world.add_component(player, Player { lives: 3 });
    world.add_component(player, Transform::new(Vec3::new(400.0, 300.0, 0.0)));
    world.add_component(player, Velocity { linear: Vec3::zero(), angular: 0.0 });
    world.add_component(player, Sprite::new_with_color("player", 32.0, 32.0, Color::new(0, 255, 0, 255)));
    for i in 0..asteroid_spawn_count() {
        let angle = i as f64 * 6.28318 / asteroid_spawn_count() as f64;
        let radius = 200.0;
        let x = 400.0 + radius * angle.cos();
        let y = 300.0 + radius * angle.sin();
        let asteroid = world.create_entity();
        world.add_component(asteroid, Asteroid { size: 3 });
        world.add_component(asteroid, Transform::new(Vec3::new(x, y, 0.0)));
        world.add_component(asteroid, Velocity { linear: Vec3::new((angle + 1.57).cos() * 50.0, (angle + 1.57).sin() * 50.0, 0.0), angular: 0.5 });
        world.add_component(asteroid, Sprite::new_with_color("asteroid", 48.0, 48.0, Color::new(128, 128, 128, 255)));
    }
    player
}

#[inline]
fn update_movement(world: &&mut World, dt: f32) {
    let transforms = world.query_mut::<Transform>();
    let velocities = world.query::<Velocity>();
    for (entity_id, velocity) in velocities {
        match world.get_component_mut::<Transform>(entity_id) {
            Some(transform) => {
                transform.position.x = transform.position.x + velocity.linear.x * dt as f64;
                transform.position.y = transform.position.y + velocity.linear.y * dt as f64;
                transform.rotation.y = transform.rotation.y + velocity.angular as f64 * dt as f64;
                if transform.position.x < 0.0 {
                    transform.position.x = 800.0;
                }
                if transform.position.x > 800.0 {
                    transform.position.x = 0.0;
                }
                if transform.position.y < 0.0 {
                    transform.position.y = 600.0;
                }
                if transform.position.y > 600.0 {
                    transform.position.y = 0.0;
                }
            },
        }
    }
}

#[inline]
fn update_player_input(world: &&mut World, input: &Input, dt: f32) {
    let players = world.query::<Player>();
    for (entity_id, _player) in players {
        match world.get_component_mut::<Velocity>(entity_id) {
            Some(velocity) => match world.get_component::<Transform>(entity_id) {
                Some(transform) => {
                    let mut turn = 0.0;
                    if input.is_key_down(Key::Left) {
                        turn = -player_turn_speed();
                    }
                    if input.is_key_down(Key::Right) {
                        turn = player_turn_speed();
                    }
                    velocity.angular = turn;
                    if input.is_key_down(Key::Up) {
                        let angle = transform.rotation.y;
                        velocity.linear.x = angle.cos() * player_speed();
                        velocity.linear.y = angle.sin() * player_speed();
                    } else {
                        velocity.linear.x = velocity.linear.x * 0.98;
                        velocity.linear.y = velocity.linear.y * 0.98;
                    }
                },
            },
        }
    }
}

#[inline]
fn update_bullets(world: &mut &mut World, dt: f32) {
    let bullets_query = world.query::<Bullet>();
    let mut to_remove = Vec::new();
    for (entity_id, bullet) in bullets_query {
        match world.get_component_mut::<Bullet>(entity_id) {
            Some(bullet_mut) => {
                bullet_mut.lifetime = bullet_mut.lifetime - dt;
                if bullet_mut.lifetime <= 0.0 {
                    to_remove.push(entity_id)
                }
            },
        }
    }
    for entity in to_remove {
        world.remove_entity(entity);
    }
}

#[inline]
fn shoot_bullet(world: &&mut World, player_id: &EntityId) {
    match world.get_component::<Transform>(player_id) {
        Some(transform) => {
            let bullet = world.create_entity();
            let angle = transform.rotation.y;
            let offset_distance = 20.0;
            let start_x = transform.position.x + angle.cos() * offset_distance;
            let start_y = transform.position.y + angle.sin() * offset_distance;
            world.add_component(bullet, Bullet { lifetime: bullet_lifetime() });
            world.add_component(bullet, Transform::new(Vec3::new(start_x, start_y, 0.0)));
            world.add_component(bullet, Velocity { linear: Vec3::new(angle.cos() * bullet_speed(), angle.sin() * bullet_speed(), 0.0), angular: 0.0 });
            world.add_component(bullet, Sprite::new_with_color("bullet", 4.0, 4.0, Color::new(255, 255, 255, 255)));
        },
    }
}

#[inline]
fn render_game(world: &World, renderer: &mut &mut Renderer2D) {
    renderer.clear(Color::new(0, 0, 0, 255));
    let transforms = world.query::<Transform>();
    let sprites = world.query::<Sprite>();
    for (entity_id, transform) in transforms {
        match world.get_component::<Sprite>(entity_id) {
            Some(sprite) => {
                renderer.draw_sprite(sprite, transform);
            },
        }
    }
    renderer.present()
}

fn main() {
    println!("=== Asteroids Game ===");
    let mut world = World::new();
    let player = init_game(&world);
    let mut renderer = Renderer2D::new(800, 600);
    let mut input = Input::new();
    let mut last_time = 0.0;
    let dt = 0.016;
    for frame in 0..10 {
        input.update();
        if frame == 3 {
            input.press_key(Key::Space)
        }
        if input.is_key_pressed(Key::Space) {
            shoot_bullet(&world, &player);
            println!("Player shot!")
        }
        update_player_input(&world, &input, dt);
        update_movement(&world, dt);
        update_bullets(&world, dt);
        render_game(&world, &renderer);
        let player_count = world.query::<Player>().len();
        let asteroid_count = world.query::<Asteroid>().len();
        let bullet_count = world.query::<Bullet>().len();
        println!(format!("Frame {}: Player={}, Asteroids={}, Bullets={}", frame, player_count, asteroid_count, bullet_count));
    }
    println!("
✨ Asteroids game loop complete!");
    println!("Demonstrated:");
    println!("  • Full ECS game architecture");
    println!("  • Input handling (keyboard)");
    println!("  • Movement system with physics");
    println!("  • Bullet spawning and lifetime");
    println!("  • Sprite rendering");
    println!("  • Screen wrapping")
}

