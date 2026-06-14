#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// HashMap lookup helpers must infer Borrowed string keys even when body uses match.
/// Simulates dialogue/tree.wj get_node vs call sites passing &String from Option.
#[test]
fn test_hashmap_get_node_infers_borrowed_string_param() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let node_dir = src.join("dialogue");
    fs::create_dir_all(&node_dir).expect("mkdir");

    fs::write(
        node_dir.join("node.wj"),
        r##"
pub struct DialogueNode {
    pub text: string,
}

impl DialogueNode {
    pub fn text(self) -> string {
        self.text
    }
}
"##,
    )
    .unwrap();

    fs::write(
        node_dir.join("tree.wj"),
        r##"
use super::node::DialogueNode

pub struct DialogueNodeTree {
    nodes: HashMap<string, DialogueNode>,
}

impl DialogueNodeTree {
    pub fn get_node(self, id: string) -> Option<DialogueNode> {
        match self.nodes.get(id) {
            Some(node) => Some(node),
            None => None,
        }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("manager.wj"),
        r##"
use dialogue::tree::DialogueNodeTree

pub struct Manager {
    tree: DialogueNodeTree,
    current_node_id: Option<string>,
}

impl Manager {
    pub fn current_text(self) -> Option<string> {
        if let Some(id) = self.current_node_id {
            if let Some(node) = self.tree.get_node(id) {
                return Some(node.text())
            }
        }
        None
    }
}
"##,
    )
    .unwrap();

    // Stale engine metadata must not override local HashMap key inference.
    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    fs::create_dir_all(&engine_src).expect("mkdir engine");
    fs::write(
        engine_src.join("stub.wj"),
        r##"
pub struct DialogueNodeTree {}

impl DialogueNodeTree {
    pub fn get_node(self, id: string) -> Option<DialogueNodeTree> {
        None
    }
}
"##,
    )
    .unwrap();
    let engine_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("engine build");
    assert!(
        engine_build.status.success(),
        "engine stub build failed:\n{}",
        String::from_utf8_lossy(&engine_build.stderr)
    );

    let out = tmp.path().join("gen");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
            "--metadata",
            &format!("engine={}", engine_gen.join("metadata.json").display()),
        ])
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let tree_rs = fs::read_to_string(out.join("dialogue/tree.rs")).expect("tree.rs");
    assert!(
        tree_rs.contains("id: &String") || tree_rs.contains("id: &str"),
        "get_node id should be borrowed for HashMap lookup. Generated:\n{}",
        tree_rs
    );

    let manager_rs = fs::read_to_string(out.join("manager.rs")).expect("manager.rs");
    // Owned `id` from `if let Some(id) = self.current_node_id` → `get_node(&id)` is valid.
    // Borrowed `id` from `if let Some(id) = &self.current_node_id` → `get_node(id)` is valid.
    assert!(
        manager_rs.contains("get_node(&id)") || manager_rs.contains("get_node(id)"),
        "get_node call must borrow correctly for HashMap key. Generated:\n{}",
        manager_rs
    );
    assert!(
        !manager_rs.contains("get_node(id.clone())"),
        "must not clone string keys for HashMap lookup. Generated:\n{}",
        manager_rs
    );

    let meta = fs::read_to_string(out.join("metadata.json")).expect("metadata.json");
    assert!(
        !meta.contains("VoxelGPURenderer::upload_material_palette")
            && !meta.contains("dialogue::tree::DialogueNodeTree::get_node"),
        "game metadata must not embed full engine registry or module-qualified duplicates.\n{}",
        &meta[..meta.len().min(2000)]
    );
    assert!(
        meta.contains("DialogueNodeTree::get_node"),
        "local get_node signature should be in metadata"
    );
}
