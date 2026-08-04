#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// Reused owned binding in a loop must borrow when callee emits `&T`.
#[test]
fn test_reused_binding_borrows_for_emitted_ref_callee() {
    let source = r##"
struct Graph {
    count: i32,
    label: string,
}

fn run_query(graph: Graph, query_id: u32) -> i32 {
    graph.count + query_id as i32 + graph.label.len() as i32
}

pub fn run_all(graph: Graph) -> i32 {
    let mut total = 0
    let mut q = 1
    while q <= 3 {
        total = total + run_query(graph, q as u32)
        q = q + 1
    }
    total
}
"##;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("run_query(&graph,") || generated.contains("run_query(& graph,"),
        "reused non-Copy graph in loop must borrow for &Graph callee. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("run_query(graph,")
            && !generated.contains("run_query(graph ,")
            && !generated.contains("run_query(graph.clone()"),
        "must not pass owned graph when callee emits &Graph. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("graph.clone()"),
        "must not clone when callee accepts shared borrow. Generated:\n{}",
        generated
    );
}

/// `strings::split` with std import must not coerce pipe delimiter to String.
#[test]
fn test_strings_split_std_import_pipe_delimiter_codegen() {
    let source = r##"
use std::strings

pub fn first_field(line: string) -> string {
    let parts = strings::split(line, "|")
    parts[0]
}
"##;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains(r#"strings::split("#),
        "expected strings::split call in:\n{}",
        generated
    );
    assert!(
        !generated.contains(r#""|".to_string()"#),
        "pipe delimiter must stay a string literal for &str formal. Generated:\n{}",
        generated
    );
}
