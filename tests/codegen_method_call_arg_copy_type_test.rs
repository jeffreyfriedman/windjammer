use std::fs;
/// TDD: Method call results that are Copy types should not be borrowed/cloned
///
/// Bug: When passing `input.is_key_down(Key::W)` (returns bool) as an argument,
/// the compiler generates `&input.is_key_down(Key::W).clone()` instead of just
/// passing the bool directly.
///
/// This is wrong because:
/// 1. bool is Copy, so no ownership transfer issues
/// 2. The function expects `bool`, not `&bool`
/// 3. `.clone()` is unnecessary for Copy types
///
/// Root cause: The analyzer is over-eagerly borrowing method call results
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_bool_arg_not_borrowed() {
    let source = r#"
struct Input {}

impl Input {
    fn is_key_down(&self) -> bool {
        true
    }
}

struct Paddle {}

impl Paddle {
    fn update(&mut self, value: bool) {
        // Do something with value
    }
}

fn test_function() {
    let mut paddle = Paddle {}
    let input = Input {}
    
    // This should generate: paddle.update(input.is_key_down())
    // NOT: paddle.update(&input.is_key_down().clone())
    paddle.update(input.is_key_down())
}
"#;

    fs::write("test_input.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_input.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(output.status.success(), "Compilation failed");

    let generated =
        fs::read_to_string("./build/test_input.rs").expect("Failed to read generated file");

    // Should NOT have unnecessary borrow
    assert!(
        !generated.contains("&input.is_key_down()"),
        "Should not borrow Copy type method result"
    );

    // Should NOT have unnecessary clone
    assert!(
        !generated.contains(".clone()"),
        "Should not clone Copy type (bool)"
    );

    // Should have direct call
    assert!(
        generated.contains("paddle.update(input.is_key_down())"),
        "Should pass bool directly without borrow or clone"
    );

    fs::remove_file("test_input.wj").ok();
}
