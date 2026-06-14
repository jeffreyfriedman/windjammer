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

/// TDD: HashMap.get() match arms must have compatible types
///
/// Bug: `HashMap.get(&key)` returns `Option<&V>`, so `Some(v) => v` yields `&V`.
/// But `None => default_value` yields `V`. The compiler generates mismatched arms:
///   `Some(v) => v` (&f32) vs `None => 999999.0_f32` (f32)
///
/// Fix: When the scrutinee is `HashMap.get()`, the compiler should deref the Some arm:
///   `Some(v) => *v`
///
/// Dogfooding source: ai/astar_grid.wj, ai/navmesh.wj
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_hashmap_get_match_deref_some_arm() {
    let source = r#"
use std::collections::HashMap

pub fn lookup_score(scores: HashMap<i32, f32>, key: i32) -> f32 {
    match scores.get(key) {
        Some(v) => v,
        None => 0.0,
    }
}

pub fn main() {
    let mut scores = HashMap::new()
    scores.insert(1, 42.0)
    let s = lookup_score(scores, 1)
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    assert!(
        rust.contains("*v") || rust.contains("v.clone()"),
        "Some(v) should be dereferenced when None arm is an owned value.\n\
         Expected `*v` or `v.clone()` in Some arm.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_hashmap_get_match_deref_with_tuple_key() {
    let source = r#"
use std::collections::HashMap

pub fn get_g_score(g_score: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {
    match g_score.get((x, y)) {
        Some(v) => v,
        None => 999999.0,
    }
}

pub fn main() {
    let mut g = HashMap::new()
    g.insert((1, 2), 3.5)
    let s = get_g_score(g, 1, 2)
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    assert!(
        rust.contains("*v") || rust.contains("v.clone()"),
        "Some(v) from HashMap.get() should be dereferenced.\nGenerated:\n{}",
        rust
    );
}
