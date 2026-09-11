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
))]

//! `match` with `Some(path)` owned `string` and `None => ""` must compile
//! (unify to owned `string`, not `&str` vs `String` E0308).
//! Ecosystem `wj-fetch` `output_path_value` hit this class of bug.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const OPTIONAL_PATH: &str = include_str!("fixtures/library_multipass/optional_path_match.wj");

#[test]
fn match_option_owned_string_none_empty_literal_must_compile() {
    let source = r#"
pub fn optional_path(value: Option<string>) -> string {
    match value {
        Some(path) => path,
        None => "",
    }
}
"#;
    let generated = test_utils::compile_single(source);
    let owns = generated.contains("String::new()")
        || generated.contains("\"\".to_string()")
        || generated.contains("String::from(\"\")");
    assert!(
        owns,
        "None => \"\" must own empty string in match arms, got:\n{generated}"
    );
    assert!(
        !generated.contains("None => \"\""),
        "must not leave bare &str in None arm, got:\n{generated}"
    );
}

#[test]
fn match_option_owned_string_none_empty_literal_multipass_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod paths
pub use paths::optional_path
"#,
    );
    project.add_file("paths.wj", OPTIONAL_PATH);

    let map = project
        .compile()
        .expect("optional_path multipass compile should succeed");
    let paths_rs = map.get("paths.rs").expect("paths.rs");
    assert!(
        paths_rs.contains("String::new()")
            || paths_rs.contains("\"\".to_string()")
            || paths_rs.contains("String::from(\"\")"),
        "multipass None arm must own empty string; emitted:\n{paths_rs}"
    );
    assert!(
        !paths_rs.contains("None => \"\""),
        "multipass must not leave bare &str in None arm; emitted:\n{paths_rs}"
    );

    project
        .cargo_check()
        .expect("multipass Option<string> match with None => \"\" must cargo-check");
}
