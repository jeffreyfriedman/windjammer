// TDD Test: Float literal inference from function parameter types
//
// Bug: Vec3::new(x, 0.0, z) generates 0.0_f64 instead of 0.0_f32
// Expected: Look up Vec3::new signature → (f32, f32, f32) → constrain args
//
// Dogfooding Win: Constructors are everywhere in game code

use std::fs;
use std::path::Path;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

fn gather_generated_rust(output_dir: &Path) -> String {
    fn walk(dir: &Path, buf: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, buf);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name == "mod.rs" || name == "lib.rs" {
                    continue;
                }
                if let Ok(s) = fs::read_to_string(&p) {
                    buf.push(s);
                }
            }
        }
    }
    let mut parts = Vec::new();
    walk(output_dir, &mut parts);
    parts
        .join("\n")
        .lines()
        .filter(|l| !l.contains("use super::"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_single_file(source: &str) -> String {
    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");
    fs::write(src.path().join("test.wj"), source).expect("write test.wj");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        false,
        &[],
    )
    .expect("build_project_ext");
    gather_generated_rust(out.path())
}

#[test]
fn test_function_param_float_inference() {
    let wj_source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

fn create_vector(x: f32, z: f32) -> Vec3 {
    Vec3::new(x, 0.0, z)
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 0.0 should be f32 (from Vec3::new(f32, f32, f32))
    assert!(
        !rust_code.contains("Vec3::new(x, 0.0_f64") && !rust_code.contains("new(x, 0_f64"),
        "0.0 should NOT be f64 when passed to Vec3::new(f32, f32, f32), got:\n{}",
        rust_code
    );
}
