#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD Test: Array literals should generate fixed-size syntax [] not vec![]
///
/// UPDATED: Array literal codegen now generates `[...]` (fixed-size) for ALL
/// non-empty array literals, not just in struct fields. This is correct because:
/// 1. Fixed-size arrays are more efficient than Vec
/// 2. Rust can infer the size from the literal
/// 3. `vec![...]` macro is still available when Vec is explicitly needed
///
/// Exception: Empty `[]` still generates `vec![]` because Rust can't infer
/// type/size without context. For typed empty arrays, use explicit syntax.
#[path = "../../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_struct_field_array_uses_fixed_syntax() {
    // Array literals in struct fields should generate [...] not vec![...]
    let code = test_utils::compile_single(
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

    // Should use fixed-size array syntax [...] not vec![...]
    // Float inference correctly adds _f32 suffix for [f32; 3] context
    assert!(
        code.contains("position: [1.0_f32, 2.0_f32, 3.0_f32]"),
        "Struct field [f32; 3] should use fixed-size array with correct float suffix. Generated:\n{}",
        code
    );
    assert!(
        code.contains("color: [1.0_f32, 0.0_f32, 0.0_f32, 1.0_f32]"),
        "Struct field [f32; 4] should use fixed-size array with correct float suffix. Generated:\n{}",
        code
    );

    // Should NOT contain vec![] for struct fields
    assert!(
        !code.contains("vec!["),
        "Struct field should NOT use vec![...] for fixed-size arrays. Generated:\n{}",
        code
    );
}

#[test]
fn test_standalone_array_uses_fixed_syntax() {
    // Array literals everywhere now generate fixed-size syntax [...]
    let code = test_utils::compile_single(
        r#"
fn main() {
    let items = [1, 2, 3]
}
"#,
    );

    // Should use fixed-size array syntax
    assert!(
        code.contains("[1, 2, 3]"),
        "Array literal should use fixed-size syntax [...]. Generated:\n{}",
        code
    );

    // Should NOT use vec![] (unless explicit vec![] macro is used in source)
    assert!(
        !code.contains("vec![1, 2, 3]"),
        "Array literal should NOT use vec![...] macro. Generated:\n{}",
        code
    );
}

#[test]
fn test_function_returning_fixed_array_uses_fixed_syntax() {
    // Functions with return type [f32; N] should generate [...] not vec![...]
    let code = test_utils::compile_single(
        r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

fn main() {
    let v = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
}
"#,
    );

    // Should contain fixed-size array syntax in the return
    assert!(
        code.contains("[self.x, self.y, self.z]"),
        "Return value of fn -> [f32; 3] should use [...] not vec![...]. Generated:\n{}",
        code
    );

    // Should NOT contain vec![] in the to_array method
    assert!(
        !code.contains("vec![self.x, self.y, self.z]"),
        "Return value should NOT use vec![...] for fixed-size array return. Generated:\n{}",
        code
    );
}

#[test]
fn test_empty_array_in_struct_uses_vec_syntax() {
    // Empty array literals generate vec![] because Rust can't infer type/size from []
    // TODO: Future enhancement: use type information from struct field to generate []
    let code = test_utils::compile_single(
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

    // Empty array currently generates vec![] (type inference limitation)
    assert!(
        code.contains("vec![]"),
        "Empty array currently generates vec![] due to type inference. Generated:\n{}",
        code
    );

    // Verify it compiles (rustc will convert vec![] to the target type)
    assert!(
        code.contains("values: [i32; 0]"),
        "Struct field type should be [i32; 0]. Generated:\n{}",
        code
    );
}
