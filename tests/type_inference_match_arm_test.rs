// TDD Test: Float literal inference in match arms
//
// Bug: Match arms returning float literals don't constrain to expected type
// Pattern: match option { Some(x) => x, None => 999999.0 } // Should be 999999.0_f32
//
// Dogfooding Win: Common pattern in game code (default values)

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
fn test_float_literal_in_match_arm() {
    let wj_source = r#"
fn get_score_or_default(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(key) {
        Some(score) => *score,
        None => 999999.0
    }
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 999999.0 should be f32 (from function return type)
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "999999.0 should NOT be f64 when match arm returns f32, got:\n{}",
        rust_code
    );
}
