#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

use std::process::Command;
use std::{fs, path::Path};

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
fn test_explicit_deref_borrowed_iter_vs_owned_param() {
    // Case: *id == flag_id where id is &String (borrowed iter), flag_id is String (owned param NOT inferred as borrowed)
    // Source declares: flag_id: string (not borrowed in declaration)
    // Generated: flag_id: String (owned, because inference didn't mark it borrowed)
    // Expected: Keep * or add to other side → *id == flag_id (String == String)
    let wj_code = r#"
struct GameState {
    custom_flags: Vec<(string, bool)>,
}

impl GameState {
    pub fn check_flag(self, flag_id: string) -> bool {
        for (id, val) in self.custom_flags {
            if *id == flag_id {
                return val
            }
        }
        false
    }
}

pub fn main() {
    let state = GameState { custom_flags: Vec::new() }
    let result = state.check_flag("test".to_string())
}
"#;

    let (_root, build_dir) = setup_wj_build_and_build_dir(wj_code);

    let manifest = build_dir.join("Cargo.toml");
    let cargo_output = Command::new("cargo")
        .args(["build", "--manifest-path", manifest.to_str().unwrap()])
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        let rs_file = build_dir.join("test.rs");
        if Path::new(&rs_file).exists() {
            let generated_code = fs::read_to_string(&rs_file).unwrap();
            println!("Generated Rust code:\n{}", generated_code);
        }
        panic!("Cargo build failed:\n{}", stderr);
    }
}
