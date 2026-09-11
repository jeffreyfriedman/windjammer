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

//! P3.224 (`wj-scheduler`): read-only `Vec` helper formal inferred as `&Vec`,
//! but call site after `rest[i]` reuse emits `rest.clone()` (owned) → E0308.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str = r#"
use std::strings

pub fn parse_next(rest: Vec<string>) -> Result<string, string> {
    if rest.len() < 2 {
        return Err("usage")
    }
    let expr = rest[1]
    let at = match flag_value(rest, "--from") {
        Ok(v) => v,
        Err(e) => return Err(e),
    }
    Ok("${expr}|${at}")
}

fn flag_value(args: Vec<string>, flag: string) -> Result<string, string> {
    match optional_flag_value(args, flag) {
        Some(v) => Ok(v),
        None => Err("missing flag"),
    }
}

fn optional_flag_value(args: Vec<string>, flag: string) -> Option<string> {
    let mut i = 0
    while i + 1 < args.len() {
        if args[i] == flag {
            return Some(args[i + 1])
        }
        i = i + 1
    }
    None
}
"#;

#[test]
fn readonly_vec_helper_after_index_reuse_must_not_e0308() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    // Prefer borrow call site when formal is &Vec, OR owned formal with move/clone.
    // Must not mix: formal &Vec + call site owned clone.
    let formal_borrowed = rs.contains("fn flag_value(args: &Vec")
        || rs.contains("fn flag_value(args: &[");
    let call_owned_clone = rs.contains("flag_value(rest.clone()")
        || rs.contains("flag_value(args.clone()");
    assert!(
        !(formal_borrowed && call_owned_clone),
        "RED: borrowed Vec formal must not be called with rest.clone() (E0308). Got:\n{rs}"
    );
    assert!(
        ok,
        "RED: fixture must cargo-check after ownership fix. Generated:\n{rs}"
    );
}
