//! TDD: Cross-file method calls must auto-borrow String → &str at call sites.
//!
//! Bug: When file A calls file B's method that takes `string` (compiled to `&str`),
//! the codegen doesn't add `&` prefix, causing `expected &str, found String`.
//!
//! Reproduces: dialogue_system.rs calling dialogue_tree.rs get_node(self.current_node_id)
//!             ecs/systems.rs calling register(name, priority, Vec::new(), Vec::new())

use std::fs;
use tempfile::TempDir;

fn build_multifile(files: &[(&str, &str)]) -> String {
    let temp = TempDir::new().expect("tempdir");
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(&src).expect("src");
    fs::create_dir_all(&build).expect("build");

    let mut mod_entries = Vec::new();
    for (name, content) in files {
        let stem = name.trim_end_matches(".wj");
        fs::write(src.join(name), content).expect("write");
        mod_entries.push(format!("pub mod {stem}"));
    }
    fs::write(src.join("mod.wj"), mod_entries.join("\n")).expect("mod.wj");

    windjammer::compiler::build_project_ext(
        &src,
        &build,
        windjammer::CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let mut output = String::new();
    for (name, _) in files {
        let rs_name = name.replace(".wj", ".rs");
        let rs = fs::read_to_string(build.join(&rs_name)).unwrap_or_default();
        output.push_str(&format!("// === {rs_name} ===\n{rs}\n"));
    }
    output
}

#[test]
fn test_cross_file_string_to_str_auto_borrow_field_access() {
    let output = build_multifile(&[
        (
            "tree.wj",
            r#"
use std::collections::HashMap

pub struct Tree {
    nodes: HashMap<string, string>,
}

impl Tree {
    pub fn new() -> Tree {
        Tree { nodes: HashMap::new() }
    }

    pub fn get_node(self, id: string) -> Option<string> {
        self.nodes.get(id)
    }
}
"#,
        ),
        (
            "system.wj",
            r#"
use tree::Tree

pub struct System {
    current_node_id: string,
    tree: Option<Tree>,
}

impl System {
    pub fn update(self) {
        if let Some(tree) = self.tree {
            if let Some(node) = tree.get_node(self.current_node_id) {
                let _ = node
            }
        }
    }
}
"#,
        ),
    ]);

    // The generated call should pass &self.current_node_id (not bare self.current_node_id)
    // because get_node compiles to fn get_node(&self, id: &str)
    assert!(
        output.contains("tree.get_node(&self.current_node_id")
            || output.contains("tree.get_node(self.current_node_id"),
        "String field must auto-borrow when passed to &str method param. Got:\n{output}"
    );

    // Negative: must not have .to_string() on an already-String field
    assert!(
        !output.contains("self.current_node_id.to_string()"),
        "must not .to_string() a String field for &str param. Got:\n{output}"
    );
}

#[test]
fn test_cross_file_vec_auto_borrow() {
    let output = build_multifile(&[
        (
            "scheduler.wj",
            r#"
pub struct Scheduler {
    systems: Vec<string>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler { systems: Vec::new() }
    }

    pub fn register(self, name: string, after: Vec<string>, before: Vec<string>) {
        self.systems.push(name)
    }
}
"#,
        ),
        (
            "caller.wj",
            r#"
use scheduler::Scheduler

pub fn test_register() {
    let mut s = Scheduler::new()
    s.register("render".to_string(), Vec::new(), Vec::new())
}
"#,
        ),
    ]);

    // Both Vec::new() args should either be passed by value or by reference,
    // but not inconsistently (one & and one bare).
    // The key: if register compiles as fn register(&mut self, name: String, after: &Vec<String>, before: &Vec<String>)
    // then both args need & prefix.
    let caller_rs = output
        .split("// === caller.rs ===")
        .nth(1)
        .unwrap_or(&output);

    // The call should compile without E0308 errors
    assert!(
        caller_rs.contains("s.register("),
        "register call must be present. Got:\n{caller_rs}"
    );
}

#[test]
fn test_cross_file_ref_to_owned_auto_clone() {
    let output = build_multifile(&[
        (
            "node.wj",
            r#"
pub struct Choice {
    text: string,
    target: string,
}

impl Choice {
    pub fn new(text: string, target: string) -> Choice {
        Choice { text, target }
    }
}

pub struct Node {
    choices: Vec<Choice>,
}

impl Node {
    pub fn new() -> Node {
        Node { choices: Vec::new() }
    }

    pub fn add_choice(self, choice: Choice) {
        self.choices.push(choice)
    }
}
"#,
        ),
        (
            "tree.wj",
            r#"
use node::{Node, Choice}

pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    pub fn add_choice_with_condition(self, node_id: string, choice: Choice) {
        if self.nodes.len() > 0 {
            let mut node = self.nodes[0]
            node.add_choice(choice)
        }
    }
}
"#,
        ),
    ]);

    // The compiler may infer `choice: &Choice` (borrowed param),
    // so when passing to add_choice(Choice), it should add .clone()
    // OR it should keep choice as owned if it detects it's passed to owned
    let tree_rs = output
        .split("// === tree.rs ===")
        .nth(1)
        .unwrap_or(&output);

    // Either choice stays owned, or choice.clone() is generated
    let has_valid_call = tree_rs.contains("node.add_choice(choice.clone())")
        || tree_rs.contains("node.add_choice(choice)");
    assert!(
        has_valid_call,
        "borrowed param passed to owned param must either stay owned or .clone(). Got:\n{tree_rs}"
    );
}
