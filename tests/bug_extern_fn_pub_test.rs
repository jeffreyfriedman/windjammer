use std::fs;
/// TDD test: extern fn declarations should generate `pub fn` inside `extern "C"` blocks
///
/// Bug: extern fn declarations generated as private, making them inaccessible
/// from other modules via `pub use module::*;` re-exports.
///
/// Root Cause: generate_extern_function() emitted `fn name(...)` without `pub`.
///
/// Fix: Emit `pub fn name(...)` for extern function declarations.
use std::process::Command;

fn transpile_wj(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let _output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    fs::read_to_string(&rust_file).expect("Failed to read generated Rust file")
}

#[test]
fn test_extern_fn_generates_pub() {
    let source = r#"
extern fn do_something(x: i32, y: i32) -> i32
extern fn do_nothing()
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("extern \"C\""),
        "Should generate extern C block"
    );
    assert!(
        generated.contains("pub fn do_something("),
        "extern fn should generate pub fn, got:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn do_nothing("),
        "extern fn should generate pub fn, got:\n{}",
        generated
    );
}
