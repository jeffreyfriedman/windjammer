use anyhow::Result;
/// TDD Test: Convert simple match expressions to matches! macro
///
/// PROBLEM: Clippy warns about match expressions that just return bool and
/// suggests using the matches! macro instead for better readability.
///
/// WINDJAMMER PHILOSOPHY: Generate idiomatic Rust that passes Clippy without warnings.
///
/// Example WJ source:
/// ```
/// match value {
///     Some(_) => true,
///     None => false,
/// }
/// ```
///
/// Should generate:
/// ```rust
/// matches!(value, Some(_))
/// ```
///
/// Should NOT generate:
/// ```rust
/// match value {  // ⚠️  Clippy: single_match_else or match_bool
///     Some(_) => true,
///     None => false,
/// }
/// ```
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_true_false_becomes_matches() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_matches_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with simple match returning bool
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn is_some(value: Option<i32>) -> bool {
    match value {
        Some(_) => true,
        None => false,
    }
}

fn main() {
    let opt = Some(42)
    let result = is_some(opt)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "matches-test"
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

    // ASSERTION 1: Should use matches! macro
    assert!(
        generated.contains("matches!("),
        "Match returning bool should be optimized to matches! macro\nGenerated:\n{}",
        generated
    );

    // ASSERTION 2: Should contain the pattern
    assert!(
        generated.contains("Some(_)"),
        "matches! macro should contain the pattern\nGenerated:\n{}",
        generated
    );

    // ASSERTION 3: Should NOT have verbose match block (unless it's more complex)
    // We allow match blocks for complex cases, but simple true/false should use matches!
    let match_count = generated.matches("match value").count();
    let matches_count = generated.matches("matches!(value").count();
    assert!(
        matches_count > 0 || match_count == 0,
        "Simple boolean match should use matches! macro\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_false_true_becomes_not_matches() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_not_matches_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with match returning false/true (inverted)
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn is_none(value: Option<i32>) -> bool {
    match value {
        Some(_) => false,
        None => true,
    }
}

fn main() {
    let opt = Some(42)
    let result = is_none(opt)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "not-matches-test"
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

    // ASSERTION: Should use !matches! for inverted pattern
    assert!(
        (generated.contains("!matches!(") || generated.contains("! matches!("))
            && generated.contains("Some(_)"),
        "Inverted match should be optimized to !matches! macro\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_complex_match_not_optimized() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_complex_match_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with complex match (should NOT be optimized)
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn get_value(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x * 2,
        None => 0,
    }
}

fn main() {
    let opt = Some(42)
    let result = get_value(opt)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "complex-match-test"
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

    // ASSERTION: Complex match should NOT use matches! (returns different values)
    assert!(
        generated.contains("match opt"),
        "Complex match should remain as match expression\nGenerated:\n{}",
        generated
    );

    // Should NOT use matches! for non-boolean returns
    assert!(
        !generated.contains("matches!(opt"),
        "matches! should only be used for boolean results\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
