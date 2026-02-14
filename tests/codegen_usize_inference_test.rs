// TDD Test: Codegen Should Correctly Infer usize in Comparisons
// Bug: expression_produces_usize() has incomplete coverage, causing incorrect
//      casts in comparisons. Four patterns fail:
//      1. Method return type: frame_count() returns usize but not recognized
//      2. Nested field access: self.config.max_size -> usize not recognized
//      3. Non-self field access: asset.data_size -> usize not recognized
//      4. usize variable from method: free_slots() returns usize but not recognized
//
// Root Cause: expression_produces_usize() only handles .len()/.count()/.capacity()
//             for methods, only self.field for field access, and ignores parameters.
// Fix: Use infer_expression_type() as fallback to check if expression is usize.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check(code: &str) -> (bool, String, String) {
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
            String::new(),
            format!(
                "Compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        );
    }

    let generated_path = out_dir.join("test.rs");
    let generated =
        fs::read_to_string(&generated_path).unwrap_or_else(|e| format!("Read error: {}", e));

    // Compile the generated Rust with rustc
    let rustc = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc {
        Ok(rustc_output) => {
            let err = String::from_utf8_lossy(&rustc_output.stderr).to_string();
            (rustc_output.status.success(), generated, err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

// Test 1: usize parameter compared with usize field
#[test]
fn test_usize_param_compared_with_usize_field() {
    let (ok, generated, err) = compile_and_check(
        r#"
struct Config {
    max_size: usize,
}

struct Loader {
    config: Config,
}

impl Loader {
    fn validate_size(self, size_bytes: usize) -> bool {
        size_bytes > self.config.max_size
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    // Should NOT cast either side to i64 - both are usize
    assert!(
        !generated.contains("as i64"),
        "Should not cast usize to i64 when both sides are usize.\nGenerated:\n{}",
        generated
    );
    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}

// Test 2: usize method return compared with usize variable
#[test]
fn test_usize_method_return_compared_with_usize_var() {
    let (ok, generated, err) = compile_and_check(
        r#"
struct Animation {
    frames: Vec<int>,
}

impl Animation {
    fn frame_count(self) -> usize {
        self.frames.len()
    }
}

struct Controller {
    current_frame: usize,
}

impl Controller {
    fn is_past_end(self, animation: &Animation) -> bool {
        let frame_count = animation.frame_count()
        self.current_frame >= frame_count
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}

// Test 3: non-self field access with usize type
#[test]
fn test_non_self_usize_field_in_comparison() {
    let (ok, generated, err) = compile_and_check(
        r#"
struct Asset {
    data_size: usize,
}

struct AssetManager {
    assets: Vec<Asset>,
}

impl AssetManager {
    fn get_large_assets(self, min_size: usize) -> int {
        let mut count = 0
        for asset in &self.assets {
            if asset.data_size >= min_size {
                count = count + 1
            }
        }
        count
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}

// Test 4: usize from method call compared with usize variable
#[test]
fn test_usize_method_result_in_comparison() {
    let (ok, generated, err) = compile_and_check(
        r#"
struct Inventory {
    slots: Vec<int>,
}

impl Inventory {
    fn free_slots(self) -> usize {
        let mut count: usize = 0
        for slot in &self.slots {
            if *slot == 0 {
                count = count + 1
            }
        }
        count
    }

    fn has_space(self, needed: usize) -> bool {
        let free = self.free_slots()
        needed <= free
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}
