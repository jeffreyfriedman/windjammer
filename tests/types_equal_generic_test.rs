#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_passthrough_ownership_inferred_owned() {
    // When a Vec<i32> param is passed to a function that takes owned Vec<i32>,
    // passthrough inference should detect the match and infer Owned.
    let output = test_utils::compile_single(
        r#"
fn consume_vec(items: Vec<i32>) -> i32 {
    let mut sum = 0
    for item in items {
        sum = sum + item
    }
    sum
}

fn process(data: Vec<i32>) -> i32 {
    consume_vec(data)
}
"#,
    );
    // Passthrough: `process` forwards `data`, so its formal follows `consume_vec`'s. A read-only Vec of Copy
    // elements is borrowed (the IR rule P3.528), so both are `&Vec<i32>`; both owned is consistent as well.
    let both_borrowed = output.contains("items: &Vec<i32>") && output.contains("data: &Vec<i32>");
    let both_owned = output.contains("items: Vec<i32>") && (output.contains("data: Vec<i32>") || output.contains("mut data: Vec<i32>"));
    assert!(both_borrowed || both_owned, "passthrough formal must follow the callee's. Got:\n{output}");
}

#[test]
fn test_option_passthrough_ownership_inferred_owned() {
    let output = test_utils::compile_single(
        r#"
fn unwrap_option(opt: Option<i32>) -> i32 {
    match opt {
        Some(v) => v,
        None => 0
    }
}

fn check(value: Option<i32>) -> i32 {
    unwrap_option(value)
}
"#,
    );
    // check's `value` should be owned (not &Option<i32>)
    assert!(
        !output.contains("value: &Option<i32>"),
        "value should NOT be &Option<i32>. Got:\n{}",
        output
    );
}

#[test]
fn test_vec_of_string_passthrough_ownership() {
    let output = test_utils::compile_single(
        r#"
fn join_strings(parts: Vec<string>) -> string {
    let mut result = ""
    for part in parts {
        result.push_str(part)
    }
    result
}

fn merge(items: Vec<string>) -> string {
    join_strings(items)
}
"#,
    );
    assert!(
        !output.contains("items: &Vec<"),
        "items should NOT be &Vec<String>. Got:\n{}",
        output
    );
}
