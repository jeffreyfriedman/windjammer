/// TDD test: Basic WGSL compilation
///
/// Tests that the WGSL backend can compile a simple function to WGSL.
use std::fs;
use std::process::Command;

fn transpile_wj_to_wgsl(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_wgsl_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    // Use CARGO_BIN_EXE_wj for cross-platform compatibility
    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("wgsl")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    // Check compilation status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let wgsl_file = out_dir.join("test.wgsl");
    let content = fs::read_to_string(&wgsl_file).expect("Failed to read generated WGSL file");

    // Clean up temp directory
    let _ = fs::remove_dir_all(&test_dir);

    content
}

#[test]
fn test_simple_add_function() {
    let source = r#"
pub fn add(x: uint, y: uint) -> uint {
    x + y
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check that the function was generated
    assert!(
        generated.contains("fn add"),
        "Generated WGSL should contain 'fn add'. Got:\n{}",
        generated
    );
    
    // Check parameters
    assert!(
        generated.contains("x: u32"),
        "Should have u32 parameter. Got:\n{}",
        generated
    );
    
    // Check return type
    assert!(
        generated.contains("-> u32"),
        "Should have u32 return type. Got:\n{}",
        generated
    );
    
    // Check function body
    assert!(
        generated.contains("return"),
        "Should have return statement. Got:\n{}",
        generated
    );
}

#[test]
fn test_primitive_types() {
    let source = r#"
pub fn test_types(a: uint, b: int32, c: float, d: bool) -> float {
    c
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check type mappings
    assert!(generated.contains("a: u32"));
    assert!(generated.contains("b: i32"));
    assert!(generated.contains("c: f32"));
    assert!(generated.contains("d: bool"));
    assert!(generated.contains("-> f32"));
}

#[test]
fn test_binary_operations() {
    let source = r#"
pub fn test_ops(x: uint, y: uint) -> uint {
    let sum = x + y
    let diff = x - y
    let prod = x * y
    let quot = x / y
    sum
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check operations are generated
    assert!(generated.contains("+"));
    assert!(generated.contains("-"));
    assert!(generated.contains("*"));
    assert!(generated.contains("/"));
    
    // Check let statements
    assert!(generated.contains("let sum"));
    assert!(generated.contains("let diff"));
    assert!(generated.contains("let prod"));
    assert!(generated.contains("let quot"));
}

#[test]
fn test_if_statement() {
    let source = r#"
pub fn max(x: uint, y: uint) -> uint {
    if x > y {
        x
    } else {
        y
    }
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check if/else structure
    assert!(generated.contains("if"));
    assert!(generated.contains("else"));
    assert!(generated.contains(">"));
}

#[test]
fn test_while_loop() {
    let source = r#"
pub fn count(n: uint) -> uint {
    let mut i = 0
    while i < n {
        i = i + 1
    }
    i
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check while loop
    assert!(generated.contains("while"));
    assert!(generated.contains("<"));
}
