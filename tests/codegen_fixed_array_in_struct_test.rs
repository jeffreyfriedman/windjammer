/// TDD Test: Fixed-size array literals in struct fields should generate [] not vec![]
///
/// Bug: The array literal codegen unconditionally generates `vec![...]` for all
/// `Expression::Array` nodes. But when used as a struct field initializer where
/// the field type is `[f32; 3]`, it should generate `[...]` (fixed-size array).
///
/// Discovered via: codegen_qualified_struct_init_test::test_qualified_struct_init_simple
///
/// Root Cause: The vec![] change for dynamic arrays didn't account for
/// fixed-size array contexts (struct field initialization).
///
/// Fix: When generating array expressions inside struct literal fields,
/// use fixed-size `[...]` syntax since struct fields have explicit types.

fn compile_to_rust(source: &str) -> String {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    std::fs::create_dir_all(&output_dir).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        panic!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated = output_dir.join("test.rs");
    std::fs::read_to_string(&generated).unwrap_or_else(|_| {
        panic!(
            "No test.rs generated. stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn test_struct_field_array_uses_fixed_syntax() {
    // Array literals in struct fields should generate [...] not vec![...]
    let code = compile_to_rust(
        r#"
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

fn main() {
    let v = Vertex {
        position: [1.0, 2.0, 3.0],
        color: [1.0, 0.0, 0.0, 1.0],
    }
}
"#,
    );

    // Should contain fixed-size array syntax [...]
    assert!(
        code.contains("position: [1.0, 2.0, 3.0]") || code.contains("position: [1.0,2.0,3.0]"),
        "Struct field should use fixed-size array syntax [...], not vec![...]. Generated:\n{}",
        code
    );

    // Should NOT contain vec![] for struct fields
    assert!(
        !code.contains("vec![1.0, 2.0, 3.0]"),
        "Struct field should NOT use vec![...] for fixed-size arrays. Generated:\n{}",
        code
    );
}

#[test]
fn test_standalone_array_still_uses_vec() {
    // Array literals in let bindings should still generate vec![...]
    let code = compile_to_rust(
        r#"
fn main() {
    let items = [1, 2, 3]
}
"#,
    );

    // Should use vec![] for standalone let bindings
    assert!(
        code.contains("vec![1, 2, 3]"),
        "Standalone array literal should use vec![...]. Generated:\n{}",
        code
    );
}

#[test]
fn test_empty_array_in_struct_uses_fixed_syntax() {
    // Empty array in struct field should generate [] not vec![]
    let code = compile_to_rust(
        r#"
struct Data {
    values: [i32; 0],
}

fn main() {
    let d = Data {
        values: [],
    }
}
"#,
    );

    // Empty array in struct should NOT be vec![]
    assert!(
        !code.contains("vec![]") || code.contains("values: []"),
        "Empty array in struct field should use [...] syntax. Generated:\n{}",
        code
    );
}
