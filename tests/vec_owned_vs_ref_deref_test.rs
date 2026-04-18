//! Vec<Copy> index codegen: never emit `*(vec[i])` (E0614).
//!
//! Windjammer lowers many `Vec<T>` parameters to `&Vec<T>` in Rust, but `vec[i]` is still `T` for
//! `T: Copy` without an extra dereference.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_vec_deref_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
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
fn test_vec_param_index_no_star_compiles() {
    let src = r#"
pub fn process(values: Vec<f32>) -> f32 {
    values[0]
}
"#;
    let (rs, compiles) = compile_wj_to_rust(src);
    assert!(
        !rs.contains("*(values[") && !rs.contains("* (values["),
        "must not emit *(values[…]) (E0614). Generated:\n{rs}"
    );
    assert!(compiles, "generated Rust should compile with rustc:\n{rs}");
}

#[test]
fn test_vec_local_index_no_star_compiles() {
    let src = r#"
pub fn sample() -> f32 {
    let mut values = vec![1.0, 2.0]
    values[1]
}
"#;
    let (rs, compiles) = compile_wj_to_rust(src);
    assert!(
        !rs.contains("*(values[") && !rs.contains("* (values["),
        "local Vec indexing must not use explicit deref. Generated:\n{rs}"
    );
    assert!(compiles, "generated Rust should compile:\n{rs}");
}

#[test]
fn test_vec_copy_struct_field_index_no_star() {
    let src = r#"
pub struct S {
    pub data: Vec<f32>,
}

pub fn first(s: S) -> f32 {
    s.data[0]
}
"#;
    let (rs, compiles) = compile_wj_to_rust(src);
    assert!(
        !rs.contains("*(s.data[") && !rs.contains("* (s.data["),
        "field Vec<Copy> index must not use *. Generated:\n{rs}"
    );
    assert!(compiles, "generated Rust should compile:\n{rs}");
}
