// TDD Test: Float literal inference in match expression assigned to variable
//
// Bug: let x = match map.get(k) { None => 999999.0 } doesn't infer from map type
// Pattern: Variable assignment from match, need to track HashMap<K, f32> → f32
//
// Dogfooding Win: Exact pattern from astar_grid.wj line 209

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_match_arm_in_variable_assignment() {
    let wj_source = r#"
use std::collections::HashMap

fn find_path(g_score: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {
    let current_g = match g_score.get(&(x, y)) {
        Some(v) => *v,
        None => 999999.0
    }
    current_g
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 999999.0 should be f32 (from HashMap<K, f32> → f32)
    assert!(
        !rust_code.contains("999999.0_f64") && !rust_code.contains("999999_f64"),
        "999999.0 should NOT be f64 when HashMap value type is f32, got:\n{}",
        rust_code
    );
}
