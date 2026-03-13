/// TDD Test: Method call mut propagation - calling mutating methods on fields
///
/// Bug: Calling self.field.mutating_method() doesn't propagate &mut self inference.
/// Method A calls self.field.mutating_method() → method A needs &mut self
/// But analyzer generates &self instead!
///
/// Root Cause: Analyzer checks direct field mutation (self.field = ...)
/// but doesn't properly check method calls on fields that require &mut
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = test_dir.join("build/test.rs");
    let generated =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");
    Ok(generated)
}

fn verify_rust_compiles(rust_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let rust_file = test_dir.join("test.rs");
    std::fs::write(&rust_file, rust_code).expect("Failed to write Rust file");

    let check = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            rust_file.to_str().unwrap(),
            "-o",
            test_dir.join("test.rlib").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if check.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&check.stderr).to_string())
    }
}

#[test]
fn test_method_calling_field_mutating_method() {
    // Exact pattern from breach-protocol faction.wj
    let code = r#"
pub struct Faction {
    pub reputation: f32,
}

impl Faction {
    pub fn adjust_reputation(self, amount: f32) {
        self.reputation = self.reputation + amount
    }
}

pub struct FactionManager {
    pub factions: Vec<Faction>,
}

impl FactionManager {
    pub fn adjust_reputation(self, index: i32, amount: f32) {
        self.factions[index].adjust_reputation(amount)
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(
        result.is_ok(),
        "Calling mutating method on field should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn adjust_reputation(&mut self, index: i32"),
        "FactionManager::adjust_reputation should have &mut self. Generated:\n{}",
        rust
    );

    // Verify generated Rust compiles
    let verify = verify_rust_compiles(&rust);
    assert!(
        verify.is_ok(),
        "Generated Rust should compile:\n{}",
        verify.err().unwrap_or_default()
    );
}

#[test]
fn test_nested_field_method_call() {
    let code = r#"
pub struct Player {
    pub health: f32,
}

impl Player {
    pub fn damage(self, amount: f32) {
        self.health = self.health - amount
    }
}

pub struct Game {
    pub player: Player,
}

impl Game {
    pub fn damage_player(self, amount: f32) {
        self.player.damage(amount)
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(
        result.is_ok(),
        "Nested field method call should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn damage_player(&mut self, amount: f32)"),
        "Game::damage_player should have &mut self. Generated:\n{}",
        rust
    );
}

#[test]
fn test_vec_push_on_field() {
    let code = r#"
pub struct EventLog {
    pub events: Vec<string>,
}

impl EventLog {
    pub fn log(self, message: string) {
        self.events.push(message)
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(
        result.is_ok(),
        "Vec::push on field should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn log(&mut self,"),
        "EventLog::log should have &mut self. Generated:\n{}",
        rust
    );
}

#[test]
fn test_breach_protocol_faction_pattern() {
    // Exact pattern from breach-protocol: while loop with if, calling adjust_reputation
    let code = r#"
pub enum FactionId {
    A,
    B,
}

impl FactionId {
    pub fn equals(self, other: FactionId) -> bool {
        true
    }
}

pub struct Faction {
    pub id: FactionId,
    pub reputation: f32,
}

impl Faction {
    pub fn adjust_reputation(self, amount: f32) {
        self.reputation = self.reputation + amount
    }
}

pub struct FactionSystem {
    pub factions: Vec<Faction>,
}

impl FactionSystem {
    pub fn adjust_reputation(self, id: FactionId, amount: f32) {
        let mut i = 0
        while i < self.factions.len() {
            if self.factions[i].id.equals(id) {
                self.factions[i].adjust_reputation(amount)
                return
            }
            i = i + 1
        }
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(
        result.is_ok(),
        "Breach-protocol faction pattern should compile:\n{}",
        result.err().unwrap_or_default()
    );

    let rust = result.unwrap();
    assert!(
        rust.contains("pub fn adjust_reputation(&mut self, id: FactionId, amount: f32)"),
        "FactionSystem::adjust_reputation should have &mut self. Generated:\n{}",
        rust
    );
}
