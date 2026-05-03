// TDD Test: Float literal inference in EXACT astar_grid pattern
//
// Bug: result.push((x, y, self.get_cost(x, y) * 1.414)) generates f64
// This is the EXACT pattern from astar_grid.wj that fails
//
// Dogfooding Win: Extracted from real game code

use std::fs;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_float_literal_in_tuple_push_with_method_return() {
    let source = r#"
struct Grid {
    pub width: i32,
    pub cells: Vec<f32>,
}

impl Grid {
    fn get_cost(self, x: i32, y: i32) -> f32 {
        self.cells[(y * self.width + x) as usize]
    }

    fn get_neighbors(self, x: i32, y: i32) -> Vec<(i32, i32, f32)> {
        let mut result = Vec::new()
        result.push((x + 1, y + 1, self.get_cost(x + 1, y + 1) * 1.414))
        result
    }
}
"#;

    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");
    fs::write(src.path().join("test.wj"), source).expect("write test.wj");

    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let rust_code = fs::read_to_string(out.path().join("test.rs")).expect("read test.rs");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32 method return in tuple, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "1.414 should be f32 when multiplying f32 method return in tuple, got:\n{}",
        rust_code
    );
}
