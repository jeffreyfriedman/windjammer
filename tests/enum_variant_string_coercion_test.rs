//! TDD Test: Enum variant constructors with string literal arguments
//!
//! Bug: When constructing an enum variant that takes a String field with a
//! string literal (e.g., GameEvent::ItemPickup("Health Potion")), the codegen
//! generates `GameEvent::ItemPickup("Health Potion")` without adding `.to_string()`.
//! Rust expects `String`, not `&str`.
//!
//! Expected: `GameEvent::ItemPickup("Health Potion".to_string())`
//! Actual:   `GameEvent::ItemPickup("Health Potion")`

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_enum_variant_string_literal_gets_to_string() {
    let source = r#"
enum Message {
    Text(string),
    Error(string),
    Quit,
}

fn main() {
    let msg = Message::Text("Hello world")
    let err = Message::Error("Something failed")
    let quit = Message::Quit
    println("done")
}
"#;

    let (rust_code, compiles) = test_utils::compile_single_check(source);

    println!("Generated Rust:\n{}", rust_code);

    // The generated code should convert string literals to String for enum variants
    assert!(
        rust_code.contains(r#""Hello world".to_string()"#),
        "Expected string literal to get .to_string() in enum variant constructor.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains(r#""Something failed".to_string()"#),
        "Expected string literal to get .to_string() in enum variant constructor.\nGenerated:\n{}",
        rust_code
    );

    assert!(compiles, "Generated Rust should compile successfully");
}

#[test]
fn test_enum_variant_mixed_types_string_coercion() {
    let source = r#"
enum GameEvent {
    PlayerMove(f32, f32),
    PlayerAttack(i32),
    ItemPickup(string),
    ChatMessage(string, string),
    None,
}

fn main() {
    let events: Vec<GameEvent> = vec![
        GameEvent::PlayerMove(1.0, 2.0),
        GameEvent::PlayerAttack(50),
        GameEvent::ItemPickup("Sword"),
        GameEvent::ChatMessage("Alice", "Hello"),
        GameEvent::None,
    ]
    println("done")
}
"#;

    let (rust_code, compiles) = test_utils::compile_single_check(source);

    println!("Generated Rust:\n{}", rust_code);

    // String literals in enum variants should get .to_string()
    assert!(
        rust_code.contains(r#""Sword".to_string()"#),
        "ItemPickup string should get .to_string()\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains(r#""Alice".to_string()"#),
        "ChatMessage first arg should get .to_string()\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains(r#""Hello".to_string()"#),
        "ChatMessage second arg should get .to_string()\nGenerated:\n{}",
        rust_code
    );

    // Non-string args should NOT get .to_string()
    assert!(
        !rust_code.contains("1.0.to_string()"),
        "f32 should NOT get .to_string()"
    );
    assert!(
        !rust_code.contains("50.to_string()"),
        "i32 should NOT get .to_string()"
    );

    assert!(compiles, "Generated Rust should compile successfully");
}
