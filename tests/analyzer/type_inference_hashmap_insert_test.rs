#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// TDD Test: Float literal inference in HashMap.insert()
//
// Bug: g_score.insert((x, y), 0.0) generates 0.0_f64 for HashMap<K, f32>
// Expected: HashMap<K, f32> → insert() value param should be f32
//
// Dogfooding Win: Hundreds of HashMap operations in game code

#[path = "../common/test_utils.rs"]
mod test_utils;

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

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 0.0 should be f32 (from HashMap<K, f32> → insert(K, f32))
    assert!(
        !rust_code.contains("0.0_f64") && !rust_code.contains("0_f64"),
        "0.0 should NOT be f64 when inserting into HashMap<K, f32>, got:\n{}",
        rust_code
    );
}
