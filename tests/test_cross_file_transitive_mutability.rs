// TDD: Test cross-file transitive mutability inference
//
// Bug: Multi-pass works WITHIN impl blocks but NOT ACROSS files
//
// Example (breach-protocol):
//   File 1: input/keyboard.wj
//     KeyboardState::update_key(self) mutates self.keys → &mut self
//   
//   File 2: game.wj
//     Game::poll_keyboard_input(self) calls self.keyboard.update_key(...)
//     Should infer &mut self, but currently infers &self
//
// Root Cause: File processing order + single-pass across files
//   - When game.wj is analyzed, keyboard.wj might not be analyzed yet
//   - Registry doesn't have KeyboardState::update_key signature
//   - poll_keyboard_input can't see that update_key needs &mut
//
// Solution: Global multi-pass iteration across ALL files

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_file_transitive_mutation() {
    // This test reproduces the exact breach-protocol pattern
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("input")).unwrap();
    
    // File 1: KeyboardState in input/keyboard.wj (defines mutating method)
    std::fs::write(
        src.join("input/keyboard.wj"),
        r#"
pub struct KeyboardState {
    pub keys: Vec<bool>
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState { keys: Vec::new() }
    }
    
    // Direct mutation - should be &mut self
    pub fn update_key(self, key: i32, pressed: bool) {
        while self.keys.len() <= (key as usize) {
            self.keys.push(false)
        }
        self.keys[key as usize] = pressed
    }
    
    pub fn is_key_down(self, key: i32) -> bool {
        key >= 0 && key < (self.keys.len() as i32) && self.keys[key as usize]
    }
}
"#,
    )
    .unwrap();
    
    // File 2: Game in game.wj (calls KeyboardState methods)
    std::fs::write(
        src.join("game.wj"),
        r#"
use crate::input::keyboard::KeyboardState

pub struct Game {
    pub keyboard: KeyboardState
}

impl Game {
    pub fn new() -> Game {
        Game {
            keyboard: KeyboardState::new()
        }
    }
    
    // Cross-file transitive mutation:
    // Calls self.keyboard.update_key() which needs &mut
    // Should infer &mut self for poll_input
    fn poll_input(self) {
        let key_down = true
        self.keyboard.update_key(1, key_down)
        self.keyboard.update_key(2, false)
    }
    
    // Level 2: Calls poll_input which calls mutating method
    // Should ALSO infer &mut self
    pub fn update(self) {
        self.poll_input()
    }
}
"#,
    )
    .unwrap();
    
    // File 3: input/mod.wj (module declaration)
    std::fs::write(
        src.join("input/mod.wj"),
        r#"
pub mod keyboard
"#,
    )
    .unwrap();
    
    // File 4: Root mod.wj
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod input
pub mod game
"#,
    )
    .unwrap();
    
    // Build as library (multi-file)
    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true, // library mode
        &[],
    )
    .expect("Build should succeed");
    
    // Check KeyboardState methods
    let keyboard_code = std::fs::read_to_string(build.join("input/keyboard.rs")).unwrap();
    assert!(
        keyboard_code.contains("pub fn update_key(&mut self"),
        "KeyboardState::update_key should be &mut self (direct mutation)"
    );
    assert!(
        keyboard_code.contains("pub fn is_key_down(&self"),
        "KeyboardState::is_key_down should be &self (read-only)"
    );
    
    // Check Game methods - THIS IS WHERE THE BUG IS
    let game_code = std::fs::read_to_string(build.join("game.rs")).unwrap();
    
    // ASSERT: poll_input should be &mut self (calls mutating method on field)
    assert!(
        game_code.contains("fn poll_input(&mut self)"),
        "Game::poll_input should be &mut self (cross-file transitive mutation). Found:\n{}",
        game_code.lines().find(|l| l.contains("fn poll_input")).unwrap_or("NOT FOUND")
    );
    
    // ASSERT: update should be &mut self (calls poll_input which is &mut)
    assert!(
        game_code.contains("pub fn update(&mut self)"),
        "Game::update should be &mut self (level 2 cross-file). Found:\n{}",
        game_code.lines().find(|l| l.contains("fn update")).unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_cross_file_with_three_files() {
    // More complex: A → B → C (three files, two-level transitive)
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // File 1: state.wj (level 0 - direct mutation)
    std::fs::write(
        src.join("state.wj"),
        r#"
pub struct State {
    pub value: i32
}

impl State {
    pub fn new() -> State {
        State { value: 0 }
    }
    
    pub fn increment(self) {
        self.value = self.value + 1
    }
}
"#,
    )
    .unwrap();
    
    // File 2: manager.wj (level 1 - calls State::increment)
    std::fs::write(
        src.join("manager.wj"),
        r#"
use crate::state::State

pub struct Manager {
    pub state: State
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            state: State::new()
        }
    }
    
    fn tick(self) {
        self.state.increment()
    }
}
"#,
    )
    .unwrap();
    
    // File 3: app.wj (level 2 - calls Manager::tick)
    std::fs::write(
        src.join("app.wj"),
        r#"
use crate::manager::Manager

pub struct App {
    pub manager: Manager
}

impl App {
    pub fn new() -> App {
        App {
            manager: Manager::new()
        }
    }
    
    pub fn run(self) {
        self.manager.tick()
    }
}
"#,
    )
    .unwrap();
    
    // Root
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod state
pub mod manager
pub mod app
"#,
    )
    .unwrap();
    
    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("Build should succeed");
    
    // Verify all levels have correct inference
    let state_code = std::fs::read_to_string(build.join("state.rs")).unwrap();
    assert!(
        state_code.contains("pub fn increment(&mut self)"),
        "State::increment should be &mut self"
    );
    
    let manager_code = std::fs::read_to_string(build.join("manager.rs")).unwrap();
    assert!(
        manager_code.contains("fn tick(&mut self)"),
        "Manager::tick should be &mut self (cross-file level 1)"
    );
    
    let app_code = std::fs::read_to_string(build.join("app.rs")).unwrap();
    assert!(
        app_code.contains("pub fn run(&mut self)"),
        "App::run should be &mut self (cross-file level 2)"
    );
}
