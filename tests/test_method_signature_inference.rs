// TDD: Test method signature inference (self vs &self vs &mut self)
//
// Bug: Analyzer incorrectly infers method signatures:
// - Getters that return owned values inferred as `self` (consuming) instead of `&self` (borrowing)
// - Methods that mutate fields inferred as `&self` (immutable) instead of `&mut self` (mutable)
//
// Root Cause: Ownership inference doesn't distinguish between:
// 1. Methods that need to consume self (move ownership out)
// 2. Methods that only need to read self (borrow immutably)
// 3. Methods that need to mutate self (borrow mutably)

use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

#[test]
fn test_getter_should_borrow_not_consume() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // Getter method that returns a field value
    std::fs::write(
        src.join("game.wj"),
        r#"
pub struct Hud {
    pub frame_count: u64
}

pub struct Game {
    pub hud: Hud
}

impl Game {
    pub fn new() -> Game {
        Game {
            hud: Hud { frame_count: 0 }
        }
    }
    
    // Getter: should be &self (borrow), NOT self (consume)
    pub fn get_hud(self) -> Hud {
        self.hud
    }
}

pub fn test_usage() {
    let mut game = Game::new()
    let hud = game.get_hud()  // First call
    game.update(0.016)        // Should still work! game not consumed
}

impl Game {
    pub fn update(self, dt: f32) {
        self.hud.frame_count = self.hud.frame_count + 1
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("game.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("game.rs")).unwrap();
    
    // ASSERT: get_hud should be &self, not self
    assert!(
        rust_code.contains("pub fn get_hud(&self)") || rust_code.contains("pub fn get_hud(& self)"),
        "get_hud should be &self (borrow), not self (consume). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn get_hud")).unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_mutating_method_should_be_mut_self() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // Method that mutates fields
    std::fs::write(
        src.join("keyboard.wj"),
        r#"
pub struct KeyboardState {
    pub keys: Vec<bool>
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState {
            keys: Vec::new()
        }
    }
    
    // Mutates self.keys, should be &mut self, NOT &self
    pub fn update_key(self, key: i32, pressed: bool) {
        if key >= 0 && key < 256 {
            while self.keys.len() <= (key as usize) {
                self.keys.push(false)
            }
            self.keys[key as usize] = pressed
        }
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("keyboard.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("keyboard.rs")).unwrap();
    
    // ASSERT: update_key should be &mut self, not &self
    assert!(
        rust_code.contains("pub fn update_key(&mut self") || rust_code.contains("pub fn update_key(&mut  self"),
        "update_key should be &mut self (mutable), not &self. Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn update_key")).unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_method_calling_mutating_method_on_field() {
    // This reproduces the breach-protocol bug:
    // poll_keyboard_input(self) calls self.keyboard.update_key(...)
    // Should infer &mut self because it mutates self.keyboard
    
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    std::fs::write(
        src.join("game.wj"),
        r#"
pub struct KeyboardState {
    pub keys: Vec<bool>
}

impl KeyboardState {
    pub fn update_key(self, key: i32, pressed: bool) {
        if key >= 0 && key < 256 {
            while self.keys.len() <= (key as usize) {
                self.keys.push(false)
            }
            self.keys[key as usize] = pressed
        }
    }
}

pub struct Game {
    pub keyboard: KeyboardState
}

impl Game {
    pub fn new() -> Game {
        Game {
            keyboard: KeyboardState { keys: Vec::new() }
        }
    }
    
    // Calls self.keyboard.update_key(...) which mutates keyboard
    // Should infer &mut self, NOT self
    pub fn poll_input(self) {
        self.keyboard.update_key(1, true)
        self.keyboard.update_key(2, false)
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("game.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("game.rs")).unwrap();
    
    // ASSERT: poll_input should be &mut self
    assert!(
        rust_code.contains("pub fn poll_input(&mut self)"),
        "poll_input should be &mut self (mutates field via method call). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn poll_input")).unwrap_or("NOT FOUND")
    );
    
    // ASSERT: update_key should also be &mut self
    assert!(
        rust_code.contains("pub fn update_key(&mut self"),
        "update_key should be &mut self. Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn update_key")).unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_consuming_method_should_be_self() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // Method that truly needs to consume self (e.g., into_inner)
    std::fs::write(
        src.join("wrapper.wj"),
        r#"
pub struct Wrapper {
    pub value: String
}

impl Wrapper {
    pub fn new(v: String) -> Wrapper {
        Wrapper { value: v }
    }
    
    // Consumes self and returns inner value - CORRECT to be self
    pub fn into_inner(self) -> String {
        self.value
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("wrapper.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("wrapper.rs")).unwrap();
    
    // ASSERT: into_inner should be self (consume), not &self
    assert!(
        rust_code.contains("pub fn into_inner(self)"),
        "into_inner should be self (consume) since it moves out the inner value. Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn into_inner")).unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_readonly_method_should_be_ref_self() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    // Method that only reads fields (no mutation)
    std::fs::write(
        src.join("stats.wj"),
        r#"
pub struct Stats {
    pub score: i32,
    pub lives: i32
}

impl Stats {
    pub fn new() -> Stats {
        Stats { score: 0, lives: 3 }
    }
    
    // Only reads fields, should be &self
    pub fn is_game_over(self) -> bool {
        self.lives <= 0
    }
    
    // Only reads fields, should be &self
    pub fn get_score(self) -> i32 {
        self.score
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("stats.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("stats.rs")).unwrap();
    
    // ASSERT: Both should be &self (immutable borrow)
    assert!(
        rust_code.contains("pub fn is_game_over(&self)"),
        "is_game_over should be &self (read-only). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn is_game_over")).unwrap_or("NOT FOUND")
    );
    
    assert!(
        rust_code.contains("pub fn get_score(&self)"),
        "get_score should be &self (read-only). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn get_score")).unwrap_or("NOT FOUND")
    );
}
