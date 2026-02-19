use anyhow::Result;
/// TDD Test: Optimize .len() == 0 comparisons to .is_empty()
///
/// PROBLEM: Clippy warns about `.len() == 0` and `.len() != 0` patterns.
/// These should be replaced with `.is_empty()` and `!.is_empty()` respectively.
///
/// WINDJAMMER PHILOSOPHY: Generate idiomatic Rust that passes Clippy without warnings.
///
/// Example WJ source:
/// ```
/// if items.len() == 0 { }
/// ```
///
/// Should generate:
/// ```rust
/// if items.is_empty() { }
/// ```
///
/// Should NOT generate:
/// ```rust
/// if items.len() == 0 { }  // ⚠️  Clippy: len_zero
/// ```
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_len_eq_zero_becomes_is_empty() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_len_zero_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with len() == 0 comparison
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn check_empty(items: Vec<i32>) -> bool {
    if items.len() == 0 {
        return true
    }
    false
}

fn main() {
    let empty = Vec::new()
    let result = check_empty(empty)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "len-zero-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile with --no-cargo to just generate Rust
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

    // ASSERTION 1: Should use .is_empty() instead of .len() == 0
    assert!(
        generated.contains(".is_empty()"),
        ".len() == 0 should be optimized to .is_empty()\nGenerated:\n{}",
        generated
    );

    // ASSERTION 2: Should NOT contain .len() == 0
    assert!(
        !generated.contains(".len() == 0"),
        "Should NOT generate .len() == 0 (Clippy warning)\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_len_ne_zero_becomes_not_is_empty() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_len_ne_zero_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with len() != 0 comparison
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn has_items(items: Vec<i32>) -> bool {
    items.len() != 0
}

fn main() {
    let items = vec![1, 2, 3]
    let result = has_items(items)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "len-ne-zero-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let generated = fs::read_to_string(output_dir.join("main.rs")).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated main.rs\nstdout: {}\nstderr: {}",
            stdout, stderr
        )
    });

    // ASSERTION 1: Should use !.is_empty() instead of .len() != 0
    assert!(
        generated.contains("!") && generated.contains(".is_empty()"),
        ".len() != 0 should be optimized to !.is_empty()\nGenerated:\n{}",
        generated
    );

    // ASSERTION 2: Should NOT contain .len() != 0
    assert!(
        !generated.contains(".len() != 0"),
        "Should NOT generate .len() != 0 (Clippy warning)\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_len_gt_zero_becomes_not_is_empty() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_len_gt_zero_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with len() > 0 comparison
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn has_items(items: Vec<i32>) -> bool {
    items.len() > 0
}

fn main() {
    let items = vec![1, 2, 3]
    let result = has_items(items)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "len-gt-zero-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let generated = fs::read_to_string(output_dir.join("main.rs")).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated main.rs\nstdout: {}\nstderr: {}",
            stdout, stderr
        )
    });

    // ASSERTION 1: Should use !.is_empty() instead of .len() > 0
    assert!(
        generated.contains("!") && generated.contains(".is_empty()"),
        ".len() > 0 should be optimized to !.is_empty()\nGenerated:\n{}",
        generated
    );

    // ASSERTION 2: Should NOT contain .len() > 0
    assert!(
        !generated.contains(".len() > 0"),
        "Should NOT generate .len() > 0 (Clippy warning)\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
