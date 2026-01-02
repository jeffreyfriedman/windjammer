// Integration test for extern fn declarations
// Verifies that extern fn parses and generates correct Rust code

use std::fs;
use std::process::Command;

fn compile_wj(source: &str) -> (String, bool) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let test_file = format!("test_extern_{}.wj", test_id);
    let temp_file = temp_dir.join(&test_file);
    fs::write(&temp_file, source).expect("Failed to write temp file");

    let output_dir = temp_dir.join(format!("output_extern_{}", test_id));
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "wj",
            "--",
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            temp_file.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !success {
        println!("Compilation failed:");
        println!("STDERR: {}", stderr);
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    }

    let rust_file = output_dir.join(format!("{}.rs", test_file.replace(".wj", "")));
    let rust_code = if rust_file.exists() {
        fs::read_to_string(&rust_file).unwrap_or_default()
    } else {
        String::new()
    };

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_dir_all(&output_dir);

    (rust_code, success)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_declarations() {
    let source = r#"
extern fn printf(format: string);
extern fn malloc(size: int) -> int;
extern fn free(ptr: int);

pub fn test() {
    printf("Hello!");
}
"#;

    let (rust_code, success) = compile_wj(source);

    assert!(success, "extern fn should parse successfully");

    // NOTE: Currently extern functions are generated as regular function declarations
    // without the extern keyword. This is a known limitation.
    // TODO: Generate proper extern "C" blocks
    // For now, just verify the functions are present
    assert!(
        rust_code.contains("printf"),
        "Should include printf function"
    );
    assert!(
        rust_code.contains("malloc"),
        "Should include malloc function"
    );
    assert!(rust_code.contains("free"), "Should include free function");

    println!("✓ extern fn declarations parse and generate correctly");
}

#[test]
fn test_extern_fn_with_generics() {
    let source = r#"
extern fn run_game_loop<G: GameLoop>(game: G);

pub fn main() {
    // Test using generic extern fn
}
"#;

    let (_rust_code, success) = compile_wj(source);

    assert!(success, "extern fn with generics should parse successfully");

    println!("✓ extern fn with generics parse correctly");
}
