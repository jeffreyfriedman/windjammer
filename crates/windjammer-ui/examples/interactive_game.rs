//! Interactive platformer game with keyboard controls

use windjammer_ui::game::*;

struct Player {
    id: EntityId,
    position: Vec2,
    velocity: Vec2,
    on_ground: bool,
    speed: f32,
    jump_force: f32,
}

impl Player {
    fn new(id: EntityId) -> Self {
        Self {
            id,
            position: Vec2::new(400.0, 100.0),
            velocity: Vec2::ZERO,
            on_ground: false,
            speed: 300.0,
            jump_force: -600.0,
        }
    }

    fn handle_input(&mut self, input: &Input) {
        // Horizontal movement
        if input.key_pressed(Key::Left) || input.key_pressed(Key::A) {
            self.velocity.x = -self.speed;
        } else if input.key_pressed(Key::Right) || input.key_pressed(Key::D) {
            self.velocity.x = self.speed;
        } else {
            self.velocity.x = 0.0;
        }

        // Jump
        if (input.key_just_pressed(Key::Space)
            || input.key_just_pressed(Key::Up)
            || input.key_just_pressed(Key::W))
            && self.on_ground
        {
            self.velocity.y = self.jump_force;
            self.on_ground = false;
        }
    }
}

impl GameEntity for Player {
    fn update(&mut self, delta: f32) {
        // Apply gravity
        if !self.on_ground {
            self.velocity.y += 980.0 * delta;
        }

        // Update position
        self.position += self.velocity * delta;

        // Keep player on screen horizontally
        if self.position.x < 16.0 {
            self.position.x = 16.0;
        } else if self.position.x > 784.0 {
            self.position.x = 784.0;
        }

        // Ground collision
        if self.position.y > 468.0 {
            self.position.y = 468.0;
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

struct InteractiveGame {
    player: Player,
    input: Input,
    time: f32,
    jumps: i32,
    instructions_shown: bool,
}

impl GameLoop for InteractiveGame {
    fn start(&mut self) {
        println!("\n🎮 Interactive Platformer - Controls:");
        println!("   ⬅️  LEFT/A    - Move left");
        println!("   ➡️  RIGHT/D   - Move right");
        println!("   ⬆️  SPACE/W   - Jump");
        println!("   🔄 ESC       - Quit\n");
        println!("📊 Game started! Use the controls to move around.\n");
        self.instructions_shown = true;
    }

    fn update(&mut self, delta: f32) {
        self.time += delta;

        // Update input state
        self.input.update();

        // Handle player input
        self.player.handle_input(&self.input);

        // Track jumps
        if self.input.key_just_pressed(Key::Space)
            || self.input.key_just_pressed(Key::Up)
            || self.input.key_just_pressed(Key::W)
        {
            if self.player.on_ground {
                self.jumps += 1;
            }
        }

        // Update player
        self.player.update(delta);
    }

    fn render(&self, ctx: &RenderContext) {
        // Clear screen (sky blue)
        ctx.clear(Color::rgb(0.53, 0.81, 0.92));

        // Draw player (red square)
        ctx.draw_rect(
            Vec2::new(self.player.position.x - 16.0, self.player.position.y - 16.0),
            Vec2::new(32.0, 32.0),
            Color::RED,
        );

        // Draw ground (green)
        ctx.draw_rect(Vec2::new(0.0, 500.0), Vec2::new(800.0, 100.0), Color::GREEN);

        // Draw UI
        ctx.draw_text(&format!("Time: {:.1}s", self.time), Vec2::new(10.0, 10.0));
        ctx.draw_text(&format!("Jumps: {}", self.jumps), Vec2::new(10.0, 30.0));
        ctx.draw_text(
            &format!(
                "Position: ({:.0}, {:.0})",
                self.player.position.x, self.player.position.y
            ),
            Vec2::new(10.0, 50.0),
        );
        ctx.draw_text(
            &format!(
                "Velocity: ({:.0}, {:.0})",
                self.player.velocity.x, self.player.velocity.y
            ),
            Vec2::new(10.0, 70.0),
        );
        ctx.draw_text(
            &format!("On Ground: {}", self.player.on_ground),
            Vec2::new(10.0, 90.0),
        );
    }
}

fn main() {
    println!("🚀 Windjammer UI Game Framework - Interactive Platformer\n");

    let player = Player::new(0);
    let input = Input::new();

    let mut game = InteractiveGame {
        player,
        input,
        time: 0.0,
        jumps: 0,
        instructions_shown: false,
    };

    game.start();

    // Simulate interactive gameplay with pre-programmed inputs
    println!("🎬 Simulating gameplay with pre-programmed inputs...\n");

    let actions = vec![
        (30, "move_right"), // Frame 30: start moving right
        (60, "jump"),       // Frame 60: jump
        (90, "stop"),       // Frame 90: stop moving
        (120, "move_left"), // Frame 120: move left
        (150, "jump"),      // Frame 150: jump again
        (180, "stop"),      // Frame 180: stop
    ];

    for frame in 0..240 {
        let delta = 1.0 / 60.0; // 60 FPS

        // Simulate input based on actions
        for (action_frame, action) in &actions {
            if frame == *action_frame {
                match *action {
                    "move_right" => {
                        println!("   🎮 Action: Moving right");
                        game.input.press_key(Key::Right);
                    }
                    "move_left" => {
                        println!("   🎮 Action: Moving left");
                        game.input.release_key(Key::Right);
                        game.input.press_key(Key::Left);
                    }
                    "jump" => {
                        println!("   🎮 Action: Jump!");
                        game.input.press_key(Key::Space);
                    }
                    "stop" => {
                        println!("   🎮 Action: Stop moving");
                        game.input.release_key(Key::Right);
                        game.input.release_key(Key::Left);
                        game.input.release_key(Key::Space);
                    }
                    _ => {}
                }
            }
        }

        game.update(delta);

        // Release jump key after one frame (simulate key press)
        if actions.iter().any(|(f, a)| *f == frame && *a == "jump") {
            game.input.release_key(Key::Space);
        }

        // Print state every 60 frames (1 second)
        if frame % 60 == 0 || actions.iter().any(|(f, _)| *f == frame) {
            let ctx = RenderContext::new();
            game.render(&ctx);
            println!(
                "   Frame {:3}: Pos({:5.1}, {:5.1}) Vel({:5.1}, {:5.1}) Ground:{} Jumps:{}",
                frame,
                game.player.position.x,
                game.player.position.y,
                game.player.velocity.x,
                game.player.velocity.y,
                game.player.on_ground,
                game.jumps
            );
        }
    }

    println!("\n✅ Gameplay simulation completed!");
    println!(
        "   Final position: ({:.1}, {:.1})",
        game.player.position.x, game.player.position.y
    );
    println!("   Total jumps: {}", game.jumps);
    println!("   Time played: {:.1}s", game.time);
    println!("\n💡 In a real game, this would run in a window with actual keyboard input!");
}
