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

//! `"${s}"` yields a fresh value: passing it to an owned formal must not move `s`, even
//! when the call-site last-use logic would otherwise peel a trailing `.clone()` into a move.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn check(src: &str) {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", src);
    test.cargo_check().unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn interpolated_binding_into_owned_formal_then_reused() {
    check(
        r#"
pub struct Row { pub text: string }

pub fn store(text: string) -> Row { Row { text: text } }

pub fn run(raw: string) -> int {
    let a = store("${raw}")
    let b = store(raw)
    a.text.len() + b.text.len()
}
"#,
    );
}

#[test]
fn interpolated_binding_into_owned_formal_in_match_arm_then_reused() {
    check(
        r#"
pub struct Row { pub text: string }

pub fn store(text: string) -> Row { Row { text: text } }
pub fn peek(text: string) -> int { text.len() }

pub fn run(kind: string, raw: string) -> int {
    match kind {
        "a" => {
            let first = store("${raw}")
            let second = peek(raw)
            first.text.len() + second
        },
        _ => 0,
    }
}
"#,
    );
}

#[test]
fn interpolated_binding_pushed_then_reused() {
    check(
        r#"
pub fn collect(raw: string) -> Vec<string> {
    let mut out: Vec<string> = Vec::new()
    out.push("${raw}")
    out.push(raw)
    out
}
"#,
    );
}
