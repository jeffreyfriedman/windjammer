// TDD TEST: if-let expressions should not have semicolons in return position
//
// BUG: if-let converted to match expressions are adding semicolons
//
// FIX: Expression-context matches should not add semicolons

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_if_let_in_return_position() {
    let code = r#"
pub fn get_value(opt: Option<i32>) -> i32 {
    if let Some(x) = opt {
        x
    } else {
        0
    }
}

fn main() {
    let v = get_value(Some(42));
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
        !rust_code.contains("x;\n") && !rust_code.contains("0;\n"),
        "if-let in return position should not have semicolons in arms:\n{}",
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

    println!("✅ if-let in return position generates valid code");
}
