// TDD: Iteration variable method calls and comparisons
//
// Tests that iteration variables work correctly in method calls
// and comparisons without needing Rust-specific .as_str() calls.
// Windjammer infers string types and comparisons automatically.

use std::path::PathBuf;
use std::process::Command;

fn compile_code(code: &str) -> Result<String, String> {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let src_file = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::create_dir(&out_dir).map_err(|e| format!("Failed to create out dir: {}", e))?;

    let mut file =
        fs::File::create(&src_file).map_err(|e| format!("Failed to create source file: {}", e))?;
    file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write source: {}", e))?;

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&src_file)
        .arg("-o")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = out_dir.join("test.rs");
    fs::read_to_string(&generated_file).map_err(|e| format!("Failed to read generated file: {}", e))
}

fn compile_and_verify_rustc(code: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static VERIFY_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let generated = compile_code(code).expect("WJ compilation failed");

    let verify_id = VERIFY_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let rs_file = temp_dir.join(format!(
        "verify_iter_method_{}_{}.rs",
        std::process::id(),
        verify_id
    ));
    std::fs::write(&rs_file, &generated).expect("Failed to write rs file");

    let verify = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            rs_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let _ = std::fs::remove_file(&rs_file);

    if !verify.status.success() {
        let verify_stderr = String::from_utf8_lossy(&verify.stderr);
        panic!(
            "Generated Rust doesn't compile:\n{}\n\nGenerated code:\n{}",
            verify_stderr, generated
        );
    }

    generated
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_string_comparison() {
    // Idiomatic Windjammer: compare iteration variable directly to string literal
    let code = r#"
    pub fn process_strings(items: Vec<string>) -> Vec<bool> {
        let mut results = Vec::new()
        for item in items {
            let matches = item == "test"
            results.push(matches)
        }
        return results
    }
    "#;
    let generated = compile_and_verify_rustc(code);
    // The comparison should be clean, no .as_str() needed
    assert!(
        !generated.contains(".as_str()"),
        "Should not generate .as_str() - Windjammer handles string comparison automatically: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_comparison_with_struct_field() {
    // Idiomatic Windjammer: compare iteration variable with struct field
    let code = r#"
    struct ThemeSwitcher {
        themes: Vec<string>,
        current_theme: string,
    }
    impl ThemeSwitcher {
        pub fn render(self) -> string {
            let mut output = String::new()
            for t in self.themes {
                let selected = if t == self.current_theme { "selected" } else { "" }
                output.push_str(selected)
            }
            return output
        }
    }
    "#;
    let generated = compile_and_verify_rustc(code);
    // No .as_str(), no type mismatch errors
    assert!(
        !generated.contains(".as_str()"),
        "Should not generate .as_str(): {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_method_call_on_string() {
    // Iteration variable should support string method calls like .len(), .trim()
    let code = r#"
    pub fn count_long_strings(items: Vec<string>, min_len: i32) -> i32 {
        let mut count = 0
        for item in items {
            if item.len() as i32 > min_len {
                count = count + 1
            }
        }
        return count
    }
    "#;
    let generated = compile_and_verify_rustc(code);
    assert!(
        generated.contains(".len()"),
        "Should generate .len() method call on iteration variable: {}",
        generated
    );
}
