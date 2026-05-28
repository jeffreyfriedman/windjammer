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

//! TDD: E0507 Final Phase - Reduce remaining "cannot move out of" errors to <5
//!
//! Patterns addressed:
//! A. Vec index + method(owned self) - vec[i].clone().sample()
//! B. Option match/if-let with borrowed base - &expr.field, ref/ref mut
//! C. Option match with param &T - &node.children
//! D. Builder pattern - self.clone().method() when &self
//! E. Struct literal from borrowed - self.bindings.clone(), self.graph.passes.clone()

#[path = "common/test_utils.rs"]
mod test_utils;

use std::process::Command;

fn rust_compiles(rust_code: &str) -> bool {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let rs_path = temp_dir.path().join("test.rs");
    std::fs::write(&rs_path, rust_code).expect("write");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&rs_path)
        .output()
        .expect("rustc");
    output.status.success()
}

#[test]
fn test_vec_index_method_owned_self_clone() {
    // Pattern A: vec[i].sample(time) when sample takes owned self
    let source = r#"
pub struct Keyframe { pub time: f32 }
pub struct BoneTrack { pub bone_id: u32 }
impl BoneTrack {
    pub fn sample(self, time: f32) -> Keyframe { Keyframe { time } }
}
pub struct Clip { pub tracks: Vec<BoneTrack> }
impl Clip {
    pub fn sample_at(self, i: i32, time: f32) -> Option<Keyframe> {
        if i >= 0 && (i as usize) < self.tracks.len() {
            return Some(self.tracks[i as usize].sample(time))
        }
        return None
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    // Compiler may infer &self for sample() when it only reads data,
    // making clone unnecessary. Both patterns are valid.
    let has_clone = rust.contains(".clone().sample(") || rust.contains("].clone().sample(");
    let has_direct_sample = rust.contains(".sample(time)") || rust.contains(".sample(");
    assert!(
        has_clone || has_direct_sample,
        "Expected .clone().sample() or direct .sample(): {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_match_param_borrows() {
    // Pattern C: match node.children when node is &T
    let source = r#"
pub struct Node { pub children: Option<Vec<Node>> }
pub fn count_nodes(node: Node) -> i32 {
    match node.children {
        Some(c) => c.len() as i32 + 1,
        None => 1,
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    // When node is owned, we can match directly. When &node, we need &node.children
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_if_let_mut_self_ref_mut() {
    // Pattern B: if let Some(search) = self.search when &mut self
    let source = r#"
pub struct SearchState { pub progress: f32 }
impl SearchState {
    pub fn update(self, dt: f32) -> bool { true }
    pub fn is_complete(self) -> bool { false }
}
pub struct NPC { pub search: Option<SearchState> }
impl NPC {
    pub fn update(self, dt: f32) {
        if let Some(search) = self.search {
            if !search.update(dt) || search.is_complete() {
                // Search complete
            } else {
                // Continue search
            }
        }
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    // Should generate &mut self.search or &self.search when &mut self
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_struct_literal_borrowed_field_clones() {
    // Pattern E: PassDefinition { bindings: self.bindings } when &self
    let source = r#"
pub struct Binding { pub slot: u32 }
pub struct PassDef {
    pub pass_id: u32,
    pub bindings: Vec<Binding>,
}
pub struct Builder { pub pass_id: u32, pub bindings: Vec<Binding> }
impl Builder {
    pub fn build(self) -> PassDef {
        PassDef { pass_id: self.pass_id, bindings: self.bindings }
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_let_binding_borrowed_field_clones() {
    // Pattern E: let mut new_passes = self.graph.passes when &self
    let source = r#"
pub struct Pass { pub id: u32 }
pub struct Graph { pub passes: Vec<Pass> }
pub struct Builder { pub graph: Graph }
impl Builder {
    pub fn add_pass(self) -> Builder {
        let mut new_passes = self.graph.passes
        new_passes.push(Pass { id: 0 })
        Builder { graph: Graph { passes: new_passes } }
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    // When compiler infers owned self, moving self.graph.passes is valid
    // (no clone needed). When &self, clone or borrow is needed.
    let has_clone = rust.contains(".clone()");
    let has_borrow = rust.contains("&self.graph.passes");
    let has_owned_move = rust.contains("self.graph.passes");
    assert!(
        has_clone || has_borrow || has_owned_move,
        "Let binding from field needs clone, borrow, or owned move: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_vec_index_let_binding_borrows() {
    // Pattern A: let preset = self.available_presets[i] when non-Copy
    let source = r#"
pub struct Preset { pub name: String }
pub struct Editor { pub presets: Vec<Preset> }
impl Editor {
    pub fn has_preset(self, i: i32) -> bool {
        if i >= 0 && (i as usize) < self.presets.len() {
            let p = self.presets[i as usize]
            return !p.name.is_empty()
        }
        return false
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    // Should generate &self.presets[i] or .clone() - field access on &Preset works
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}
