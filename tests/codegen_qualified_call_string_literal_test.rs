/// Test: String literals in qualified module::function calls should NOT get .to_string()
///
/// Bug: When calling draw::draw_text("HELLO", 100.0, 200.0), the codegen
/// incorrectly adds .to_string() to the string literal argument.
///
/// Root cause: The signature lookup falls back from qualified "draw::draw_text"
/// to simple "draw_text". If a DIFFERENT module has a function also named
/// "draw_text" with different param_ownership (Owned vs Borrowed), the wrong
/// signature is used. The fallback should only apply to Type::method patterns
/// (CamelCase prefix), not module::function patterns (lowercase prefix).
///
/// Generated code:
///   draw::draw_text("HELLO".to_string(), 100.0_f32, 200.0_f32)
///
/// Expected code:
///   draw::draw_text("HELLO", 100.0_f32, 200.0_f32)
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_in_qualified_call_no_to_string() {
    let temp_dir = TempDir::new().unwrap();

    // Create the draw module with a function taking string param
    let draw_source = r#"
pub fn draw_text(text: string, x: f32, y: f32) {
    println!("{} at ({}, {})", text, x, y)
}
"#;

    // Create the game module that calls draw::draw_text with a string literal
    let game_source = r#"
use crate::draw

pub struct Game {
    pub active: bool,
}

impl Game {
    pub fn render(self) {
        draw::draw_text("HELLO WORLD", 100.0, 200.0)
    }
}
"#;

    // Write files
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("draw.wj"), draw_source).unwrap();
    fs::write(src_dir.join("game.wj"), game_source).unwrap();

    // Compile all files together
    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Check generated game.rs
    let game_rs = temp_dir.path().join("build").join("game.rs");
    let generated = fs::read_to_string(&game_rs).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated game.rs. Build output:\n{}",
            String::from_utf8_lossy(&wj_output.stdout)
        )
    });

    // String literal "HELLO WORLD" should NOT get .to_string() when passed to
    // a function with a borrowed string parameter (text: string → &str in Rust)
    assert!(
        !generated.contains(r#""HELLO WORLD".to_string()"#),
        "String literal should NOT get .to_string() for borrowed string param.\nGenerated:\n{}",
        generated
    );

    // Should contain the string literal passed directly
    assert!(
        generated.contains(r#""HELLO WORLD""#),
        "String literal should be passed directly.\nGenerated:\n{}",
        generated
    );
}

/// Test: When compiling a single file that uses a module-qualified call
/// (draw::draw_text), and .wj.meta files provide a conflicting signature
/// for "draw_text" with Owned ownership, the qualified call should NOT
/// blindly use the unqualified fallback signature.
///
/// This reproduces the real bug: when compiling breach-protocol/src/game.wj,
/// windjammer-game's api.wj.meta has draw_text with Owned, but the actual
/// draw module uses Borrowed. The simple-name fallback picks up the wrong one.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_single_file_qualified_call_with_conflicting_metadata() {
    let temp_dir = TempDir::new().unwrap();

    // Single file that calls draw::draw_text with a string literal
    let game_source = r#"
use crate::draw

pub fn render() {
    draw::draw_text("HELLO", 100.0, 200.0)
}
"#;

    // Conflicting .wj.meta with draw_text having Owned string param
    // (simulates windjammer-game's api.wj.meta loading)
    let conflicting_meta = r#"{
  "module_path": "rendering_api",
  "functions": {
    "draw_text": {
      "params": ["String", "Custom(\"f32\")", "Custom(\"f32\")"],
      "return_type": null,
      "is_associated": false,
      "parent_type": null,
      "param_ownership": ["Owned", "Owned", "Owned"],
      "has_self_receiver": false
    }
  },
  "structs": {},
  "trait_impls": {},
  "copy_structs": [],
  "version": "0.46.0"
}"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("game.wj"), game_source).unwrap();
    // Place conflicting metadata in source dir so the compiler finds it
    fs::write(src_dir.join("rendering_api.wj.meta"), conflicting_meta).unwrap();

    // Compile SINGLE FILE (not directory), which forces single-file path
    // that relies on metadata for cross-module signatures
    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/game.wj")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    let game_rs = temp_dir.path().join("build").join("game.rs");
    let generated = fs::read_to_string(&game_rs).unwrap_or_else(|_| {
        panic!("Failed to read generated game.rs")
    });

    // The qualified call draw::draw_text("HELLO", ...) should NOT get
    // .to_string() from the conflicting metadata's Owned signature.
    // When the qualifier is a module (lowercase), the simple-name fallback
    // should NOT be used since it may match a different module's function.
    assert!(
        !generated.contains(r#""HELLO".to_string()"#),
        "String literal should NOT get .to_string() from wrong module's metadata.\n\
         The simple-name fallback 'draw_text' matched rendering_api's signature (Owned),\n\
         but the call is draw::draw_text which is a different module.\n\
         Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_in_type_qualified_call_with_owned_param() {
    let temp_dir = TempDir::new().unwrap();

    // Create a struct with a constructor that takes an owned String
    let source = r#"
pub struct Label {
    pub text: String,
}

impl Label {
    pub fn new(text: String) -> Label {
        Label { text: text }
    }
}

pub fn make_label() -> Label {
    Label::new("Hello")
}
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("label.wj"), source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    let label_rs = temp_dir.path().join("build").join("label.rs");
    let generated = fs::read_to_string(&label_rs).unwrap_or_else(|_| {
        panic!("Failed to read generated label.rs")
    });

    // Label::new("Hello") should get .to_string() because the param is owned String
    assert!(
        generated.contains(r#""Hello".to_string()"#),
        "String literal should get .to_string() for owned String param.\nGenerated:\n{}",
        generated
    );
}
