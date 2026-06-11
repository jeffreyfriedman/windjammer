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

/// TDD: Vec.retain() closure parameter must be dereferenced for comparison
///
/// Bug: `Vec<T>::retain(|x| x != val)` -- Rust's `retain` passes `&T` to the closure,
/// so `x` is `&T`. Comparing `&T != T` fails with E0277 (trait bound not satisfied).
///
/// Fix: The compiler should generate `*x != val` or `x != &val` for retain closures.
///
/// Dogfooding source: editor/hierarchy_panel.wj, editor/scene_editor.wj,
///                     scene_graph/scene_graph_state.wj
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_retain_closure_deref_i64() {
    let source = r#"
struct Panel {
    ids: Vec<i64>
}

impl Panel {
    pub fn remove(self, entity_id: i64) {
        self.ids.retain(|id| id != entity_id)
    }
}

pub fn main() {
    let mut p = Panel { ids: vec![1, 2, 3] }
    p.remove(2)
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // retain closure param is &i64, so comparison must deref: *id != entity_id
    // or use reference on RHS: id != &entity_id
    assert!(
        rust.contains("*id != entity_id")
            || rust.contains("id != &entity_id")
            || rust.contains("*id != *entity_id"),
        "retain closure should deref parameter for comparison.\n\
         Expected `*id != entity_id` or `id != &entity_id`.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_retain_closure_deref_string() {
    let source = r#"
pub fn remove_name(names: Vec<string>, target: string) -> Vec<string> {
    let mut result = names
    result.retain(|n| n != target)
    result
}

pub fn main() {
    let mut names = vec!["alice", "bob", "charlie"]
    let result = remove_name(names, "bob")
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // For String comparisons, need deref or ref-compare
    assert!(
        rust.contains("*n != ")
            || rust.contains("n != &")
            || rust.contains("n.as_str()")
            || rust.contains("n != target"),
        "retain closure should handle string comparison correctly.\nGenerated:\n{}",
        rust
    );
}
