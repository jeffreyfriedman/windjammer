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

#[test]
fn test_library_multipass_hashmap_i64_key_auto_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/hashmap_i64_bfs_keys.wj",
        &fixture("hashmap_i64_bfs_keys.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/hashmap_i64_bfs_keys.rs")
        .expect("hashmap_i64_bfs_keys.rs generated");

    assert!(
        rs.contains("contains_key(&") && rs.contains(".get(&"),
        "HashMap<i64> keys must auto-borrow in library multipass. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_strings_split_pipe_delimiter() {
    let mut test = MultiFileTest::new();
    test.add_file("csv/split_pipe.wj", &fixture("csv_split_pipe.wj"));

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("csv/split_pipe.rs")
        .expect("csv/split_pipe.rs generated");

    assert!(
        rs.contains("strings::split("),
        "expected strings::split call. Got:\n{rs}"
    );
    assert!(
        !rs.contains("\"|\".to_string()"),
        "pipe delimiter must not coerce to String in library multipass. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_loop_reused_graph_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod loop_query_engine;\npub mod loop_query_runner;\n");
    test.add_file("loop_query_engine.wj", &fixture("loop_query_engine.wj"));
    test.add_file("loop_query_runner.wj", &fixture("loop_query_runner.wj"));

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("loop_query_runner.rs")
        .expect("loop_query_runner.rs generated");

    assert!(
        rs.contains("run_query(&graph,") || rs.contains("run_query(& graph,"),
        "reused graph in loop must borrow for cross-module &Graph callee. Got:\n{rs}"
    );
    assert!(
        !rs.contains("graph.clone()"),
        "must not clone graph each loop iteration. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_hashmap_i64_loop_local_key_auto_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/hashmap_i64_loop_local_key.wj",
        &fixture("hashmap_i64_loop_local_key.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/hashmap_i64_loop_local_key.rs")
        .expect("hashmap_i64_loop_local_key.rs generated");

    assert!(
        rs.contains("contains_key(&n") || rs.contains("contains_key(& n"),
        "loop-local i64 keys must auto-borrow for HashMap::contains_key. Got:\n{rs}"
    );
    assert!(
        !rs.contains("contains_key(n)") || rs.contains("contains_key(&n"),
        "must not pass owned i64 to contains_key. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_hashmap_i64_f64_zero_literal_insert() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/hashmap_i64_f64_zero_literal.wj",
        &fixture("hashmap_i64_f64_zero_literal.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/hashmap_i64_f64_zero_literal.rs")
        .expect("hashmap_i64_f64_zero_literal.rs generated");

    assert!(
        !rs.contains("0.0_f32"),
        "HashMap<i64,f64>::insert must infer f64 for 0.0 literal. Got:\n{rs}"
    );
    assert!(
        rs.contains("0.0_f64") || rs.contains("0.0"),
        "expected f64 zero literal in insert. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_for_in_vertices_reuse_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/for_in_vertices_reuse.wj",
        &fixture("for_in_vertices_reuse.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/for_in_vertices_reuse.rs")
        .expect("for_in_vertices_reuse.rs generated");

    assert!(
        rs.contains("vertex_lookup_len(&vertices") || rs.contains("vertex_lookup_len(& vertices"),
        "for-in loop must borrow vertices when reused in callee. Got:\n{rs}"
    );
    assert!(
        !rs.contains("vertices.clone()"),
        "must not clone vertices each for-in iteration. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_graph_bfs_hashmap_compiles() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/graph_bfs_hashmap.wj",
        &fixture("graph_bfs_hashmap.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/graph_bfs_hashmap.rs")
        .expect("graph_bfs_hashmap.rs generated");

    assert!(
        rs.contains("contains_key(&n") || rs.contains("contains_key(& n"),
        "neighbor_at binding must auto-borrow for HashMap::contains_key. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_library_multipass_csv_for_in_line_string_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "csv/for_in_line.wj",
        &fixture("csv_for_in_line.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let _rs = map
        .get("csv/for_in_line.rs")
        .expect("csv/for_in_line.rs generated");

    test.assert_compiles_without_error();
}
