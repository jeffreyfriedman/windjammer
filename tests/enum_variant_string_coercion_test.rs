//! TDD Test: Enum variant constructors with string literal arguments
//!
//! Bug: When constructing an enum variant that takes a String field with a
//! string literal (e.g., GameEvent::ItemPickup("Health Potion")), the codegen
//! generates `GameEvent::ItemPickup("Health Potion")` without adding `.to_string()`.
//! Rust expects `String`, not `&str`.
//!
//! Expected: `GameEvent::ItemPickup("Health Potion".to_string())`
//! Actual:   `GameEvent::ItemPickup("Health Potion")`

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_dir(prefix: &str) -> std::path::PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("wj-test-enum-str-{}-{}-{}", prefix, pid, id))
}

fn compile_wj_to_rust(wj_source: &str, test_name: &str) -> (String, bool) {
    let input_dir = unique_dir(test_name);
    let output_dir = unique_dir(&format!("{}-out", test_name));
    std::fs::create_dir_all(&input_dir).unwrap();

    let wj_file = input_dir.join("test.wj");
    std::fs::write(&wj_file, wj_source).unwrap();

    let wj_binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let _output = Command::new(&wj_binary)
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj compiler");

    let rs_file = output_dir.join("test.rs");
    let rust_code = std::fs::read_to_string(&rs_file).unwrap_or_default();

    // Try to compile with rustc
    let compiles = if !rust_code.is_empty() {
        let bin_output = output_dir.join("test_bin");
        let rustc_output = Command::new("rustc")
            .args([
                "--edition",
                "2021",
                "--crate-type",
                "bin",
                rs_file.to_str().unwrap(),
                "-o",
                bin_output.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to run rustc");
        if !rustc_output.status.success() {
            eprintln!(
                "rustc stderr: {}",
                String::from_utf8_lossy(&rustc_output.stderr)
            );
        }
        rustc_output.status.success()
    } else {
        false
    };

    // Cleanup
    let _ = std::fs::remove_dir_all(&input_dir);
    let _ = std::fs::remove_dir_all(&output_dir);

    (rust_code, compiles)
}

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

    let (rust_code, compiles) = compile_wj_to_rust(source, "enum-str-basic");

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

    let (rust_code, compiles) = compile_wj_to_rust(source, "enum-str-mixed");

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
