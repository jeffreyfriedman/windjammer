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
    feature = "integration_tests",
))]

//! GREEN regression — `self.logs = Vec::new()` in a mutating method must emit `&mut self`
//! (`wj-proxy` historically used a pop-loop workaround; tip clears via assign).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CLEAR: &str = include_str!("fixtures/library_multipass/proxy_clear_logs_vec.wj");

#[test]
fn proxy_clear_logs_vec_assign_must_emit_mut_self() {
    let generated = test_utils::compile_single(CLEAR);
    assert!(
        generated.contains("fn clear_logs(&mut self)")
            || generated.contains("clear_logs(&mut self)"),
        "clear_logs assigning Vec::new() must emit &mut self; got:\n{generated}"
    );
    assert!(
        generated.contains("Vec::new()") || generated.contains("vec![]"),
        "must assign empty Vec; got:\n{generated}"
    );
}

#[test]
fn proxy_clear_logs_vec_assign_hexagonal_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub use domain::app::App
"#,
    );
    project.add_file("domain/mod.wj", "pub mod app\n");
    project.add_file("domain/app.wj", CLEAR);

    let map = project
        .compile()
        .expect("clear_logs multipass compile should succeed");
    let app_key = map
        .keys()
        .find(|k| k.ends_with("app.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing app.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let app_rs = map.get(&app_key).expect("app.rs");
    assert!(
        app_rs.contains("fn clear_logs(&mut self)")
            || app_rs.contains("clear_logs(&mut self)"),
        "multipass clear_logs must emit &mut self; got:\n{app_rs}"
    );

    project
        .cargo_check()
        .expect("self.logs = Vec::new() with &mut self must cargo-check");
}
