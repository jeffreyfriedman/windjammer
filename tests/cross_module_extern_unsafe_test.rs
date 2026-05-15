use std::path::Path;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> String {
    let test_dir = std::env::temp_dir().join("wj_cross_extern_test");
    let _ = std::fs::remove_dir_all(&test_dir);
    let _ = std::fs::create_dir_all(&test_dir);

    let input_file = test_dir.join("test_input.wj");
    std::fs::write(&input_file, source).unwrap();

    let wj_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg("--no-cargo")
        .arg("test_input.wj")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj compiler");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let output_file = test_dir.join("build").join("test_input.rs");
    if output_file.exists() {
        return std::fs::read_to_string(&output_file).unwrap_or_default();
    }

    let alt = test_dir.join("test_input.rs");
    if alt.exists() {
        return std::fs::read_to_string(&alt).unwrap_or_default();
    }

    // Try to find any .rs file
    if let Ok(entries) = std::fs::read_dir(&test_dir) {
        for entry in entries {
            if let Ok(e) = entry {
                if e.path().extension().map(|x| x == "rs").unwrap_or(false) {
                    return std::fs::read_to_string(e.path()).unwrap_or_default();
                }
            }
        }
    }
    if let Ok(build_dir) = std::fs::read_dir(test_dir.join("build")) {
        for entry in build_dir {
            if let Ok(e) = entry {
                if e.path().extension().map(|x| x == "rs").unwrap_or(false) {
                    return std::fs::read_to_string(e.path()).unwrap_or_default();
                }
            }
        }
    }

    format!("NO RS FILE FOUND:\nstdout: {}\nstderr: {}", stdout, stderr)
}

/// When calling an extern fn, the generated Rust must wrap it in an unsafe block.
#[test]
fn test_extern_fn_call_gets_unsafe_wrapper() {
    let source = r#"
extern fn some_ffi_function(x: f32, y: f32) -> f32

pub fn call_ffi() -> f32 {
    some_ffi_function(1.0, 2.0)
}
"#;
    let output = compile_wj_to_rust(source);
    assert!(
        output.contains("unsafe"),
        "Extern fn call should generate unsafe block. Got:\n{}",
        output
    );
}
