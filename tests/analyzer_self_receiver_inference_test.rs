/// TDD Test: Self receiver inference for nested mutations
///
/// Bug: Methods calling mutating methods on fields need &mut self inference.
/// - self.factions[i].adjust_reputation() - indexed field method call
/// - self.companion.adjust_loyalty() - field method call
/// - handle_player_input(self.game.player, dt) - passing field to mutating function
///
/// Root Cause: expression_traces_to_self didn't handle Index; Call case
/// didn't check when self.field is passed to function expecting &mut.
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

#[test]
fn test_method_calling_mutating_method_on_indexed_field() {
    let code = r#"
pub struct Item {
    pub value: i32,
}

impl Item {
    pub fn increment(self) {
        self.value += 1
    }
}

pub struct Container {
    pub items: Vec<Item>,
}

impl Container {
    pub fn increment_item(self, index: usize) {
        self.items[index].increment()
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Compilation should succeed");

    // Should infer &mut self because calling mutating method on indexed field
    assert!(
        rust.contains("pub fn increment_item(&mut self, index: usize)"),
        "increment_item should infer &mut self (calls mutating method on self.items[index])\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_method_passing_field_as_mut_ref() {
    let code = r#"
pub struct Player {
    pub x: f32,
}

pub fn update_position(player: Player, dt: f32) {
    player.x = player.x + dt
}

pub struct Game {
    pub player: Player,
}

impl Game {
    pub fn tick(self, dt: f32) {
        update_position(self.player, dt)
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Compilation should succeed");

    // Should infer &mut self because passing field to mutating function
    assert!(
        rust.contains("pub fn tick(&mut self, dt: f32)"),
        "tick should infer &mut self (passes self.player to mutating function)\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_method_mutating_vec_element() {
    let code = r#"
pub struct List {
    pub items: Vec<i32>,
}

impl List {
    pub fn increment_at(self, index: usize) {
        self.items[index] = self.items[index] + 1
    }
}
"#;

    let rust = compile_windjammer_code(code).expect("Compilation should succeed");

    // Should infer &mut self (direct indexed assignment)
    assert!(
        rust.contains("pub fn increment_at(&mut self, index: usize)"),
        "increment_at should infer &mut self\nGenerated:\n{}",
        rust
    );
}
