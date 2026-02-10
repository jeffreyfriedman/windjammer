// TDD Test: Method Calls Inside If Blocks Should Use Dot Syntax
// Bug: Method calls inside if blocks without semicolons generate module syntax
// Root Cause: Expression context in if blocks causes incorrect code generation

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_calls_in_if_blocks() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    // Write Windjammer code matching scene_serializer pattern
    fs::write(
        &test_file,
        r#"fn test_if_method_calls() -> string {
    let mut json = String::from("{")
    
    let pretty = true
    if pretty {
        json.push_str("\n  ")
    }
    
    json.push_str("test")
    json
}
"#,
    )
    .unwrap();

    // Run wj build with locally built binary
    let wj_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj");

    let output = Command::new(&wj_binary)
        .args(["build", test_file.to_str().unwrap(), "--no-cargo"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("wj build failed");
    }

    // Read generated code
    let rust_file = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Failed to read generated Rust");

    println!("Generated Rust code:\n{}", rust_code);

    // THE BUG: Method calls in if blocks generate json::push_str instead of json.push_str
    assert!(
        !rust_code.contains("json::push_str"),
        "BUG FOUND! Generated code uses json::push_str (module syntax) instead of json.push_str (method call). \
         This happens when method calls are inside if blocks without semicolons.\n\
         Generated code:\n{}",
        rust_code
    );

    // Verify correct syntax
    assert!(
        rust_code.contains("json.push_str"),
        "Method calls should use dot syntax"
    );
}
