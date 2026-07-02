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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: When a function parameter is inferred as `&Vec<T>` (because the body only reads it),
/// call sites that pass an owned `Vec<T>` must auto-borrow with `&`.
///
/// Source pattern (Windjammer):
///   fn helper(items: Vec<i32>) -> i32 { items.len() as i32 }
///   fn caller() { let v = Vec::new(); let n = helper(v); }
///
/// Expected generated Rust:
///   fn helper(items: &Vec<i32>) -> i32 { items.len() as i32 }
///   fn caller() { let v: Vec<i32> = Vec::new(); let n = helper(&v); }
///                                                         ^^^ auto-borrow
#[test]
fn test_auto_borrow_owned_vec_to_ref_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r##"
struct Item {
    pub name: string,
    pub value: i32,
}

fn sum_values(items: Vec<Item>) -> i32 {
    let mut total = 0
    let mut i = 0
    while i < items.len() {
        total = total + items[i].value
        i = i + 1
    }
    total
}

pub fn run() -> i32 {
    let mut items: Vec<Item> = Vec::new()
    items.push(Item { name: "a", value: 10 })
    items.push(Item { name: "b", value: 20 })
    let total = sum_values(items)
    total
}
"##,
    );
    test.assert_compiles_without_error();
}

/// Same as above but with a reassignment pattern:
///   nodes = update_params(nodes, id, val)
/// where update_params takes &Vec and returns Vec
#[test]
fn test_auto_borrow_reassign_pattern() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r##"
struct Record {
    pub id: i32,
    pub label: string,
}

fn filter_records(records: Vec<Record>, min_id: i32) -> Vec<Record> {
    let mut out: Vec<Record> = Vec::new()
    let mut i = 0
    while i < records.len() {
        if records[i].id >= min_id {
            out.push(records[i])
        }
        i = i + 1
    }
    out
}

pub fn run() -> i32 {
    let mut recs: Vec<Record> = Vec::new()
    recs.push(Record { id: 1, label: "first" })
    recs.push(Record { id: 5, label: "fifth" })
    recs.push(Record { id: 3, label: "third" })
    recs = filter_records(recs, 3)
    recs.len() as i32
}
"##,
    );
    test.assert_compiles_without_error();
}
