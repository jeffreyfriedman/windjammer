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

//! FAILING REPRO — recursive helper taking owned `Vec<string>` must not borrow at call site.
//!
//! Ecosystem `wj-yaml` hit E0308 when codegen emitted `parse_value(&nested_lines, &next_indent)`
//! for owned formals. Workaround: `let mut nl = nested_lines` re-box before call.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn recursive_owned_vec_helper_must_move_not_borrow() {
    let generated = test_utils::compile_single(
        r#"
pub fn head(lines: Vec<string>) -> Option<(string, Vec<string>)> {
    if lines.len() == 0 {
        return None
    }
    let mut first = ""
    let mut rest: Vec<string> = Vec::new()
    let mut seen = false
    for line in lines {
        if !seen {
            first = line
            seen = true
        } else {
            rest.push(line)
        }
    }
    Some((first, rest))
}

pub fn parse_block(lines: Vec<string>) -> string {
    match head(lines) {
        Some(pair) => {
            let mut nested: Vec<string> = Vec::new()
            nested.push(pair.1[0])
            let depth = 2
            parse_block(nested)
        },
        None => "[]".to_string(),
    }
}
"#,
    );

    assert!(
        !generated.contains("parse_block(&nested"),
        "owned Vec formal must not receive & at recursive call site:\n{generated}"
    );
}
