struct Paddle {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    speed: f64,
}

struct Ball {
    x: f64,
    y: f64,
    size: f64,
    dx: f64,
    dy: f64,
}

struct Game {
    left_paddle: Paddle,
    right_paddle: Paddle,
    ball: Ball,
    left_score: i64,
    right_score: i64,
}

impl Game {
#[inline]
fn new() -> Game {
        Game { left_paddle: Paddle { x: -0.9, y: 0.0, width: 0.05, height: 0.3, speed: 0.02 }, right_paddle: Paddle { x: 0.85, y: 0.0, width: 0.05, height: 0.3, speed: 0.02 }, ball: Ball { x: 0.0, y: 0.0, size: 0.04, dx: 0.01, dy: 0.008 }, left_score: 0, right_score: 0 }
}
fn update_paddle(&mut self, mut paddle: &mut Paddle, mut up: bool, mut down: bool) {
        if up {
            paddle.y = paddle.y + paddle.speed;
        }
        if down {
            paddle.y = paddle.y - paddle.speed;
        }
        if paddle.y > 1.0 {
            paddle.y = 1.0;
        }
        if paddle.y < -1.0 + paddle.height {
            paddle.y = -1.0 + paddle.height;
        }
}
#[inline]
fn update_ball(&mut self) {
        self.ball.x = self.ball.x + self.ball.dx;
        self.ball.y = self.ball.y + self.ball.dy;
        if self.ball.y > 1.0 || self.ball.y < -1.0 {
            self.ball.dy = -self.ball.dy;
        }
}
#[inline]
fn check_collision(&self, mut paddle: &Paddle) -> bool {
        self.ball.x < paddle.x + paddle.width && self.ball.x + self.ball.size > paddle.x && self.ball.y < paddle.y && self.ball.y - self.ball.size > paddle.y - paddle.height
}
#[inline]
fn check_collisions(&mut self) {
        if self.check_collision(&self.left_paddle) {
            self.ball.dx = self.ball.dx.abs();
        }
        if self.check_collision(&self.right_paddle) {
            self.ball.dx = -self.ball.dx.abs();
        }
}
#[inline]
fn check_scoring(&mut self) -> bool {
        if self.ball.x < -1.0 {
            self.right_score = self.right_score + 1;
            println!("Right scores! {} - {}", self.left_score, self.right_score);
            self.ball.x = 0.0;
            self.ball.y = 0.0;
            self.ball.dx = -self.ball.dx;
            return true;
        }
        if self.ball.x > 1.0 {
            self.left_score = self.left_score + 1;
            println!("Left scores! {} - {}", self.left_score, self.right_score);
            self.ball.x = 0.0;
            self.ball.y = 0.0;
            self.ball.dx = -self.ball.dx;
            return true;
        }
        false
}
}

fn main() {
    println!("🎮 PONG - Pure Windjammer");
    println!("=========================");
    println!("This is a PURE WINDJAMMER game!");
    println!("No Rust syntax, no wgpu, no winit exposed!");
    println!("");
    println!("Game logic works - rendering would be added via");
    println!("the game framework once we have proper backend support.");
    println!("");
    let mut game = Game::new();
    println!("Initial state:");
    println!("  Left paddle: ({}, {})", game.left_paddle.x, game.left_paddle.y);
    println!("  Right paddle: ({}, {})", game.right_paddle.x, game.right_paddle.y);
    println!("  Ball: ({}, {})", game.ball.x, game.ball.y);
    println!("");
    println!("Simulating game...");
    for i in 0..100 {
        game.update_ball();
        game.check_collisions();
        if game.check_scoring() {
            println!("  Frame {}: Score changed!", i)
        }
    }
    println!("");
    println!("Final score: {} - {}", game.left_score, game.right_score);
    println!("");
    println!("✅ This proves Windjammer can handle game logic!");
    println!("✅ Zero Rust exposure in game code!");
    println!("✅ Clean, simple syntax!")
}

