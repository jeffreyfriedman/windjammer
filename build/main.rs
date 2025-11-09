use windjammer_game_framework::renderer3d::{Renderer3D, Camera3D};
use windjammer_game_framework::renderer::Color;
use windjammer_game_framework::input::{Input, Key};
use windjammer_game_framework::math::{Vec3, Mat4};

struct ShooterGame {
    player_pos: Vec3,
    player_velocity: Vec3,
    player_yaw: f32,
    player_pitch: f32,
    player_health: i64,
    player_on_ground: bool,
    move_speed: f32,
    sprint_speed: f32,
    jump_velocity: f32,
    gravity: f32,
    mouse_sensitivity: f32,
    weapon: i64,
    ammo: i64,
    score: i64,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    walls: Vec<Wall>,
    floor_y: f32,
    paused: bool,
}

impl Default for ShooterGame {
    fn default() -> Self {
        ShooterGame {
            player_pos: Vec::new(),
            player_velocity: Vec::new(),
            player_yaw: Default::default(),
            player_pitch: Default::default(),
            player_health: 0,
            player_on_ground: false,
            move_speed: Default::default(),
            sprint_speed: Default::default(),
            jump_velocity: Default::default(),
            gravity: Default::default(),
            mouse_sensitivity: Default::default(),
            weapon: 0,
            ammo: 0,
            score: 0,
            enemies: Vec::new(),
            bullets: Vec::new(),
            walls: Vec::new(),
            floor_y: Default::default(),
            paused: false,
        }
    }
}

struct Enemy {
    pos: Vec3,
    velocity: Vec3,
    health: i64,
    state: i64,
    color: Color,
}

struct Bullet {
    pos: Vec3,
    velocity: Vec3,
    damage: i64,
    lifetime: f32,
}

struct Wall {
    pos: Vec3,
    size: Vec3,
    color: Color,
}

impl ShooterGame {
#[inline]
fn create_level(mut self) {
        self.walls.push(Wall { pos: Vec3::new(0.0, 2.0, -20.0), size: Vec3::new(40.0, 4.0, 1.0), color: Color::rgb(0.5, 0.5, 0.5) });
        self.walls.push(Wall { pos: Vec3::new(0.0, 2.0, 20.0), size: Vec3::new(40.0, 4.0, 1.0), color: Color::rgb(0.5, 0.5, 0.5) });
        self.walls.push(Wall { pos: Vec3::new(-20.0, 2.0, 0.0), size: Vec3::new(1.0, 4.0, 40.0), color: Color::rgb(0.5, 0.5, 0.5) });
        self.walls.push(Wall { pos: Vec3::new(20.0, 2.0, 0.0), size: Vec3::new(1.0, 4.0, 40.0), color: Color::rgb(0.5, 0.5, 0.5) });
        self.walls.push(Wall { pos: Vec3::new(-10.0, 2.0, 0.0), size: Vec3::new(1.0, 4.0, 15.0), color: Color::rgb(0.4, 0.4, 0.4) });
        self.walls.push(Wall { pos: Vec3::new(10.0, 2.0, 5.0), size: Vec3::new(1.0, 4.0, 20.0), color: Color::rgb(0.4, 0.4, 0.4) });
        self.walls.push(Wall { pos: Vec3::new(0.0, 2.0, -10.0), size: Vec3::new(15.0, 4.0, 1.0), color: Color::rgb(0.4, 0.4, 0.4) });
        self.walls.push(Wall { pos: Vec3::new(5.0, 2.0, 10.0), size: Vec3::new(20.0, 4.0, 1.0), color: Color::rgb(0.4, 0.4, 0.4) })
}
#[inline]
fn spawn_enemies(mut self) {
        self.enemies.push(Enemy { pos: Vec3::new(10.0, 1.0, 10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, color: Color::rgb(1.0, 0.0, 0.0) });
        self.enemies.push(Enemy { pos: Vec3::new(-10.0, 1.0, 10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, color: Color::rgb(1.0, 0.2, 0.0) });
        self.enemies.push(Enemy { pos: Vec3::new(10.0, 1.0, -10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, color: Color::rgb(0.8, 0.0, 0.0) });
        self.enemies.push(Enemy { pos: Vec3::new(-10.0, 1.0, -10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, color: Color::rgb(1.0, 0.1, 0.1) });
        self.enemies.push(Enemy { pos: Vec3::new(0.0, 1.0, 15.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, color: Color::rgb(0.9, 0.0, 0.0) })
}
}

fn init(game: &mut ShooterGame) {
    game.player_pos = Vec3::new(0.0, 2.0, 0.0);
    game.player_velocity = Vec3::new(0.0, 0.0, 0.0);
    game.player_yaw = 0.0;
    game.player_pitch = 0.0;
    game.player_health = 100;
    game.player_on_ground = true;
    game.move_speed = 5.0;
    game.sprint_speed = 10.0;
    game.jump_velocity = 8.0;
    game.gravity = -20.0;
    game.mouse_sensitivity = 0.1;
    game.weapon = 0;
    game.ammo = 100;
    game.score = 0;
    game.floor_y = 0.0;
    game.paused = false;
    game.create_level();
    game.spawn_enemies();
    println!("=== GREYBOX SHOOTER ===");
    println!("WASD - Move");
    println!("Space - Jump");
    println!("Shift - Sprint");
    println!("Mouse - Look");
    println!("Left Click - Shoot");
    println!("1/2/3 - Switch weapon");
    println!("ESC - Pause");
    println!("");
    println!("Kill all enemies to win!")
}

fn handle_input(game: &mut ShooterGame, input: &Input) {
    if game.paused {
        if input.pressed(Key::Escape) {
            game.paused = false;
            println!("Game resumed!")
        }
        return;
    }
    if input.pressed(Key::Escape) {
        game.paused = true;
        println!("Game paused! Press ESC to resume");
        return;
    }
    if input.pressed(Key::Num1) {
        game.weapon = 0;
        println!("Switched to Pistol")
    }
    if input.pressed(Key::Num2) {
        game.weapon = 1;
        println!("Switched to Shotgun")
    }
    if input.pressed(Key::Num3) {
        game.weapon = 2;
        println!("Switched to Rocket Launcher")
    }
}

#[inline]
fn update(game: &mut ShooterGame, delta: f32, input: &Input) {
    if game.paused {
        return;
    }
    update_player_movement(&game, delta, &input);
    update_enemies(&game, delta);
    update_bullets(&game, delta);
    if game.enemies.len() == 0 {
        println!(format!("{}{}", "YOU WIN! Score: ", game.score.to_string()));
        println!("Press ESC to exit")
    }
}

fn update_player_movement(game: &ShooterGame, delta: f32, input: &Input) {
    let yaw_rad = game.player_yaw * 3.14159 / 180.0;
    let forward_x = yaw_rad.sin();
    let forward_z = yaw_rad.cos();
    let right_x = (yaw_rad + 1.5708).sin();
    let right_z = (yaw_rad + 1.5708).cos();
    let mut move_x = 0.0;
    let mut move_z = 0.0;
    if input.held(Key::W) {
        move_x += forward_x;
        move_z += forward_z;
    }
    if input.held(Key::S) {
        move_x -= forward_x;
        move_z -= forward_z;
    }
    if input.held(Key::A) {
        move_x -= right_x;
        move_z -= right_z;
    }
    if input.held(Key::D) {
        move_x += right_x;
        move_z += right_z;
    }
    let move_length = (move_x * move_x + move_z * move_z).sqrt();
    if move_length > 0.0 {
        move_x /= move_length;
        move_z /= move_length;
    }
    let speed = {
        if input.held(Key::Shift) {
            game.sprint_speed
        } else {
            game.move_speed
        }
    };
    game.player_velocity.x = move_x * speed;
    game.player_velocity.z = move_z * speed;
    if input.pressed(Key::Space) && game.player_on_ground {
        game.player_velocity.y = game.jump_velocity;
        game.player_on_ground = false;
    }
    if !game.player_on_ground {
        game.player_velocity.y = game.player_velocity.y + game.gravity * delta;
    }
    let new_x = game.player_pos.x + game.player_velocity.x * delta;
    let new_y = game.player_pos.y + game.player_velocity.y * delta;
    let new_z = game.player_pos.z + game.player_velocity.z * delta;
    let mut can_move_x = true;
    let mut can_move_z = true;
    for wall in game.walls {
        if check_collision(new_x.clone(), game.player_pos.z.clone(), &wall) {
            can_move_x = false;
        }
        if check_collision(game.player_pos.x, new_z, &wall) {
            can_move_z = false;
        }
    }
    if can_move_x {
        game.player_pos.x = new_x;
    }
    if can_move_z {
        game.player_pos.z = new_z;
    }
    if new_y <= 2.0 {
        game.player_pos.y = 2.0;
        game.player_velocity.y = 0.0;
        game.player_on_ground = true;
    } else {
        game.player_pos.y = new_y;
    }
}

#[inline]
fn check_collision(x: f32, z: f32, wall: &Wall) -> bool {
    let half_width = wall.size.x / 2.0;
    let half_depth = wall.size.z / 2.0;
    let player_radius = 0.5;
    let dx = (x - wall.pos.x).abs();
    let dz = (z - wall.pos.z).abs();
    return dx < half_width + player_radius && dz < half_depth + player_radius;
}

#[inline]
fn update_enemies(game: &ShooterGame, delta: f32) {
    let mut i = 0;
    while i < game.enemies.len() {
        let enemy = game.enemies[i];
        if enemy.state == 3 {
            game.enemies.remove(i);
            continue;
        }
        if enemy.state == 1 {
            let dx = game.player_pos.x - enemy.pos.x;
            let dz = game.player_pos.z - enemy.pos.z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist > 0.1 {
                let speed = 2.0;
                enemy.velocity.x = dx / dist * speed;
                enemy.velocity.z = dz / dist * speed;
                enemy.pos.x = enemy.pos.x + enemy.velocity.x * delta;
                enemy.pos.z = enemy.pos.z + enemy.velocity.z * delta;
            }
            if dist < 2.0 {
                enemy.state = 2;
            }
        }
        if enemy.state == 2 {
            let dx = game.player_pos.x - enemy.pos.x;
            let dz = game.player_pos.z - enemy.pos.z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist > 3.0 {
                enemy.state = 1;
            }
        }
        i += 1;
    }
}

#[inline]
fn update_bullets(game: &ShooterGame, delta: f32) {
    let mut i = 0;
    while i < game.bullets.len() {
        let bullet = game.bullets[i];
        bullet.pos.x = bullet.pos.x + bullet.velocity.x * delta;
        bullet.pos.y = bullet.pos.y + bullet.velocity.y * delta;
        bullet.pos.z = bullet.pos.z + bullet.velocity.z * delta;
        bullet.lifetime = bullet.lifetime - delta;
        if bullet.lifetime <= 0.0 {
            game.bullets.remove(i);
            continue;
        }
        let mut hit_wall = false;
        for wall in game.walls {
            if check_collision(bullet.pos.x, bullet.pos.z, &wall) {
                hit_wall = true;
                break;
            }
        }
        if hit_wall {
            game.bullets.remove(i);
            continue;
        }
        let mut hit_enemy = false;
        let mut j = 0;
        while j < game.enemies.len() {
            let enemy = game.enemies[j];
            let dx = bullet.pos.x - enemy.pos.x;
            let dy = bullet.pos.y - enemy.pos.y;
            let dz = bullet.pos.z - enemy.pos.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < 1.0 {
                enemy.health = enemy.health - bullet.damage;
                if enemy.health <= 0 {
                    enemy.state = 3;
                    game.score = game.score + 100;
                    println!(format!("{}{}", "Enemy killed! Score: ", game.score.to_string()))
                }
                hit_enemy = true;
                break;
            }
            j += 1;
        }
        if hit_enemy {
            game.bullets.remove(i);
            continue;
        }
        i += 1;
    }
}

fn render(game: &mut ShooterGame, renderer: &mut Renderer3D, camera: &Camera3D) {
    camera.position = game.player_pos;
    camera.yaw = game.player_yaw;
    camera.pitch = game.player_pitch;
    renderer.clear(Color::rgb(0.1, 0.1, 0.15));
    renderer.draw_plane(Vec3::new(0.0, game.floor_y, 0.0), Vec3::new(100.0, 0.0, 100.0), Color::rgb(0.2, 0.2, 0.2));
    for wall in game.walls {
        renderer.draw_cube(wall.pos, wall.size, wall.color);
    }
    for enemy in game.enemies {
        if enemy.state != 3 {
            renderer.draw_cube(enemy.pos, Vec3::new(1.0, 2.0, 1.0), enemy.color)
        }
    }
    for bullet in game.bullets {
        renderer.draw_cube(bullet.pos, Vec3::new(0.2, 0.2, 0.2), Color::rgb(1.0, 1.0, 0.0));
    }
}

#[inline]
fn cleanup(game: &mut ShooterGame) {
    println!(format!("{}{}", "Final Score: ", game.score.to_string()));
    println!("Thanks for playing!")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windjammer_game_framework::*;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    // Create event loop and window
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Windjammer Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)?;

    // Initialize game state
    let mut game = ShooterGame::default();

    // Call init function
    init(&mut game);

    // Initialize renderer
    let window_ref: &'static winit::window::Window = unsafe { std::mem::transmute(&window) };
    let mut renderer = pollster::block_on(renderer3d::Renderer3D::new(window_ref))?;
    let mut camera = renderer3d::Camera3D::new();

    // Initialize input
    let mut input = input::Input::new();

    // Game loop
    let mut last_time = std::time::Instant::now();

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    cleanup(&mut game);
                    elwt.exit();
                }
                WindowEvent::RedrawRequested => {
                    // Calculate delta time
                    let now = std::time::Instant::now();
                    let delta = (now - last_time).as_secs_f32();
                    last_time = now;

                    // Update game logic
                    update(&mut game, delta, &input);

                    // Render
                    renderer.set_camera(&camera);
                    render(&mut game, &mut renderer, &mut camera);
                    renderer.present();

                    // Clear input frame state
                    input.clear_frame_state();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    input.update_from_winit(&event);
                    handle_input(&mut game, &input);
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}


