//! TDD Test: Fix Vec.remove() integer literal auto-ref bug
//!
//! Bug: When calling Vec.remove() with an integer literal,
//! the transpiler incorrectly adds `&`, resulting in `&{integer}` instead of `usize`.
//!
//! Example:
//!   Windjammer: `vec.remove(0)`
//!   Generated:  `vec.remove(&0)` ❌ Type error: expected usize, found &{integer}
//!   Should be:  `vec.remove(0)` ✅

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
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
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc_output {
        Ok(output) => {
            let rustc_success = output.status.success();
            let rustc_err = String::from_utf8_lossy(&output.stderr).to_string();
            (rustc_success, generated, rustc_err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_integer_literal() {
    let code = r#"
pub fn remove_first<T: Clone>(vec: &mut Vec<T>) -> Option<T> {
    if vec.len() > 0 {
        Some(vec.remove(0))
    } else {
        None
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        !generated.contains("vec.remove(&0)"),
        "Should not auto-ref integer literal: vec.remove(&0) is wrong"
    );

    assert!(
        generated.contains("vec.remove(0)"),
        "Should use integer literal directly: vec.remove(0)"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}
