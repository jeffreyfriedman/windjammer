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
    // process's `data` should be owned (not &Vec<i32>) because it's passed to consume_vec
    assert!(
        !output.contains("data: &Vec<i32>"),
        "data should NOT be &Vec<i32> - passthrough should infer owned. Got:\n{}",
        output
    );
    assert!(
        output.contains("data: Vec<i32>") || output.contains("mut data: Vec<i32>"),
        "data should be Vec<i32> (owned). Got:\n{}",
        output
    );
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
fn join_strings(parts: Vec<String>) -> String {
    let mut result = String::new()
    for part in parts {
        result.push_str(part)
    }
    result
}

fn merge(items: Vec<String>) -> String {
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
