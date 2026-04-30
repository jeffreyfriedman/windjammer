// TDD Test: Float literal inference from struct field types
//
// Bug: AStarCell { cost: 1.0 } generates cost: 1.0_f64 even though cost: f32
// Expected: Struct field type should constrain literal type
//
// Dogfooding Win: This pattern appears hundreds of times in game code

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
fn test_float_literal_in_struct_field() {
    let wj_source = r#"
struct Cell {
    pub walkable: bool,
    pub cost: f32,
}

fn main() {
    let cell = Cell { walkable: true, cost: 1.0 }
    println!("{}", cell.cost)
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.0 in struct field should be f32 (from cost: f32)
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("cost: 1.0_f32")
            || rust_code.contains("cost: 1_f32")
            || rust_code.contains("cost: 1.0"),
        "1.0 should be f32 (or type-inferred) when assigned to f32 struct field, got:\n{}",
        rust_code
    );
}
