//! Validates [`integration_test_helpers`] behavior (passing build, failing build, cargo smoke).

#[path = "../common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multi_file_test_passes_minimal_two_module_build() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "greet.wj",
        r#"
pub fn hello() -> i32 {
    42
}
"#,
    );
    test.add_file(
        "client.wj",
        r#"
use greet::hello

pub fn bump() -> i32 {
    hello() + 1
}
"#,
    );
    test.assert_contains("client.rs", "hello");
    test.assert_contains("client.rs", "bump");
}

#[test]
fn multi_file_test_fails_with_clear_parse_error() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ok.wj",
        r#"
pub fn fine() -> i32 { 1 }
"#,
    );
    test.add_file(
        "broken.wj",
        r#"
pub fn bad( -> i32 { 1 }
"#,
    );
    test.assert_compile_error("Parse error");
}

#[test]
#[ignore = "Slow: spawns `cargo check` in a temp crate (compiles windjammer-runtime on cold cache). Run: cargo test --release --test integration_test_helpers_self_test -- --ignored --nocapture"]
#[cfg_attr(tarpaulin, ignore)]
fn multi_file_test_cargo_check_smoke() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "alpha.wj",
        r#"
pub fn a() -> i32 { 0 }
"#,
    );
    test.add_file(
        "beta.wj",
        r#"
use alpha::a

pub fn b() -> i32 {
    a() + 1
}
"#,
    );
    test.assert_compiles_without_error();
}
