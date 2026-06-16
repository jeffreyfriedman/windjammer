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

//! Library multipass codegen regressions (E0308 map keys / E0596 false mut-borrow).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;
use std::path::PathBuf;

fn fixture(relative: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/library_multipass")
            .join(relative),
    )
    .unwrap_or_else(|e| panic!("missing fixture {relative}: {e}"))
}

#[test]
fn test_library_multipass_hashmap_str_key_no_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "animation/controller.wj",
        r#"
use std::collections::HashMap

pub struct Animation {
    name: string,
}

impl Animation {
    pub fn name(self) -> string {
        self.name
    }
}

pub struct AnimationController {
    animations: HashMap<string, Animation>,
}

impl AnimationController {
    pub fn has_animation(self, name: string) -> bool {
        self.animations.contains_key(name)
    }

    pub fn play(self, name: string) {
        if self.animations.contains_key(name) {
            let _ = name
        }
    }
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("animation/controller.rs")
        .expect("controller.rs generated");

    assert!(
        rs.contains("contains_key(name)"),
        "str key must pass through without .to_string(). Got:\n{rs}"
    );
    assert!(
        !rs.contains("contains_key(name.to_string())"),
        "must not emit .to_string() for HashMap string keys. Got:\n{rs}"
    );
}

#[test]
fn test_library_multipass_hashmap_tuple_key_auto_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("pathfind/astar_hashmap_keys.wj", &fixture("astar_hashmap_keys.wj"));

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("pathfind/astar_hashmap_keys.rs")
        .expect("astar_hashmap_keys.rs generated");

    assert!(
        rs.contains("g_score.get(&("),
        "local HashMap tuple keys must be auto-borrowed in library multipass. Got:\n{rs}"
    );
    assert!(
        rs.contains("came_from.get(&node)"),
        "local HashMap variable keys must be auto-borrowed. Got:\n{rs}"
    );
    assert!(
        !rs.contains("g_score.get((current_x, current_y))"),
        "must not pass owned tuple to HashMap::get. Got:\n{rs}"
    );
}

#[test]
fn test_library_multipass_hashmap_tuple_key_auto_borrow_with_mod_wj() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod pathfind;\n");
    test.add_file("pathfind/astar_hashmap_keys.wj", &fixture("astar_hashmap_keys.wj"));

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("pathfind/astar_hashmap_keys.rs")
        .expect("astar_hashmap_keys.rs generated");

    assert!(
        rs.contains("g_score.get(&("),
        "mod.wj sibling must not break tuple key auto-borrow. Got:\n{rs}"
    );
}

#[test]
fn test_library_multipass_readonly_param_not_mut_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "behavior_tree/behavior_tree_executor.wj",
        &fixture("behavior_tree_executor.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("behavior_tree/behavior_tree_executor.rs")
        .expect("behavior_tree_executor.rs generated");

    assert!(
        !rs.contains("find_index_by_id(&mut tree"),
        "read-only tree lookup must not mut-borrow owned tree param. Got:\n{rs}"
    );
    assert!(
        rs.contains("find_index_by_id(&tree") || rs.contains("find_index_by_id(tree"),
        "find_index_by_id call must borrow or move tree, not &mut. Got:\n{rs}"
    );
}
