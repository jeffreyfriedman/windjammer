//! TDD Test: Float literal inference in match arms (astar_grid pattern)
//!
//! Bug: `match g_score.get(&(x, y)) { Some(v) => *v, None => 999999.0 }` generates
//! 999999.0_f64 when *v is f32, causing E0308 "expected f32, found f64".
//!
//! Root cause: get_known_float_type_from_expr didn't handle Unary Deref (*v),
//! and match pattern variables (v) weren't populated for Some(v) over Option<&f32>.

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
fn test_match_hashmap_get_none_arm_infers_f32() {
    let wj_source = r#"
use std::collections::HashMap

pub fn pathfind(grid: (i32, i32), start: (i32, i32), goal: (i32, i32)) -> f32 {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((0, 0), 0.0)
    let (current_x, current_y) = (0, 0)
    let current_g = match g_score.get(&(current_x, current_y)) {
        Some(v) => *v,
        None => 999999.0,
    }
    current_g
}
"#;

    let rust_code = compile_single_file(wj_source);

    // The literal 999999.0 should be f32 (from *v in other arm)
    assert!(
        rust_code.contains("999999.0_f32") || rust_code.contains("999999.0f32"),
        "999999.0 should be f32 when match arm has *v from HashMap<_, f32>.get(), got:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "Should NOT use f64 when other arm is f32, got:\n{}",
        rust_code
    );
}
