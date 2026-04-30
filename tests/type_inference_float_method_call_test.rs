// TDD Test: Float literal inference with method call returning f32
//
// Bug: self.get_cost() * 1.414 generates 1.414_f64 instead of 1.414_f32
// Expected: Binary op with f32 method return should constrain literal to f32
//
// Dogfooding Win: This is a real bug found in astar_grid.wj

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
fn test_float_literal_in_binary_op_with_method_return() {
    let wj_source = r#"
struct Grid {
    pub cost: f32,
}

impl Grid {
    fn get_cost(self) -> f32 {
        self.cost
    }
    
    fn scaled_cost(self) -> f32 {
        self.get_cost() * 1.414
    }
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.414 in `self.get_cost() * 1.414` should be f32 (from get_cost: f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32 method return, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "1.414 should be f32 when multiplying f32 method return, got:\n{}",
        rust_code
    );
}
