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

//! FAILING REPRO — cross-module `domain/config` + `api` must not emit `&mut char` in chars loop.
//!
//! Mirrors `wj-auth-api` / `wj-webhook` `domain/config.wj` `parse_positive_int`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CONFIG: &str = include_str!("fixtures/library_multipass/config_parse_positive_int_chars.wj");
const API: &str = include_str!("fixtures/library_multipass/config_port_from_env.wj");

#[test]
fn multipass_config_parse_positive_int_must_not_emit_mut_char() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod api
"#,
    );
    test.add_file("domain/mod.wj", "pub mod config\n");
    test.add_file("domain/config.wj", CONFIG);
    test.add_file("api/mod.wj", "pub mod port\n");
    test.add_file("api/port.wj", API);

    let map = test.compile().expect("config parse fixture should compile");
    let config_rs = map.get("domain/config.rs").expect("domain/config.rs");

    assert!(
        !config_rs.contains("char_to_digit(&mut ch)"),
        "RED: domain/config char loop must be `char`; emitted:\n{config_rs}"
    );
    assert!(
        !config_rs.contains("for mut ch in strings::chars"),
        "RED: domain/config for-in must not use `mut ch`; emitted:\n{config_rs}"
    );
    test.cargo_check()
        .expect("hexagonal config parse must cargo-check");
}
