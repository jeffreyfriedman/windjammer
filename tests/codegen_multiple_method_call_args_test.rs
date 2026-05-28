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
/// TDD: Multiple method call results as arguments should not be borrowed/cloned
///
/// Bug: When passing multiple method call results (that return Copy types) as arguments,
/// the compiler incorrectly generates borrows/clones.
///
/// Example from Pong:
///   paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))
/// Generates:
///   paddle.update(delta, &input.is_key_down(Key::W).clone(), input.is_key_down(Key::S).clone())
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_bool_method_call_args() {
    let source = r#"
enum Key {
    W,
    S,
}

struct Input {}

impl Input {
    fn is_key_down(&self, key: Key) -> bool {
        true
    }
}

struct Paddle {}

impl Paddle {
    fn update(&mut self, up: bool, down: bool) {
        // Do something
    }
}

fn test_function() {
    let mut paddle = Paddle {}
    let input = Input {}
    
    paddle.update(input.is_key_down(Key::W), input.is_key_down(Key::S))
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test_multi_bool.wj");
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

    let generated = fs::read_to_string(output_dir.join("test_multi_bool.rs"))
        .expect("Failed to read generated file");

    println!("Generated code:\n{}", generated);

    // Should NOT have unnecessary borrows
    assert!(
        !generated.contains("&input.is_key_down"),
        "Should not borrow Copy type method result"
    );

    // Should NOT have unnecessary clones
    assert!(
        !generated.contains(".clone()"),
        "Should not clone Copy type (bool)"
    );
}
