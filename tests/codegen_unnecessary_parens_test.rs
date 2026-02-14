// TDD Test: Generated Rust should not have unnecessary parentheses around cast expressions
//
// Bug: The codegen wraps ALL cast expressions in parentheses: `(expr as type)`.
// This causes 300+ "unnecessary parentheses" warnings in generated code when the cast
// appears in positions where parens are not needed (assignments, indexes, arguments, returns).
//
// Root Cause: Expression::Cast generates `format!("({} as {})", ...)` unconditionally.
// The parens are only needed when the cast is followed by `.method()` or `.field` access
// (because `as` has lower precedence than `.` in Rust), but NOT in assignments, indexes,
// function arguments, or comparison operands (where `as` already has correct precedence).
//
// Fix: Generate casts WITHOUT parens by default. Add parens only when the cast is the
// receiver of a method call or field access.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check_warnings(code: &str) -> (bool, String, Vec<String>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return (
            false,
            format!(
                "Compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            vec![],
        );
    }

    let generated_path = out_dir.join("test.rs");
    let generated =
        fs::read_to_string(&generated_path).unwrap_or_else(|e| format!("Read error: {}", e));

    let rustc = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc {
        Ok(rustc_output) => {
            let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();
            let paren_warnings: Vec<String> = stderr
                .lines()
                .filter(|l| l.contains("unnecessary parentheses"))
                .map(|l| l.to_string())
                .collect();
            (rustc_output.status.success(), generated, paren_warnings)
        }
        Err(e) => (false, format!("Failed to run rustc: {}", e), vec![]),
    }
}

#[test]
fn test_cast_in_assignment_no_unnecessary_parens() {
    let (ok, generated, warnings) = compile_and_check_warnings(
        r#"
struct Grid {
    width: i64,
    data: Vec<f32>,
}

impl Grid {
    fn get_index(self, x: i64, y: i64) -> usize {
        (y * self.width + x) as usize
    }

    fn set_value(self, x: i64, y: i64, val: f32) {
        let index = (y * self.width + x) as usize
        self.data[index] = val
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !warnings.is_empty() {
        println!("Paren warnings:\n{}", warnings.join("\n"));
    }

    assert!(ok, "Generated Rust should compile");
    assert!(
        warnings.is_empty(),
        "Should have no 'unnecessary parentheses' warnings.\nWarnings:\n{}\nGenerated:\n{}",
        warnings.join("\n"),
        generated
    );
}

#[test]
fn test_cast_in_index_no_unnecessary_parens() {
    let (ok, generated, warnings) = compile_and_check_warnings(
        r#"
struct Items {
    data: Vec<i64>,
}

impl Items {
    fn get_at(self, idx: i64) -> i64 {
        self.data[idx as usize]
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !warnings.is_empty() {
        println!("Paren warnings:\n{}", warnings.join("\n"));
    }

    assert!(ok, "Generated Rust should compile");
    assert!(
        warnings.is_empty(),
        "Should have no 'unnecessary parentheses' warnings for cast in index.\nWarnings:\n{}\nGenerated:\n{}",
        warnings.join("\n"),
        generated
    );
}

#[test]
fn test_cast_in_return_no_unnecessary_parens() {
    // Test that cast as a block return value has no unnecessary parens
    let (ok, generated, warnings) = compile_and_check_warnings(
        r#"
struct Grid {
    width: i64,
}

impl Grid {
    fn total_cells(self) -> usize {
        (self.width * self.width) as usize
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !warnings.is_empty() {
        println!("Paren warnings:\n{}", warnings.join("\n"));
    }

    assert!(ok, "Generated Rust should compile");
    assert!(
        warnings.is_empty(),
        "Should have no 'unnecessary parentheses' warnings for cast as return value.\nWarnings:\n{}\nGenerated:\n{}",
        warnings.join("\n"),
        generated
    );
}
