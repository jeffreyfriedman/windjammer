use std::fs;
/// TDD: Method call args from trait implementations should not be borrowed/cloned
///
/// Bug: When calling methods with Copy-type arguments from within a trait implementation,
/// the compiler incorrectly generates borrows/clones.
///
/// This is the actual Pong bug context.
use std::process::Command;

#[test]
fn test_trait_impl_method_call_bool_args() {
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

trait GameLoop {
    fn update(&mut self, delta: f32, input: &Input);
}

struct Game {
    paddle: Paddle,
}

impl GameLoop for Game {
    fn update(&mut self, delta: f32, input: &Input) {
        // This is the context where the bug occurs
        self.paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))
    }
}
"#;

    fs::write("test_trait_impl.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_trait_impl.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string("./build/test_trait_impl.rs").expect("Failed to read generated file");

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

    fs::remove_file("test_trait_impl.wj").ok();
}
