use std::fs;
use std::process::Command;

fn setup_wj_build_and_build_dir(wj_code: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let test_root = tempfile::tempdir().expect("tempdir");
    let test_dir = test_root.path();
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, wj_code).expect("write test.wj");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", wj_file.to_str().unwrap()])
        .current_dir(test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let build_dir = test_dir.join("build");
    (test_root, build_dir)
}

#[test]
fn test_for_loop_detects_mutation_needs_mut() {
    // Problem: for (id, val) in self.items where val is mutated → needs &mut
    // User writes: *val = new_value (mutation)
    // Compiler should generate: for (id, val) in &mut self.items
    let wj_code = r#"
struct GameState {
    flags: Vec<(string, bool)>,
}

impl GameState {
    pub fn set_flag(self, flag_id: string, value: bool) {
        for (id, val) in self.flags {
            if *id == flag_id {
                *val = value
                return
            }
        }
    }
}

pub fn main() {
    let mut state = GameState { flags: Vec::new() }
    state.set_flag("test".to_string(), true)
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let rs_file = build_dir.join("test.rs");
    let generated_code = fs::read_to_string(&rs_file).unwrap();

    // Should generate &mut for mutable iteration
    assert!(
        generated_code.contains("for (id, val) in &mut self.flags"),
        "Expected '&mut self.flags' for mutable iteration, got:\n{}",
        generated_code
    );

    let manifest = build_dir.join("Cargo.toml");
    let cargo_output = Command::new("cargo")
        .args(["build", "--manifest-path", manifest.to_str().unwrap()])
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        println!("Generated Rust code:\n{}", generated_code);
        panic!("Cargo build failed:\n{}", stderr);
    }
}

#[test]
fn test_for_loop_readonly_uses_shared_borrow() {
    // Control test: readonly iteration uses &
    let wj_code = r#"
struct GameState {
    flags: Vec<(string, bool)>,
}

impl GameState {
    pub fn has_flag(self, flag_id: string) -> bool {
        for (id, val) in self.flags {
            if *id == flag_id {
                return *val
            }
        }
        false
    }
}

pub fn main() {
    let state = GameState { flags: Vec::new() }
    let result = state.has_flag("test".to_string())
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let rs_file = build_dir.join("test.rs");
    let generated_code = fs::read_to_string(&rs_file).unwrap();

    // Should generate & for readonly iteration
    assert!(
        generated_code.contains("for (id, val) in &self.flags"),
        "Expected '&self.flags' for readonly iteration, got:\n{}",
        generated_code
    );

    let manifest = build_dir.join("Cargo.toml");
    let cargo_output = Command::new("cargo")
        .args(["build", "--manifest-path", manifest.to_str().unwrap()])
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        println!("Generated Rust code:\n{}", generated_code);
        panic!("Cargo build failed:\n{}", stderr);
    }
}
