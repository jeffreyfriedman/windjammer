use anyhow::Result;
/// TDD Test: Array Literal Codegen - Fixed Array vs Vec
///
/// PROBLEM: When WJ source uses `[a, b]` array literal syntax (not `vec![]`),
/// the compiler generates `vec![a, b]` which creates a `Vec<T>`. But when the
/// target expects a fixed-size array `[T; N]` (e.g., `painter.line_segment([p1, p2], stroke)`),
/// this causes Rust E0308: "expected `[Pos2; 2]`, found `Vec<Pos2>`".
///
/// FIX: Generate fixed-size array `[a, b]` for `Expression::Array` literals (not `vec![...]`).
/// The `vec![]` macro is a separate WJ syntax (`MacroInvocation`) and should still produce `vec![]`.
///
/// KEY INSIGHT: In WJ, `[a, b]` is a fixed-size array literal → `[a, b]` in Rust.
///              In WJ, `vec![a, b]` is an explicit Vec constructor → `vec![a, b]` in Rust.
///
/// Example WJ source:
/// ```
/// painter.line_segment([center, pos], stroke)
/// ```
///
/// Should generate:
/// ```rust
/// painter.line_segment([center, pos], stroke);
/// ```
///
/// Should NOT generate:
/// ```rust
/// painter.line_segment(vec![center, pos], stroke);  // ❌ E0308!
/// ```
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_literal_generates_fixed_array_not_vec() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_array_literal_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write a WJ file that uses array literal syntax [a, b]
    // This should generate [a, b] NOT vec![a, b]
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn make_pair(a: i32, b: i32) -> [i32; 2] {
    [a, b]
}

fn use_array() {
    let pair = make_pair(1, 2)
    let arr = [10, 20, 30]
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "array-literal-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile with --no-run-cargo to just generate Rust
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Read the generated Rust file
    let generated = fs::read_to_string(output_dir.join("main.rs")).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated main.rs\nstdout: {}\nstderr: {}",
            stdout, stderr
        )
    });

    // ASSERTION 1: [a, b] should generate fixed-size array [a, b], NOT vec![a, b]
    assert!(
        !generated.contains("vec![a, b]"),
        "Array literal [a, b] should NOT generate vec![a, b].\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("[a, b]"),
        "Array literal [a, b] should generate fixed-size array [a, b].\nGenerated:\n{}",
        generated
    );

    // ASSERTION 2: [10, 20, 30] should also generate fixed-size array
    assert!(
        !generated.contains("vec![10, 20, 30]"),
        "Array literal [10, 20, 30] should NOT generate vec![10, 20, 30].\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("[10, 20, 30]"),
        "Array literal [10, 20, 30] should generate fixed-size array [10, 20, 30].\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_macro_still_generates_vec() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_vec_macro_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write a WJ file that uses vec![] macro syntax
    // This should still generate vec![]
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn make_vec() -> Vec<i32> {
    vec![1, 2, 3]
}

fn use_vec() {
    let mut items = vec![10, 20, 30]
    items.push(40)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "vec-macro-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile with --no-run-cargo to just generate Rust
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Read the generated Rust file
    let generated = fs::read_to_string(output_dir.join("main.rs")).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated main.rs\nstdout: {}\nstderr: {}",
            stdout, stderr
        )
    });

    // ASSERTION: vec![1, 2, 3] should still generate vec![1, 2, 3]
    assert!(
        generated.contains("vec![1, 2, 3]"),
        "vec![] macro should still generate vec![].\nGenerated:\n{}",
        generated
    );

    // ASSERTION: vec![10, 20, 30] should still generate vec![10, 20, 30]
    assert!(
        generated.contains("vec![10, 20, 30]"),
        "vec![] macro should still generate vec![].\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
