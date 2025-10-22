use windjammer_runtime::game::*;


struct SimpleGame {
    player_x: f64,
    player_y: f64,
    score: i64,
}

// Game trait implementation for SimpleGame
// TODO: Implement Game trait

impl SimpleGame {
#[inline]
fn new(&self) -> Self {
        Self { player_x: 0.0, player_y: 0.0, score: 0 }
}
#[inline]
fn move_player(&mut self, dx: f64, dy: f64) {
        self.player_x = self.player_x + dx;
        self.player_y = self.player_y + dy;
}
#[inline]
fn add_score(&mut self, points: i64) {
        self.score = self.score + points;
}
}

fn main() {
    let mut game = SimpleGame::new();
    println!("Game initialized at position: ({}, {})", game.player_x, game.player_y);
    game.move_player(10.0, 5.0);
    println!("Player moved to: ({}, {})", game.player_x, game.player_y);
    game.add_score(100);
    println!("Score: {}", game.score)
}

