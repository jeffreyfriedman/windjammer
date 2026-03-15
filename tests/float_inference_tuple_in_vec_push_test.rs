// TDD Test: Tuple literal float inference in Vec.push()
//
// Bug: When pushing (x, y, value * 1.5) to Vec<(i32, i32, f32)>,
// the literal 1.5 should infer f32 from the Vec's element type.
//
// Pattern from game: result.push((x + 1, y + 1, self.get_cost(x, y) * 1.414))

use windjammer::{build_project, CompilationTarget};

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_name = format!("tuple_push_f32_{}", std::process::id());
    let output_dir = temp_dir.join(&test_name);
    let test_file = output_dir.join("test.wj");

    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(&test_file, source).expect("Failed to write test file");

    build_project(&test_file, &output_dir, CompilationTarget::Rust, true)
        .expect("Compilation failed");

    std::fs::read_to_string(output_dir.join("test.rs")).expect("Generated Rust file not found")
}

#[test]
fn test_tuple_in_vec_push_f32() {
    // Define Vec<(i32, i32, f32)>, push (1, 2, value * 1.5) where value: f32
    // Literal 1.5 should infer f32 from Vec element type
    let source = r#"
fn test() -> Vec<(i32, i32, f32)> {
    let value: f32 = 1.0
    let mut result: Vec<(i32, i32, f32)> = Vec::new()
    result.push((1, 2, value * 1.5))
    result
}
"#;

    let rust_code = compile_and_get_rust(source);

    // The literal 1.5 in tuple should be f32 (from Vec<(i32, i32, f32)>)
    assert!(
        rust_code.contains("1.5_f32") || rust_code.contains("1.5f32"),
        "1.5 in tuple push should be f32, got:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("1.5_f64"),
        "1.5 should NOT be f64 when pushing to Vec<(i32, i32, f32)>, got:\n{}",
        rust_code
    );
}
