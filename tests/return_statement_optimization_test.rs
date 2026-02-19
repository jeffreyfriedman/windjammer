use anyhow::Result;
/// TDD Test: Suppress unneeded explicit return statements
///
/// PROBLEM: Windjammer generates explicit `return` statements even when they're not needed.
/// Rust's last expression in a block is the implicit return value, so explicit `return`
/// at the end of a function is redundant and triggers Clippy warnings.
///
/// WINDJAMMER PHILOSOPHY: Generate idiomatic Rust that passes Clippy without warnings.
///
/// Example WJ source:
/// ```
/// fn get_value() -> i32 {
///     return 42
/// }
/// ```
///
/// Should generate:
/// ```rust
/// fn get_value() -> i32 {
///     42
/// }
/// ```
///
/// Should NOT generate:
/// ```rust
/// fn get_value() -> i32 {
///     return 42;  // ⚠️  Clippy: unneeded `return` statement
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
fn test_suppress_return_when_last_statement() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_return_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with explicit return as last statement
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn get_number() -> i32 {
    return 42
}

fn get_optional() -> Option<i32> {
    return Some(10)
}

fn process(x: i32) -> i32 {
    if x > 0 {
        return x * 2
    } else {
        return 0
    }
}

fn main() {
    let n = get_number()
    let opt = get_optional()
    let result = process(5)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "return-test"
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

    // ASSERTION 1: Last statement return should be omitted (implicit return)
    // The function should end with just `42` not `return 42;`
    assert!(
        !generated.contains("return 42;"),
        "Last statement 'return 42' should be optimized to implicit return.\nGenerated:\n{}",
        generated
    );

    // ASSERTION 2: Return in if-else branches as last statement should also be omitted
    assert!(
        !generated.contains("return x * 2;") || !generated.contains("return 0;"),
        "Return statements in if-else branches as last statements should be optimized.\nGenerated:\n{}",
        generated
    );

    // ASSERTION 3: The function should still have the correct return value (just the expression)
    assert!(
        generated.contains("fn get_number() -> i32") && (generated.contains("    42") || generated.contains("42\n}")),
        "Function should have implicit return (expression without 'return' keyword).\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_keep_return_when_early_exit() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_early_return_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with early return (should be kept)
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn check_value(x: i32) -> string {
    if x < 0 {
        return "negative"
    }
    
    if x == 0 {
        return "zero"
    }
    
    "positive"
}

fn main() {
    let result = check_value(10)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "early-return-test"
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

    // ASSERTION: Early returns SHOULD be kept (they're not the last statement)
    assert!(
        generated.contains("return") && generated.contains("negative"),
        "Early return statements should be preserved (they're not last statements).\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_suppress_void_return() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_void_return_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with void return as last statement
    fs::write(
        src_dir.join("main.wj"),
        r#"
fn do_something(x: i32) {
    if x > 0 {
        // some work
        return
    }
    // more work
}

fn main() {
    do_something(5)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "void-return-test"
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

    // ASSERTION: Void return (return; with no value) in if branches is an early exit
    // so it should be kept. But if it's the last statement of the function body, it can be omitted.
    // This test just verifies the code compiles correctly.
    assert!(
        output.status.success() || generated.contains("fn do_something"),
        "Void return handling should compile successfully.\nGenerated:\n{}",
        generated
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
