//! Simple platformer game example using windjammer-ui game framework

use windjammer_ui::game::*;

struct Player {
    id: EntityId,
    position: Vec2,
    velocity: Vec2,
    on_ground: bool,
}

impl GameEntity for Player {
    fn update(&mut self, delta: f32) {
        // Apply gravity
        if !self.on_ground {
            self.velocity.y += 980.0 * delta;
        }

        // Update position
        self.position += self.velocity * delta;

        // Simple ground collision
        if self.position.y > 500.0 {
            self.position.y = 500.0;
            self.velocity.y = 0.0;
            self.on_ground = true;
        } else {
            self.on_ground = false;
        }
    }

    fn id(&self) -> EntityId {
        self.id
    }
}

struct SimpleGame {
    player: Player,
    time: f32,
    score: i32,
}

impl GameLoop for SimpleGame {
    fn update(&mut self, delta: f32) {
        self.time += delta;
        self.player.update(delta);

        // Award points over time
        if self.time as i32 % 60 == 0 {
            self.score += 1;
        }
    }

    fn render(&self, ctx: &RenderContext) {
        // Clear screen
        ctx.clear(Color::rgb(0.2, 0.3, 0.8));

        // Draw player (as a rectangle for now)
        ctx.draw_rect(self.player.position, Vec2::new(32.0, 32.0), Color::RED);

        // Draw ground
        ctx.draw_rect(Vec2::new(0.0, 500.0), Vec2::new(800.0, 100.0), Color::GREEN);

        // Draw score
        ctx.draw_text(&format!("Score: {}", self.score), Vec2::new(10.0, 10.0));
        ctx.draw_text(
            &format!(
                "Position: ({:.1}, {:.1})",
                self.player.position.x, self.player.position.y
            ),
            Vec2::new(10.0, 30.0),
        );
    }

    fn start(&mut self) {
        println!("🎮 Simple Game Started!");
    }
}

fn main() {
    println!("🚀 Windjammer UI Game Framework - Simple Platformer Example\n");

    let player = Player {
        id: 0,
        position: Vec2::new(400.0, 100.0),
        velocity: Vec2::ZERO,
        on_ground: false,
    };

    let mut game = SimpleGame {
        player,
        time: 0.0,
        score: 0,
    };

    // Simulate game loop for demonstration
    println!("Simulating game loop (60 FPS for 3 seconds)...\n");

    for frame in 0..180 {
        let delta = 1.0 / 60.0; // 60 FPS
        game.update(delta);

        // Print state every 60 frames (1 second)
        if frame % 60 == 0 {
            let ctx = RenderContext::new();
            game.render(&ctx);
            println!(
                "Frame {}: Player at ({:.1}, {:.1}), Score: {}, On Ground: {}",
                frame,
                game.player.position.x,
                game.player.position.y,
                game.score,
                game.player.on_ground
            );
        }
    }

    println!("\n✅ Game simulation completed successfully!");
    println!("   Final score: {}", game.score);
    println!(
        "   Final position: ({:.1}, {:.1})",
        game.player.position.x, game.player.position.y
    );
}
