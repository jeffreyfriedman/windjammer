#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
/// TDD: Method call results should NEVER get .clone(), even in complex contexts
///
/// Bug: In Pong, `self.left_paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))`
/// generates `.clone()` on the method call results, which is wrong for Copy types (bool).
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_no_clone_in_struct_method() {
    let source = r#"
enum Key {
    W,
    S,
}

struct Input {}

impl Input {
    fn is_key_down(self, key: Key) -> bool {
        true
    }
}

struct Paddle {}

impl Paddle {
    fn update(self, delta: f32, up: bool, down: bool) {
        // Do something
    }
}

struct Game {
    paddle: Paddle,
}

impl Game {
    fn update(self, delta: f32, input: Input) {
        // This should NOT generate  on method call results
        self.paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test_clone_bug.wj");
    let output_dir = temp_dir.path().join("build");
    fs::write(&input_path, source).unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            input_path.to_str().unwrap(),
            "--no-cargo",
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(output_dir.join("test_clone_bug.rs"))
        .expect("Failed to read generated file");

    println!("Generated code:\n{}", generated);

    // Should NOT have .clone() on method call results
    assert!(
        !generated.contains("input.is_key_down(Key::W).clone()"),
        "Should not clone method call result (bool is Copy)"
    );
    assert!(
        !generated.contains("input.is_key_down(Key::S).clone()"),
        "Should not clone method call result (bool is Copy)"
    );
}
