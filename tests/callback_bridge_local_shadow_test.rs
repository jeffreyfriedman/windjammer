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

//! Callback bridge must not wrap local variables that shadow function names.
//!
//! Bug: When a local variable has the same name as a function, the callback
//! bridge wraps it in a closure (e.g. `|__cb0| parent(&__cb0)`) even though
//! it's a value, not a function. This generates E0618: "expected function,
//! found `&(i32, i32)`".

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_local_variable_not_wrapped_as_callback() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "tree.wj",
        r#"
pub struct Node {
    pub id: i32,
    pub children: Vec<i32>,
}

pub fn parent(nodes: Vec<Node>) -> i32 {
    if nodes.len() > 0 {
        nodes[0].id
    } else {
        -1
    }
}

pub fn reconstruct_path(came_from: Map<i32, i32>, current: i32) -> Vec<i32> {
    let mut path = Vec::new()
    let mut pos = current
    while came_from.contains_key(pos) {
        match came_from.get(pos) {
            Some(parent) => {
                path.push(parent)
                pos = parent
            },
            None => { pos = -1 }
        }
    }
    path
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("tree.rs").expect("tree.rs");

    // The local variable `parent` should NOT be wrapped in a callback closure.
    // It's a simple value being pushed to a Vec, not a function call.
    assert!(
        !rs.contains("|__cb"),
        "Local variable 'parent' shadowing function name should not be wrapped in callback bridge.\n\
         Generated:\n{rs}"
    );
}
