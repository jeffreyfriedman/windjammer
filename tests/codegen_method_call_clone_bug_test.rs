use std::fs;
/// TDD: Method call results should NEVER get .clone(), even in complex contexts
///
/// Bug: In Pong, `self.left_paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))`
/// generates `.clone()` on the method call results, which is wrong for Copy types (bool).
use std::process::Command;

#[test]
fn test_method_call_no_clone_in_struct_method() {
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
    fn update(&mut self, delta: f32, up: bool, down: bool) {
        // Do something
    }
}

struct Game {
    paddle: Paddle,
}

impl Game {
    fn update(&mut self, delta: f32, input: &Input) {
        // This should NOT generate .clone() on method call results
        self.paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))
    }
}
"#;

    fs::write("test_clone_bug.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_clone_bug.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string("./build/test_clone_bug.rs").expect("Failed to read generated file");

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

    fs::remove_file("test_clone_bug.wj").ok();
}
