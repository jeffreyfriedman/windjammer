// TDD Test: Float literal inference in Vec.push()
//
// Bug: scores.push(0.0) generates 0.0_f64 for Vec<f32>
// Expected: Vec<f32> → push(f32) should constrain argument
//
// Dogfooding Win: Common pattern in game code

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
fn test_vec_push_float_literal() {
    let wj_source = r#"
fn init_scores() -> Vec<f32> {
    let mut scores: Vec<f32> = Vec::new()
    scores.push(0.0)
    scores.push(1.0)
    scores.push(2.5)
    scores
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // All literals should be f32 (from Vec<f32> → push(f32))
    assert!(
        !rust_code.contains("_f64"),
        "Float literals should NOT be f64 when pushing to Vec<f32>, got:\n{}",
        rust_code
    );
}
