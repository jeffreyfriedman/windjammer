// TDD Test: Float literal in struct pushed to Vec in loop
//
// Bug: cells.push(Cell { cost: 1.0 }) generates 1.0_f64 in while loop
// Pattern from astar_grid.wj that's failing
//
// Dogfooding Win: This exact pattern fails in game code

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
fn test_struct_literal_in_vec_push_loop() {
    let wj_source = r#"
pub struct Cell {
    pub walkable: bool,
    pub cost: f32,
}

pub fn new_grid(size: i32) -> Vec<Cell> {
    let mut cells = Vec::new()
    let mut i = 0
    while i < size {
        cells.push(Cell { walkable: true, cost: 1.0 })
        i = i + 1
    }
    cells
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.0 in cells.push(Cell { cost: 1.0 }) should be f32
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field in Vec::push, got:\n{}",
        rust_code
    );
}
