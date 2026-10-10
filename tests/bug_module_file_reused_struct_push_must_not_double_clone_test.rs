#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! Regression: module file reused struct push must not double clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Cluster {
    pub id: int,
    pub name: string,
}

pub fn simplify(cluster: Cluster, ratio: f32) -> Cluster {
    Cluster { id: cluster.id, name: cluster.name }
}

pub fn generate(cluster: Cluster) -> Vec<Cluster> {
    let mut base = Vec::new()
    base.push(cluster)
    let simplified = simplify(cluster, 0.5)
    let mut out = Vec::new()
    out.push(simplified)
    let _kept = base
    out
}
"#;

#[test]
fn module_file_reused_struct_push_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED: reused struct double-cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}
