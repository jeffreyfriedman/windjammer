// TDD TEST: if-let with String return should not have semicolons
//
// BUG: Match arms returning String values are getting semicolons
//
// FIX: Check return type correctly

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_if_let_string_return() {
    let code = r#"
pub fn get_name(opt: Option<string>) -> string {
    if let Some(name) = opt {
        name
    } else {
        "Unknown".to_string()
    }
}

fn main() {
    let n = get_name(Some("Alice".to_string()));
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .output()
        .expect("Failed to run wj");

    assert!(
        result.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Read generated Rust
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // Should NOT have semicolons in match arms (expression context)
    assert!(
        !rust_code.contains("name;\n")
            && (!rust_code.contains("\"Unknown\".to_string();")
                || !rust_code.contains(".to_string();\n")),
        "if-let returning String should not have semicolons in arms:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_result = Command::new("rustc")
        .current_dir(output_dir)
        .arg("--crate-type")
        .arg("bin")
        .arg("test.rs")
        .output()
        .expect("Failed to run rustc");

    if !compile_result.status.success() {
        let stderr = String::from_utf8_lossy(&compile_result.stderr);
        panic!(
            "Generated code failed to compile:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    println!("✅ if-let with String return generates valid code");
}
