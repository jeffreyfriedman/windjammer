// TDD: Test transitive mutability inference across method call chains
//
// Bug: Analyzer correctly infers first-level transitive mutation
//      (method → field mutation) but FAILS on second-level (method → method → field)
//
// Example:
//   update_key(self) mutates self.keys          → Infers &mut self ✅
//   poll_keyboard(self) calls update_key        → Infers &mut self ✅  
//   update(self) calls poll_keyboard(&mut self) → Infers &self ❌ (should be &mut self)
//
// Root Cause: Ownership inference only does ONE pass for method calls.
//             Needs multi-pass analysis to propagate mutability up the call chain.

use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

#[test]
fn test_two_level_transitive_mutation() {
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
    // Level 0: Direct field mutation
    pub fn update_key(self, key: i32, pressed: bool) {
        while self.keys.len() <= (key as usize) {
            self.keys.push(false)
        }
        self.keys[key as usize] = pressed
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
    
    // Level 1: Calls mutating method on field (transitive mutation)
    fn poll_keyboard(self) {
        self.keyboard.update_key(1, true)
    }
    
    // Level 2: Calls method that performs transitive mutation
    // THIS IS THE BUG: Should infer &mut self, but currently infers &self
    pub fn update(self) {
        self.poll_keyboard()
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("game.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("game.rs")).unwrap();
    
    // ASSERT: update_key should be &mut self (direct mutation)
    assert!(
        rust_code.contains("pub fn update_key(&mut self"),
        "update_key should be &mut self (level 0). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn update_key")).unwrap_or("NOT FOUND")
    );
    
    // ASSERT: poll_keyboard should be &mut self (level 1 transitive)
    assert!(
        rust_code.contains("fn poll_keyboard(&mut self)"),
        "poll_keyboard should be &mut self (level 1). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn poll_keyboard")).unwrap_or("NOT FOUND")
    );
    
    // ASSERT: update should be &mut self (level 2 transitive) - THIS CURRENTLY FAILS
    assert!(
        rust_code.contains("pub fn update(&mut self)"),
        "update should be &mut self (level 2 - calls mutating method). Found:\n{}",
        rust_code.lines().find(|l| l.contains("fn update")).unwrap_or("NOT FOUND")
    );
}

#[test]
fn test_three_level_transitive_mutation() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    
    std::fs::write(
        src.join("game.wj"),
        r#"
pub struct State {
    pub value: i32
}

impl State {
    // Level 0: Direct mutation
    pub fn increment(self) {
        self.value = self.value + 1
    }
}

pub struct Game {
    pub state: State
}

impl Game {
    pub fn new() -> Game {
        Game {
            state: State { value: 0 }
        }
    }
    
    // Level 1: Calls mutating method on field
    fn update_state(self) {
        self.state.increment()
    }
    
    // Level 2: Calls level 1 method
    fn tick(self) {
        self.update_state()
    }
    
    // Level 3: Calls level 2 method
    pub fn run(self) {
        self.tick()
    }
}
"#,
    )
    .unwrap();
    
    build_project(&src.join("game.wj"), &build, CompilationTarget::Rust, false)
        .expect("Build should succeed");
    
    let rust_code = std::fs::read_to_string(build.join("game.rs")).unwrap();
    
    // ALL should be &mut self due to transitive mutation chain
    assert!(
        rust_code.contains("pub fn increment(&mut self)"),
        "increment should be &mut self (level 0)"
    );
    assert!(
        rust_code.contains("fn update_state(&mut self)"),
        "update_state should be &mut self (level 1)"
    );
    assert!(
        rust_code.contains("fn tick(&mut self)"),
        "tick should be &mut self (level 2)"
    );
    assert!(
        rust_code.contains("pub fn run(&mut self)"),
        "run should be &mut self (level 3)"
    );
}
