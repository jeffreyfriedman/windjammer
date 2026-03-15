/// TDD: E0308 Type Coercion - Vec.push with Index expressions
///
/// Rust's Index trait returns &T. Vec::push expects T.
/// For Copy types (i32, (i32,i32), f32), we must dereference: *vec[idx]
///
/// Bug: rev.push(path[k]) generates rev.push(&path[k]) → E0308 (expected (i32,i32), found &(i32,i32))
/// Fix: Generate rev.push(*path[k]) when element type is Copy
///
/// Key Principle: Automatic coercion between &T and T based on context (Rust's Deref coercion)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    let output_dir = test_dir.join("build");
    fs::write(&input_file, source).expect("Failed to write source file");

    let compile_result = windjammer::build_project(
        &input_file,
        &output_dir,
        windjammer::CompilationTarget::Rust,
        true,
    );

    let rs_content = if let Err(e) = compile_result {
        panic!("Windjammer compilation failed: {}", e);
    } else {
        let generated_file = output_dir.join("test.rs");
        fs::read_to_string(&generated_file).expect("Failed to read generated file")
    };

    let main_rs = output_dir.join("test.rs");
    let rlib_output = test_dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

#[test]
fn test_vec_push_index_tuple_copy_type() {
    // rev.push(path[0]) when rev: Vec<(i32,i32)>, path: Vec<(i32,i32)>
    // Index returns &(i32,i32), push expects (i32,i32) - need * to dereference
    let source = r#"
pub fn copy_first(path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut rev = Vec::new()
    rev.push(path[0])
    rev
}

fn main() {
    let path = vec![(1, 2), (3, 4)]
    let rev = copy_first(path)
}
"#;

    let (rust, compiles) = compile_wj_to_rust(source);

    // Must NOT generate &path[0] - that would be wrong (double ref, E0308)
    assert!(
        !rust.contains("&path[") && !rust.contains("& path["),
        "Should NOT add & for Vec.push(Index) when element is Copy, got:\n{}",
        rust
    );

    // Should generate *path[0] or path[0].clone() - Rust Index returns &T, push needs T
    // Prefer *(path[0]) for Copy (no alloc); path[0].clone() also works
    let has_deref = rust.contains("*(path[") || rust.contains("* (path[");
    let has_clone = rust.contains("path[0].clone()") || rust.contains("path[0usize].clone()");
    assert!(
        has_deref || (has_clone && compiles),
        "Need *(path[0]) or path[0].clone() for Copy type. Got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_push_index_primitive_copy_type() {
    // buf.push(nums[i]) when buf: Vec<i32>, nums: Vec<i32>
    let source = r#"
pub fn copy_evens(nums: Vec<i32>) -> Vec<i32> {
    let mut buf = Vec::new()
    let mut i = 0
    while i < nums.len() {
        let n = nums[i]
        if n % 2 == 0 {
            buf.push(n)
        }
        i = i + 1
    }
    buf
}

fn main() {
    let nums = vec![1, 2, 3, 4]
    let evens = copy_evens(nums)
}
"#;

    let (rust, compiles) = compile_wj_to_rust(source);

    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_push_index_f32_param() {
    // Same pattern as e0308_index_deref_copy_param but for Vec::push
    let source = r#"
pub fn collect_floats(vals: Vec<f32>) -> Vec<f32> {
    let mut out = Vec::new()
    let mut i = 0
    while i < vals.len() {
        out.push(vals[i])
        i = i + 1
    }
    out
}

fn main() {
    let v = vec![1.0, 2.0, 3.0]
    let _ = collect_floats(v)
}
"#;

    let (rust, compiles) = compile_wj_to_rust(source);

    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}
