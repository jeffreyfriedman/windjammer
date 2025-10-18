#[derive(Debug, Clone)]
struct Player {
    position: Vec2,
    velocity: Vec2,
    health: i64,
    score: i64,
}

#[derive(Debug, Clone)]
struct Enemy {
    position: Vec2,
    velocity: Vec2,
    health: i64,
}

#[derive(Debug, Clone)]
struct Bullet {
    position: Vec2,
    velocity: Vec2,
    from_player: bool,
}

#[game]
struct ShooterGame {
    player: Player,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    game_time: f32,
    spawn_timer: f32,
    game_over: bool,
}

impl ShooterGame {
#[inline]
fn new(&self) -> ShooterGame {
        ShooterGame { player: Player { position: Vec2::new(400.0, 550.0), velocity: Vec2::ZERO, health: 100, score: 0 }, enemies: vec![], bullets: vec![], game_time: 0.0, spawn_timer: 0.0, game_over: false }
}
#[inline]
fn shoot(&self) {
        self.bullets.push(Bullet { position: Vec2::new(self.player.position::x, self.player.position::y - 20.0), velocity: Vec2::new(0.0, -500.0), from_player: true })
}
#[inline]
fn spawn_enemy(&self) {
        let x = self.game_time * 137.5 % 750.0 + 25.0;
        self.enemies.push(Enemy { position: Vec2::new(x, 50.0), velocity: Vec2::new(0.0, 100.0), health: 50 })
}
}

impl GameLoop for ShooterGame {
fn update(&self, delta: f32) {
        if self.game_over {
            return;
        }
        self.game_time += delta;
        self.spawn_timer += delta;
        if self.spawn_timer >= 2.0 {
            spawn_enemy();
            self.spawn_timer = 0.0;
        }
        self.player.position::x = self.player.position::x + self.player.velocity::x * delta;
        self.player.position::x = self.player.position::x::clamp(0.0, 800.0);
        for enemy in self.enemies {
            enemy.position::y = enemy.position::y + enemy.velocity::y * delta;
        }
        for bullet in self.bullets {
            bullet.position::y = bullet.position::y + bullet.velocity::y * delta;
        }
        let mut bullets_to_remove = vec![];
        let mut enemies_to_remove = vec![];
        for (b_idx, bullet) in self.bullets.enumerate() {
            if !bullet.from_player {
                continue;
            }
            for (e_idx, enemy) in self.enemies.enumerate() {
                let dx = bullet.position::x - enemy.position::x;
                let dy = bullet.position::y - enemy.position::y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance < 30.0 {
                    enemy.health = enemy.health - 25;
                    bullets_to_remove.push(b_idx);
                    if enemy.health <= 0 {
                        enemies_to_remove.push(e_idx);
                        self.player.score = self.player.score + 100;
                    }
                    break;
                }
            }
        }
        for idx in enemies_to_remove.reversed() {
            self.enemies.remove(idx);
        }
        for idx in bullets_to_remove.reversed() {
            self.bullets.remove(idx);
        }
        self.bullets = self.bullets.filter(|b| b.position::y > -50.0 && b.position::y < 650.0);
        let enemies_at_bottom = self.enemies.filter(|e| e.position::y > 560.0).len();
        if enemies_at_bottom > 0 {
            self.player.health = self.player.health - enemies_at_bottom * 10;
        }
        self.enemies = self.enemies.filter(|e| e.position::y <= 560.0);
        if self.player.health <= 0 {
            self.game_over = true;
        }
}
fn render(&self, ctx: &mut RenderContext) {
        ctx.clear(Color::BLACK);
        ctx.draw_rect(self.player.position::x - 20.0, self.player.position::y - 20.0, 40.0, 40.0, Color::GREEN);
        for enemy in self.enemies {
            ctx.draw_rect(enemy.position::x - 15.0, enemy.position::y - 15.0, 30.0, 30.0, Color::RED);
        }
        for bullet in self.bullets {
            let color = {
                if bullet.from_player {
                    Color::GREEN
                } else {
                    Color::RED
                }
            };
            ctx.draw_circle(bullet.position::x, bullet.position::y, 5.0, color);
        }
        ctx.draw_text("Score: {player.score}", 10.0, 30.0, Color::WHITE);
        ctx.draw_text("Health: {player.health}", 10.0, 60.0, Color::WHITE);
        ctx.draw_text("Enemies: {enemies.len()}", 10.0, 90.0, Color::WHITE);
        if self.game_over {
            ctx.draw_text("GAME OVER! Final Score: {player.score}", 200.0, 300.0, Color::RED)
        }
}
}

fn main() {
    println!("🚀 Space Shooter Game Example");
    println!("==============================
");
    let mut game = ShooterGame::new();
    let ctx = RenderContext::new();
    println!("Starting game simulation...
");
    for frame in 0..180 {
        let delta = 0.016666666666666666;
        if frame % 20 == 0 {
            game.shoot();
            println!(format!("Frame {}: Player shoots! Bullets: {game.bullets.len()}", frame))
        }
        game.update(delta);
        game.render(ctx);
        if frame % 60 == 0 {
            println!("Time: {game.game_time:.1}s | Score: {game.player.score} | Health: {game.player.health} | Enemies: {game.enemies.len()} | Bullets: {game.bullets.len()}")
        }
        if game.game_over {
            println!("
💀 Game Over!");
            println!("Final Score: {game.player.score}");
            println!("Survived: {game.game_time:.1} seconds");
            break;
        }
    }
    if !game.game_over {
        println!("
✅ Simulation complete!");
        println!("Final Score: {game.player.score}");
        println!("Health Remaining: {game.player.health}")
    }
    println!("
🎯 Key Features Demonstrated:");
    println!("  - Entity management (player, enemies, bullets)");
    println!("  - Collision detection");
    println!("  - Spawning system");
    println!("  - Score tracking");
    println!("  - Health system");
    println!("  - Game over condition")
}

