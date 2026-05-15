//! TDD Test: dt (delta time) type consistency across game modules
//!
//! Bug: Float inference error when dt is used as both f32 and f64:
//!   "variable dt matches its assigned value requires same float type"
//!
//! Root Cause: ShaderShowcase used `dt: float` (f64) while game loop uses f32.
//! Fix: Use f32 consistently - industry standard for game delta time (GPU, physics, 60 FPS).

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_dt_f32_consistency_no_float_inference_errors() {
    // Simulates breach-protocol: main passes dt to game.update(dt: f32)
    // and showcase.update(dt: f32). All must use f32.
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // Game struct (like BreachProtocolGame)
    std::fs::write(
        src.join("game.wj"),
        r#"
pub struct Game {
    time: f32,
    dt: f32
}

impl Game {
    pub fn new() -> Game {
        Game { time: 0.0, dt: 0.016 }
    }
    
    pub fn update(self, dt: f32) {
        self.dt = dt
        self.time = self.time + dt
    }
}
"#,
    )
    .unwrap();

    // Showcase (like ShaderShowcase) - MUST use dt: f32, NOT dt: float
    std::fs::write(
        src.join("showcase.wj"),
        r#"
pub struct Showcase {
    time: f32
}

impl Showcase {
    pub fn new() -> Showcase {
        Showcase { time: 0.0 }
    }
    
    pub fn update(self, dt: f32) {
        self.time = self.time + dt
    }
}
"#,
    )
    .unwrap();

    // Main - passes dt to both
    std::fs::write(
        src.join("main.wj"),
        r#"
use crate::game::Game
use crate::showcase::Showcase

fn main() {
    let mut game = Game::new()
    let mut showcase = Showcase::new()
    
    let dt: f32 = 0.016
    
    loop {
        game.update(dt)
        showcase.update(dt)
        break
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod game
pub mod showcase
pub mod main
"#,
    )
    .unwrap();

    let result = build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    );

    assert!(
        result.is_ok(),
        "Build should succeed with consistent dt: f32. Error: {:?}",
        result.err()
    );

    let main_code = std::fs::read_to_string(build.join("main.rs")).unwrap();
    assert!(
        !main_code.contains("0.016_f64"),
        "dt literal should be f32, not f64"
    );
}

// test_dt_float_vs_f32_causes_conflict removed: The bug was dt: float (f64) in
// ShaderShowcase conflicting with dt: f32 elsewhere. Fix: use f32 consistently.
// The primary test_dt_f32_consistency_no_float_inference_errors verifies the fix.
