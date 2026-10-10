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

//! DB-304: `return Some(x)` must not emit `Some(x.clone()).cloned()`.
//!
//! Product tip-out/gen semantic:
//!   WJ: `return Some(m)` / `return Some(rel)`
//!   tip: `return Some(m.clone()).cloned()` → E0599 (Option<T> has no `.cloned()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Metric {
    pub name: string,
}

pub struct Model {
    pub metrics: Vec<Metric>,
}

pub fn find_metric(model: Model, tool_name: string) -> Option<Metric> {
    for m in model.metrics {
        if m.name == tool_name {
            return Some(m)
        }
    }
    None
}
"#;

#[test]
fn module_file_some_must_not_emit_cloned_chain() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(").cloned()"),
        "MultiFile emitted Some(...).cloned() (illegal on Option<T>):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
