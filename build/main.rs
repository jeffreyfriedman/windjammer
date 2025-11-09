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
    powerups: Vec<PowerUp>,
    floor_y: f32,
    paused: bool,
    speed_boost_timer: f32,
}

// Game trait implementation for ShooterGame
// TODO: Implement Game trait

struct Enemy {
    pos: Vec3,
    velocity: Vec3,
    health: i64,
    state: i64,
    enemy_type: i64,
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

struct PowerUp {
    pos: Vec3,
    powerup_type: i64,
    active: bool,
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
        self.enemies.push(Enemy { pos: Vec3::new(10.0, 1.0, 10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 2, state: 1, enemy_type: 0, color: Color::rgb(0.6, 0.4, 0.2) });
        self.enemies.push(Enemy { pos: Vec3::new(-10.0, 1.0, 10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 2, state: 1, enemy_type: 0, color: Color::rgb(0.6, 0.4, 0.2) });
        self.enemies.push(Enemy { pos: Vec3::new(10.0, 1.0, -10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, enemy_type: 1, color: Color::rgb(1.0, 0.0, 0.0) });
        self.enemies.push(Enemy { pos: Vec3::new(-10.0, 1.0, -10.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 3, state: 1, enemy_type: 1, color: Color::rgb(1.0, 0.0, 0.0) });
        self.enemies.push(Enemy { pos: Vec3::new(0.0, 1.0, 15.0), velocity: Vec3::new(0.0, 0.0, 0.0), health: 5, state: 1, enemy_type: 2, color: Color::rgb(0.8, 0.0, 0.8) })
}
#[inline]
fn spawn_powerups(mut self) {
        self.powerups.push(PowerUp { pos: Vec3::new(5.0, 0.5, 5.0), powerup_type: 0, active: true, color: Color::rgb(0.0, 1.0, 0.0) });
        self.powerups.push(PowerUp { pos: Vec3::new(-5.0, 0.5, 5.0), powerup_type: 1, active: true, color: Color::rgb(1.0, 1.0, 0.0) });
        self.powerups.push(PowerUp { pos: Vec3::new(0.0, 0.5, -10.0), powerup_type: 2, active: true, color: Color::rgb(0.0, 1.0, 1.0) });
        self.powerups.push(PowerUp { pos: Vec3::new(8.0, 0.5, -8.0), powerup_type: 0, active: true, color: Color::rgb(0.0, 1.0, 0.0) })
}
fn update_player_movement(mut self, mut delta: f32, mut input: Input) {
        let yaw_rad = self.player_yaw * 3.14159 / 180.0;
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
        let mut speed = {
            if input.held(Key::Shift) {
                self.sprint_speed
            } else {
                self.move_speed
            }
        };
        if self.speed_boost_timer > 0.0 {
            speed *= 1.5;
        }
        self.player_velocity.x = move_x * speed;
        self.player_velocity.z = move_z * speed;
        if input.pressed(Key::Space) && self.player_on_ground {
            self.player_velocity.y = self.jump_velocity;
            self.player_on_ground = false;
        }
        if !self.player_on_ground {
            self.player_velocity.y = self.player_velocity.y + self.gravity * delta;
        }
        let new_x = self.player_pos.x + self.player_velocity.x * delta;
        let new_y = self.player_pos.y + self.player_velocity.y * delta;
        let new_z = self.player_pos.z + self.player_velocity.z * delta;
        let mut can_move_x = true;
        let mut can_move_z = true;
        for wall in self.walls {
            if check_collision(new_x, self.player_pos.z, wall) {
                can_move_x = false;
            }
            if check_collision(self.player_pos.x, new_z, wall) {
                can_move_z = false;
            }
        }
        if can_move_x {
            self.player_pos.x = new_x;
        }
        if can_move_z {
            self.player_pos.z = new_z;
        }
        if new_y <= 2.0 {
            self.player_pos.y = 2.0;
            self.player_velocity.y = 0.0;
            self.player_on_ground = true;
        } else {
            self.player_pos.y = new_y;
        }
}
#[inline]
fn collect_powerups(mut self) {
        let mut i = 0;
        while i < self.powerups.len() {
            let powerup = self.powerups[i];
            if !powerup.active {
                i += 1;
                continue;
            }
            let dx = self.player_pos.x - powerup.pos.x;
            let dz = self.player_pos.z - powerup.pos.z;
            let dist = (dx * dx + dz * dz).sqrt();
            if dist < 1.5 {
                if powerup.powerup_type == 0 {
                    self.player_health = self.player_health + 25;
                    if self.player_health > 100 {
                        self.player_health = 100;
                    }
                    println(format!("{}{}{}", "+ Health! (", self.player_health.to_string(), "/100)"))
                } else {
                    if powerup.powerup_type == 1 {
                        self.ammo = self.ammo + 10;
                        println(format!("{}{}{}", "+ Ammo! (", self.ammo.to_string(), ")"))
                    } else {
                        if powerup.powerup_type == 2 {
                            self.speed_boost_timer = 5.0;
                            println("+ Speed Boost! (5 seconds)")
                        }
                    }
                }
                powerup.active = false;
            }
            i += 1;
        }
}
#[inline]
fn update_enemies(mut self, mut delta: f32) {
        let mut i = 0;
        while i < self.enemies.len() {
            let enemy = self.enemies[i];
            if enemy.state == 3 {
                self.enemies.remove(i);
                continue;
            }
            if enemy.state == 1 {
                let dx = self.player_pos.x - enemy.pos.x;
                let dz = self.player_pos.z - enemy.pos.z;
                let dist = (dx * dx + dz * dz).sqrt();
                if dist > 0.1 {
                    let speed = {
                        if enemy.enemy_type == 0 {
                            1.5
                        } else {
                            if enemy.enemy_type == 1 {
                                2.0
                            } else {
                                3.0
                            }
                        }
                    };
                    enemy.velocity.x = dx / dist * speed;
                    enemy.velocity.z = dz / dist * speed;
                    enemy.pos.x = enemy.pos.x + enemy.velocity.x * delta;
                    enemy.pos.z = enemy.pos.z + enemy.velocity.z * delta;
                }
                let attack_range = {
                    if enemy.enemy_type == 0 {
                        1.5
                    } else {
                        if enemy.enemy_type == 1 {
                            2.0
                        } else {
                            2.5
                        }
                    }
                };
                if dist < attack_range {
                    enemy.state = 2;
                }
            }
            if enemy.state == 2 {
                let dx = self.player_pos.x - enemy.pos.x;
                let dz = self.player_pos.z - enemy.pos.z;
                let dist = (dx * dx + dz * dz).sqrt();
                if dist > 3.0 {
                    enemy.state = 1;
                }
            }
            i += 1;
        }
}
#[inline]
fn update_bullets(mut self, mut delta: f32) {
        let mut i = 0;
        while i < self.bullets.len() {
            let bullet = self.bullets[i];
            bullet.pos.x = bullet.pos.x + bullet.velocity.x * delta;
            bullet.pos.y = bullet.pos.y + bullet.velocity.y * delta;
            bullet.pos.z = bullet.pos.z + bullet.velocity.z * delta;
            bullet.lifetime = bullet.lifetime - delta;
            if bullet.lifetime <= 0.0 {
                self.bullets.remove(i);
                continue;
            }
            let mut hit_wall = false;
            for wall in self.walls {
                if check_collision(bullet.pos.x, bullet.pos.z, wall) {
                    hit_wall = true;
                    break;
                }
            }
            if hit_wall {
                self.bullets.remove(i);
                continue;
            }
            let mut hit_enemy = false;
            let mut j = 0;
            while j < self.enemies.len() {
                let enemy = self.enemies[j];
                let dx = bullet.pos.x - enemy.pos.x;
                let dy = bullet.pos.y - enemy.pos.y;
                let dz = bullet.pos.z - enemy.pos.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist < 1.0 {
                    enemy.health = enemy.health - bullet.damage;
                    if enemy.health <= 0 {
                        enemy.state = 3;
                        self.score = self.score + 100;
                        println(format!("{}{}", "Enemy killed! Score: ", self.score.to_string()))
                    }
                    hit_enemy = true;
                    break;
                }
                j += 1;
            }
            if hit_enemy {
                self.bullets.remove(i);
                continue;
            }
            i += 1;
        }
}
fn shoot(mut self) {
        let yaw_rad = self.player_yaw * 3.14159 / 180.0;
        let pitch_rad = self.player_pitch * 3.14159 / 180.0;
        let forward_x = yaw_rad.sin() * pitch_rad.cos();
        let forward_y = pitch_rad.sin();
        let forward_z = yaw_rad.cos() * pitch_rad.cos();
        let speed = {
            if self.weapon == 0 {
                30.0
            } else {
                if self.weapon == 1 {
                    25.0
                } else {
                    20.0
                }
            }
        };
        let damage = {
            if self.weapon == 0 {
                1
            } else {
                if self.weapon == 1 {
                    2
                } else {
                    3
                }
            }
        };
        let spawn_offset = 1.5;
        let bullet_pos = Vec3::new(self.player_pos.x + forward_x * spawn_offset, self.player_pos.y + forward_y * spawn_offset, self.player_pos.z + forward_z * spawn_offset);
        self.bullets.push(Bullet { pos: bullet_pos, velocity: Vec3::new(forward_x * speed, forward_y * speed, forward_z * speed), damage, lifetime: 5.0 });
        println(format!("{}{}", "BANG! Fired weapon ", self.weapon.to_string()))
}
}

#[init]
fn init(mut game: ShooterGame) {
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
    game.spawn_powerups();
    println("=== GREYBOX SHOOTER ===");
    println("WASD - Move");
    println("Space - Jump");
    println("Shift - Sprint");
    println("Mouse - Look");
    println("Left Click - Shoot");
    println("1/2/3 - Switch weapon");
    println("ESC - Pause");
    println("");
    println("Kill all enemies to win!")
}

#[input]
fn handle_input(mut game: ShooterGame, mut input: Input) {
    if game.paused {
        if input.pressed(Key::Escape) {
            game.paused = false;
            println("Game resumed!")
        }
        return;
    }
    if input.pressed(Key::Escape) {
        game.paused = true;
        println("Game paused! Press ESC to resume");
        return;
    }
    if input.mouse_pressed(MouseButton::Left) {
        game.shoot()
    }
    if input.pressed(Key::Num1) {
        game.weapon = 0;
        println("Switched to Pistol")
    }
    if input.pressed(Key::Num2) {
        game.weapon = 1;
        println("Switched to Shotgun")
    }
    if input.pressed(Key::Num3) {
        game.weapon = 2;
        println("Switched to Rocket Launcher")
    }
}

#[update]
fn update(mut game: ShooterGame, mut delta: f32, mut input: Input) {
    if game.paused {
        return;
    }
    let dx = input.mouse_delta_x() as f32;
    let dy = input.mouse_delta_y() as f32;
    game.player_yaw = game.player_yaw - dx * game.mouse_sensitivity;
    game.player_pitch = game.player_pitch - dy * game.mouse_sensitivity;
    if game.player_pitch > 89.0 {
        game.player_pitch = 89.0;
    }
    if game.player_pitch < -89.0 {
        game.player_pitch = -89.0;
    }
    game.update_player_movement(delta, input);
    game.collect_powerups();
    if game.speed_boost_timer > 0.0 {
        game.speed_boost_timer = game.speed_boost_timer - delta;
        if game.speed_boost_timer < 0.0 {
            game.speed_boost_timer = 0.0;
            println("Speed boost ended!")
        }
    }
    game.update_enemies(delta);
    game.update_bullets(delta);
    if game.enemies.len() == 0 {
        println(format!("{}{}", "YOU WIN! Score: ", game.score.to_string()));
        println("Press ESC to exit")
    }
}

#[inline]
fn check_collision(mut x: f32, mut z: f32, mut wall: Wall) -> bool {
    let half_width = wall.size.x / 2.0;
    let half_depth = wall.size.z / 2.0;
    let player_radius = 0.5;
    let dx = (x - wall.pos.x).abs();
    let dz = (z - wall.pos.z).abs();
    return dx < half_width + player_radius && dz < half_depth + player_radius;
}

#[render3d]
fn render(mut game: ShooterGame, mut renderer: Renderer3D, mut camera: Camera3D) {
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
    for powerup in game.powerups {
        if powerup.active {
            renderer.draw_cube(powerup.pos, Vec3::new(0.5, 0.5, 0.5), powerup.color)
        }
    }
    let hud_distance = 2.0;
    let hud_offset_x = -1.5;
    let hud_offset_y = 1.0;
    let yaw_rad = camera.yaw * 3.14159 / 180.0;
    let pitch_rad = camera.pitch * 3.14159 / 180.0;
    let forward_x = yaw_rad.sin() * pitch_rad.cos();
    let forward_y = pitch_rad.sin();
    let forward_z = yaw_rad.cos() * pitch_rad.cos();
    let right_x = (yaw_rad + 1.5708).sin();
    let right_z = (yaw_rad + 1.5708).cos();
    let hud_base_x = camera.position.x + forward_x * hud_distance;
    let hud_base_y = camera.position.y + forward_y * hud_distance + hud_offset_y;
    let hud_base_z = camera.position.z + forward_z * hud_distance;
    let health_ratio = game.player_health as f32 / 100.0;
    let health_width = 0.5 * health_ratio;
    renderer.draw_cube(Vec3::new(hud_base_x + right_x * hud_offset_x, hud_base_y, hud_base_z + right_z * hud_offset_x), Vec3::new(health_width, 0.05, 0.01), Color::rgb(1.0, 0.0, 0.0));
    let ammo_display = {
        if game.ammo > 10 {
            10
        } else {
            game.ammo
        }
    };
    let mut ammo_x = hud_offset_x;
    let ammo_y = hud_offset_y - 0.15;
    let mut i = 0;
    while i < ammo_display {
        renderer.draw_cube(Vec3::new(hud_base_x + right_x * ammo_x, hud_base_y + ammo_y, hud_base_z + right_z * ammo_x), Vec3::new(0.04, 0.04, 0.01), Color::rgb(1.0, 0.8, 0.0));
        ammo_x += 0.05;
        i += 1;
    }
    let score_display = game.score / 100;
    let score_cubes = {
        if score_display > 10 {
            10
        } else {
            score_display
        }
    };
    let mut score_x = hud_offset_x;
    let score_y = hud_offset_y - 0.3;
    let mut j = 0;
    while j < score_cubes {
        renderer.draw_cube(Vec3::new(hud_base_x + right_x * score_x, hud_base_y + score_y, hud_base_z + right_z * score_x), Vec3::new(0.04, 0.04, 0.01), Color::rgb(0.0, 1.0, 0.0));
        score_x += 0.05;
        j += 1;
    }
    let weapon_color = {
        if game.weapon == 0 {
            Color::rgb(0.7, 0.7, 0.7)
        } else {
            if game.weapon == 1 {
                Color::rgb(0.8, 0.4, 0.0)
            } else {
                Color::rgb(1.0, 0.0, 0.0)
            }
        }
    };
    renderer.draw_cube(Vec3::new(hud_base_x + right_x * hud_offset_x, hud_base_y - 0.45, hud_base_z + right_z * hud_offset_x), Vec3::new(0.1, 0.1, 0.01), weapon_color)
}

#[inline]
#[cleanup]
fn cleanup(mut game: ShooterGame) {
    println(format!("{}{}", "Final Score: ", game.score.to_string()));
    println("Thanks for playing!")
}

fn main() {
    run_game()
}

