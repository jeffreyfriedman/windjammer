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

    let formal_borrowed = rs.contains("run_query(graph: &Graph")
        || rs.contains("fn run_query(graph: &Graph")
        || rs.contains("graph: &Graph");
    let call_borrowed = rs.contains("run_query(&graph,") || rs.contains("run_query(& graph,");
    let call_bare = rs.contains("run_query(graph,") || rs.contains("run_query(graph ,");
    assert!(
        (formal_borrowed && call_bare && !call_borrowed)
            || call_borrowed,
        "reused graph in loop must borrow consistently for cross-module &Graph callee \
         (formal_borrowed={formal_borrowed}, call_borrowed={call_borrowed}). Got:\n{rs}"
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

#[test]
fn test_library_multipass_csv_while_index_owned_string_param() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "csv/while_index_owned.wj",
        &fixture("csv_while_index_owned_string.wj"),
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("csv/while_index_owned.rs")
        .expect("while_index_owned.rs generated");

    assert!(
        !rs.contains("parse_vertex_line(&line)"),
        "owned string formal must receive line by value, not &line. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

/// Owned / demoted WJ `string` reused into a method `&str` formal must borrow
/// (`loader.load(&path)`), not move or clone (`path` / `path.clone()`).
#[test]
fn test_library_multipass_owned_string_to_string_method_must_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "assets/loader.wj",
        r#"
pub struct AssetLoader {
    loads: int,
}

impl AssetLoader {
    pub fn new() -> AssetLoader {
        AssetLoader { loads: 0 }
    }

    pub fn load(self, path: string) -> int {
        self.loads = self.loads + 1
        strings::len(path)
    }
}
"#,
    );
    test.add_file(
        "assets/runner.wj",
        r#"
use crate::assets::loader::AssetLoader

pub fn load_twice(path: string) -> int {
    let mut loader = AssetLoader::new()
    let a = loader.load(path)
    let b = loader.load(path)
    a + b
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("assets/runner.rs")
        .expect("runner.rs generated");

    assert!(
        !rs.contains("path.clone()"),
        "must not clone reused string into &str method formal. Got:\n{rs}"
    );
    let load_lines: Vec<&str> = rs
        .lines()
        .filter(|l| l.contains("loader.load("))
        .collect();
    assert!(
        !load_lines.is_empty(),
        "expected loader.load call sites. Got:\n{rs}"
    );
    // Accept either shape:
    // - owned `path: String` + `load(&path)` into demoted `&str` formal
    // - demoted `path: &str` + `load(path)` (already a shared borrow; `&path` would be `&&str`)
    let path_already_str_ref = rs.contains("path: &str") || rs.contains("path:&str");
    for line in &load_lines {
        let ok = line.contains("load(&path)")
            || line.contains("load(& path)")
            || (path_already_str_ref
                && (line.contains("load(path)") || line.contains("load( path)")));
        assert!(
            ok,
            "expected borrow-safe path at demoted string formal call site.\nLine: {line}\nFull:\n{rs}"
        );
    }

    test.assert_compiles_without_error();
}


/// WDB-084: `map = f(map, k, v)` writeback must not clone map.
#[test]
fn test_library_multipass_map_writeback_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/map_writeback.wj",
        r#"
use std::collections::HashMap

fn put_entry(map: HashMap<string, int>, key: string, value: int) -> HashMap<string, int> {
    map.insert(key, value)
    map
}

pub fn build_scores() -> HashMap<string, int> {
    let mut map: HashMap<string, int> = HashMap::new()
    map = put_entry(map, "a", 1)
    map = put_entry(map, "b", 2)
    map
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/map_writeback.rs")
        .expect("map_writeback.rs generated");

    assert!(
        !rs.contains("map.clone()"),
        "map = put_entry(map, …) writeback must not clone map. Got:\n{rs}"
    );
    assert!(
        rs.contains("put_entry(map,")
            || rs.contains("put_entry(map ,")
            || rs.contains("put_entry(&mut map,")
            || rs.contains("put_entry(&mut map ,"),
        "expected move or &mut writeback of map into put_entry (no clone). Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

/// WDB-086: HashMap::get borrow-break must emit exactly one `.copied()` on Copy V.
#[test]
fn test_library_multipass_hashmap_get_borrow_break_single_copied() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/vertex_u32_map.wj",
        r#"
use std::collections::HashMap

pub struct GraphVertexU32Map {
    pub inner: HashMap<i64, u32>,
}

impl GraphVertexU32Map {
    pub fn inc(self, label: i64) {
        if self.inner.contains_key(label) {
            if let Some(count) = self.inner.get(label) {
                self.inner.insert(label, count + 1)
                return
            }
        }
        self.inner.insert(label, 1)
    }
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/vertex_u32_map.rs")
        .expect("vertex_u32_map.rs generated");

    assert!(
        !rs.contains(".copied().copied()"),
        "HashMap::get borrow-break must not double .copied() on Copy V. Got:\n{rs}"
    );
    assert!(
        rs.contains(".copied()"),
        "HashMap::get on Copy V should use .copied() once. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

/// WDB-087: tuple restore `let t = f(v, w); v = t.0; w = t.1` must not clone heap Vecs.
#[test]
fn test_library_multipass_tuple_writeback_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/tuple_writeback.wj",
        r#"
pub fn push_pair(costs: Vec<f64>, verts: Vec<i64>, cost: f64, vertex: i64) -> (Vec<f64>, Vec<i64>) {
    let mut c = costs
    let mut v = verts
    c.push(cost)
    v.push(vertex)
    (c, v)
}

pub fn grow_heap(n: int) -> (Vec<f64>, Vec<i64>) {
    let mut costs: Vec<f64> = Vec::new()
    let mut verts: Vec<i64> = Vec::new()
    let mut i = 0
    while i < n {
        let pushed = push_pair(costs, verts, i as f64, i as i64)
        costs = pushed.0
        verts = pushed.1
        i = i + 1
    }
    (costs, verts)
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/tuple_writeback.rs")
        .expect("tuple_writeback.rs generated");

    assert!(
        !rs.contains("costs.clone()") && !rs.contains("verts.clone()"),
        "tuple writeback after push_pair must not clone heap vectors. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}

/// WDB-089: Rust `HashMap::with_capacity` takes `usize`. Multipass must not cast
/// a `usize` arg to `i64` (WJ std used to declare `capacity: int`).
#[test]
fn test_library_multipass_hashmap_with_capacity_usize_no_i64_cast() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/vertex_capacity.wj",
        r#"
use std::collections::HashMap

pub fn make_f64_map(n: usize) -> HashMap<i64, f64> {
    HashMap::with_capacity(n)
}

pub fn make_f64_map_from_int(n: int) -> HashMap<i64, f64> {
    HashMap::with_capacity(n)
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/vertex_capacity.rs")
        .expect("vertex_capacity.rs generated");

    assert!(
        !rs.contains("with_capacity(n as i64)"),
        "usize capacity must not cast to i64 for Rust HashMap. Got:\n{rs}"
    );
    assert!(
        rs.contains("with_capacity(n)")
            || rs.contains("with_capacity(n as usize)"),
        "usize capacity should pass through (or cast int→usize only). Got:\n{rs}"
    );
    // int overload must reach Rust as usize
    assert!(
        rs.contains("with_capacity(n as usize)")
            || rs.matches("with_capacity(n)").count() >= 2,
        "int capacity must coerce to usize for Rust HashMap. Got:\n{rs}"
    );

    test.assert_compiles_without_error();
}
