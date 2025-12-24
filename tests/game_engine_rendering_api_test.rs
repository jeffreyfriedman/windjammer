/// TDD Test: Game Engine Rendering API
///
/// Tests that the game engine provides all required rendering functions
/// for 2D games (rectangles, circles, text, colors, transforms).
///
/// THE WINDJAMMER WAY: Write tests first, then implement features
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_game_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test_game.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    // Set environment variable to point to local windjammer-game
    let wj_game_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .env("WJ_WINDJAMMER_GAME_PATH", wj_game_path)
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Try to cargo build to check if functions exist
    let cargo_output = Command::new("cargo")
        .arg("build")
        .current_dir(test_dir)
        .output()
        .expect("Failed to run cargo");

    if !cargo_output.status.success() {
        return Err(String::from_utf8_lossy(&cargo_output.stderr).to_string());
    }

    Ok("Build succeeded".to_string())
}

#[test]
fn test_rendering_api_basic_shapes() {
    let code = r#"
    use windjammer_game::prelude::*;
    
    struct TestGame {}
    
    impl GameLoop for TestGame {
        fn init(&mut self) {}
        
        fn update(&mut self, dt: f32) {}
        
        fn render(&mut self) {
            // Test basic rendering functions
            clear_color(0.0, 0.0, 0.0, 1.0);
            draw_rect(10.0, 10.0, 100.0, 50.0, 1.0, 0.0, 0.0, 1.0);
            draw_circle(200.0, 200.0, 25.0, 0.0, 1.0, 0.0, 1.0);
        }
    }
    
    fn main() {
        let config = GameConfig {
            title: "Test",
            width: 800,
            height: 600,
            vsync: true,
        };
        
        let mut game = TestGame {};
        run_game(&mut game, config);
    }
    "#;

    match compile_game_code(code) {
        Ok(_) => {
            // Success!
        }
        Err(err) => {
            // Check if it's missing the rendering functions
            if err.contains("cannot find function `clear_color`") {
                panic!("Missing clear_color function in prelude");
            }
            if err.contains("cannot find function `draw_rect`") {
                panic!("Missing draw_rect function in prelude");
            }
            if err.contains("cannot find function `draw_circle`") {
                panic!("Missing draw_circle function in prelude");
            }
            if err.contains("cannot find function `run_game`") {
                panic!("Missing run_game function in prelude");
            }
            if err.contains("cannot find struct `GameConfig`") {
                panic!("Missing GameConfig struct in prelude");
            }

            // Some other error - print it
            panic!("Compilation failed with unexpected error:\n{}", err);
        }
    }
}

#[test]
fn test_input_api() {
    let code = r#"
    use windjammer_game::prelude::*;
    
    struct TestGame {}
    
    impl GameLoop for TestGame {
        fn init(&mut self) {}
        
        fn update(&mut self, dt: f32) {
            let input = get_input();
            
            if input.key_down(Key::A) {
                // Move left
            }
            
            if input.key_pressed(Key::Space) {
                // Jump
            }
        }
        
        fn render(&mut self) {}
    }
    
    fn main() {
        let config = GameConfig {
            title: "Test",
            width: 800,
            height: 600,
            vsync: true,
        };
        
        let mut game = TestGame {};
        run_game(&mut game, config);
    }
    "#;

    match compile_game_code(code) {
        Ok(_) => {
            // Success!
        }
        Err(err) => {
            if err.contains("cannot find function `get_input`") {
                panic!("Missing get_input function in prelude");
            }
            if err.contains("cannot find enum `Key`") {
                panic!("Missing Key enum in prelude");
            }

            panic!("Compilation failed with unexpected error:\n{}", err);
        }
    }
}

#[test]
fn test_matrix_transforms() {
    let code = r#"
    use windjammer_game::prelude::*;
    
    struct TestGame {
        camera_x: f32,
        camera_y: f32,
    }
    
    impl GameLoop for TestGame {
        fn init(&mut self) {
            self.camera_x = 0.0;
            self.camera_y = 0.0;
        }
        
        fn update(&mut self, dt: f32) {}
        
        fn render(&mut self) {
            // Apply camera transform
            push_matrix();
            translate(-self.camera_x, -self.camera_y);
            
            draw_rect(100.0, 100.0, 50.0, 50.0, 1.0, 1.0, 1.0, 1.0);
            
            pop_matrix();
        }
    }
    
    fn main() {
        let config = GameConfig {
            title: "Test",
            width: 800,
            height: 600,
            vsync: true,
        };
        
        let mut game = TestGame { camera_x: 0.0, camera_y: 0.0 };
        run_game(&mut game, config);
    }
    "#;

    match compile_game_code(code) {
        Ok(_) => {
            // Success!
        }
        Err(err) => {
            if err.contains("cannot find function `push_matrix`") {
                panic!("Missing push_matrix function in prelude");
            }
            if err.contains("cannot find function `pop_matrix`") {
                panic!("Missing pop_matrix function in prelude");
            }
            if err.contains("cannot find function `translate`") {
                panic!("Missing translate function in prelude");
            }

            panic!("Compilation failed with unexpected error:\n{}", err);
        }
    }
}

#[test]
fn test_text_rendering() {
    let code = r#"
    use windjammer_game::prelude::*;
    
    struct TestGame {
        score: i32,
    }
    
    impl GameLoop for TestGame {
        fn init(&mut self) {
            self.score = 0;
        }
        
        fn update(&mut self, dt: f32) {}
        
        fn render(&mut self) {
            draw_text(
                format!("Score: {}", self.score),
                10.0,
                30.0,
                24.0,
                1.0, 1.0, 1.0, 1.0
            );
        }
    }
    
    fn main() {
        let config = GameConfig {
            title: "Test",
            width: 800,
            height: 600,
            vsync: true,
        };
        
        let mut game = TestGame { score: 0 };
        run_game(&mut game, config);
    }
    "#;

    match compile_game_code(code) {
        Ok(_) => {
            // Success!
        }
        Err(err) => {
            if err.contains("cannot find function `draw_text`") {
                panic!("Missing draw_text function in prelude");
            }

            panic!("Compilation failed with unexpected error:\n{}", err);
        }
    }
}
