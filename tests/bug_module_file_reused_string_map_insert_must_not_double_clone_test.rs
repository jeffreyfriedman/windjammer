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
    feature = "codegen_tests",
))]

//!
//!   `self.tag_index.insert(tag.clone().clone(), vec![])`
//!   `self.tag_index.get_mut(tag.clone())`
//! WJ is `insert(tag, vec![])` then `get_mut(tag)`. `contains_key` and
//! `get_mut` borrow; one clone into `insert` is enough because `tag` is used

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::collections::HashMap

pub fn set_tag(index: HashMap<string, Vec<int>>, tag: string, id: int) -> HashMap<string, Vec<int>> {
    let mut index = index
    if !index.contains_key(tag) {
        index.insert(tag, vec![])
    }
    if let Some(list) = index.get_mut(tag) {
        list.push(id)
    }
    index
}
"#;

#[test]
fn module_file_reused_string_map_insert_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED: reused string double-cloned into map insert:\n{rs}"
    );
    assert!(
        !rs.contains("get_mut(tag.clone())"),
        "RED: borrowed get_mut received tag.clone():\n{rs}"
    );
    let _ = test.cargo_check();
}

