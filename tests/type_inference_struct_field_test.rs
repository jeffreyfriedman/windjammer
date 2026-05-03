// TDD Test: Float literal inference from struct field types
//
// Bug: AStarCell { cost: 1.0 } generates cost: 1.0_f64 even though cost: f32
// Expected: Struct field type should constrain literal type
//
// Dogfooding Win: This pattern appears hundreds of times in game code

#[path = "test_utils.rs"]
mod test_utils;

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

    let rust_code = test_utils::compile_single(wj_source);

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
