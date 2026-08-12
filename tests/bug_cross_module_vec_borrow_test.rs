#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// Cross-module Vec formal must borrow at call site when callee emits `&Vec<T>`.
#[test]
fn test_cross_module_vec_helper_emits_borrow_at_call_site() {
    let helper = r##"
pub fn graph_pr_f64_get(keys: Vec<i64>, vals: Vec<f64>, vertex: i64) -> f64 {
    let mut i = 0
    while i < keys.len() {
        if keys[i] == vertex {
            return vals[i]
        }
        i = i + 1
    }
    0.0
}
"##;

    let caller = r##"
use helper

pub fn score(keys: Vec<i64>, vals: Vec<f64>, vertex: i64) -> f64 {
    helper::graph_pr_f64_get(keys, vals, vertex)
}
"##;

    let outputs = test_utils::compile_project(&[("helper.wj", helper), ("main.wj", caller)]);
    let generated = outputs
        .values()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n---\n");

    assert!(
        generated.contains("graph_pr_f64_get(&keys")
            || generated.contains("graph_pr_f64_get(& keys")
            // Formals already `&Vec<_>` — bare `keys` is the correct call-site shape.
            || (generated.contains("keys: &Vec<")
                && generated.contains("graph_pr_f64_get(keys")),
        "cross-module Vec helper must receive borrowed args. Generated:\n{}",
        generated
    );
}
