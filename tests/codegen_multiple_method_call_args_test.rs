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

    fs::write("test_multi_bool.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_multi_bool.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated =
        fs::read_to_string("./build/test_multi_bool.rs").expect("Failed to read generated file");

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

    fs::remove_file("test_multi_bool.wj").ok();
}
