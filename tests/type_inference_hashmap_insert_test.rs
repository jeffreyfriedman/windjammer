// TDD Test: Float literal inference in HashMap.insert()
//
// Bug: g_score.insert((x, y), 0.0) generates 0.0_f64 for HashMap<K, f32>
// Expected: HashMap<K, f32> → insert() value param should be f32
//
// Dogfooding Win: Hundreds of HashMap operations in game code

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
fn test_hashmap_insert_float_literal() {
    let wj_source = r#"
use std::collections::HashMap

fn init_scores() -> HashMap<(i32, i32), f32> {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((0, 0), 0.0)
    g_score
}
"#;

    let rust_code = compile_single_file(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 0.0 should be f32 (from HashMap<K, f32> → insert(K, f32))
    assert!(
        !rust_code.contains("0.0_f64") && !rust_code.contains("0_f64"),
        "0.0 should NOT be f64 when inserting into HashMap<K, f32>, got:\n{}",
        rust_code
    );
}
