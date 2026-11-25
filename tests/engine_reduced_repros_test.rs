#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Cross-file bugs that only show in a whole-library transpile (modules analysed in name order), reduced from a large
//! tree by scripts/wj/reduce_build.wj and a short hand pass. Each test is the reduced repro plus one assertion on the
//! generated Rust that states what correct output looks like. They FAIL on the current compiler (this repo has no
//! xfail convention for Rust tests, so they stay plain failing tests until the compiler fix lands).
//! Sources: the same files live in private-docs handoffs (engine-run/reduced/<id>/).

#[path = "common/test_utils.rs"]
mod test_utils;

fn gen(files: &[(&str, &str)], out: &str) -> String {
    let (map, ok) = test_utils::compile_project_dir(files);
    assert!(ok, "the project must transpile");
    map.get(out).cloned().unwrap_or_else(|| panic!("no {} in {:?}", out, map.keys().collect::<Vec<_>>()))
}

/// B: a call resolved to a same-named static fn (owned formals) instead of the extension method on the receiver type
/// (borrowed formals), because the extension file imports the caller's file and is analysed second.
#[test]
fn test_extension_method_beats_same_named_static_fn_across_files() {
    let generated = gen(
        &[
            ("world.wj", r#"pub struct Realm {
    pub tick: i32,
}
"#),
            ("a_system.wj", r#"use crate::world::Realm
use crate::b_tree::Effect

pub struct Runner {
    pending: Option<Effect>,
}

impl Realm {
    pub fn set_variable(self, key: string, value: string) {
    }
}
"#),
            ("b_tree.wj", r#"use crate::world::Realm

pub enum Effect {
    SetVariable { key: string, value: string },
}

impl Effect {
    pub fn set_variable(key: string, value: string) -> Effect {
        Effect::SetVariable { key: key, value: value }
    }

    pub fn apply(self, realm: Realm) {
        match self {
            Effect::SetVariable { key, value } => {
                realm.set_variable(key, value)
            }
        }
    }
}
"#),
        ],
        "b_tree.rs",
    );
    assert!(
        generated.contains("realm.set_variable(&key, &value)"),
        "both args go to the &str formals of the receiver's method. Got:\n{}",
        generated
    );
}

/// C: `result.contains(id)` on a Vec whose element type is learned later must take `&id`, even when another type in
/// the file has a user method `contains(self, id: i64)`.
#[test]
fn test_vec_contains_not_resolved_to_user_contains_homonym() {
    let generated = gen(&[("store.wj", r#"pub struct Column {
    pub ids: Vec<i64>,
}

impl Column {
    pub fn contains(self, id: i64) -> bool {
        id >= 0
    }

    pub fn entities(self) -> Vec<i64> {
        self.ids
    }
}

pub struct Registry {
    pub columns: Vec<Column>,
}

impl Registry {
    pub fn all_ids(self) -> Vec<i64> {
        let mut result = Vec::new()
        let mut i = 0
        while i < self.columns.len() {
            let ids = self.columns[i].entities()
            let mut j = 0
            while j < ids.len() {
                let id = ids[j]
                if !result.contains(id) {
                    result.push(id)
                }
                j = j + 1
            }
            i = i + 1
        }
        result
    }
}
"#)], "store.rs");
    assert!(
        generated.contains("result.contains(&id)"),
        "Vec::contains takes a reference. Got:\n{}",
        generated
    );
}

/// D: a Copy payload bound under `match &body.shape` is a `&f32`; used by value in a tuple arm it must be dereferenced.
#[test]
fn test_match_ref_binding_used_by_value_in_tuple_arm_is_dereferenced() {
    let generated = gen(&[("shape.wj", r#"pub enum ShapeKind {
    Sphere(f32),
    Hull(Vec<f32>),
}

pub struct Body {
    pub shape: ShapeKind,
    pub mass: f32,
}
"#), ("world.wj", r#"use crate::shape::Body

pub struct World {
    pub handle: u64,
}

impl World {
    pub fn add_body(self, body: Body) -> f32 {
        let (sx, sy) = match body.shape {
            crate::shape::ShapeKind::Sphere(r) => (r, 0.0),
            crate::shape::ShapeKind::Hull(_) => (1.0, 1.0),
        }
        sx + sy + body.mass
    }
}
"#)], "world.rs");
    assert!(
        generated.contains("(*r, 0.0_f32)") || !generated.contains("match &body.shape"),
        "the arm yields (f32, f32), not (&f32, f32). Got:\n{}",
        generated
    );
}

/// E: a string literal passed to an owned `String` formal needs `.to_string()`, also when the caller's file is analysed
/// before the callee's file and a user method named `push` exists elsewhere.
#[test]
fn test_literal_into_owned_string_formal_caller_file_first() {
    let generated = gen(
        &[("a_user.wj", r#"pub struct Greeter {
    volume: f32,
}

impl Greeter {
    pub fn announce(self, ctx: Host) {
        ctx.note_item("ready")
    }
}
"#), ("core.wj", r#"pub struct Host {
    registry: Vec<string>,
}

impl Host {
    pub fn note_item(self, name: string) {
        self.registry.push(name)
    }
}
"#), ("stack.wj", r#"pub struct CallStack {
    frames: Vec<string>,
}

impl CallStack {
    pub fn push(self, frame: string) {
        self.frames.push(frame)
    }
}
"#)],
        "a_user.rs",
    );
    assert!(
        generated.contains("note_item(\"ready\".to_string())"),
        "the formal is an owned String. Got:\n{}",
        generated
    );
}

/// F: a struct whose dependents derive Debug and Clone must itself derive them (E0277 in rustc otherwise).
#[test]
fn test_struct_derives_clone_debug_when_a_dependent_file_derives_them() {
    let (map, ok) = test_utils::compile_project_dir(&[("graph.wj", r#"pub struct Connection {
    from_node: u32,
    to_node: u32,
}

pub struct EventGraph {
    connections: Vec<Connection>,
}

"#), ("runtime.wj", r#"use crate::graph::EventGraph

pub struct GraphRunner {
    pub graph: EventGraph,
}
"#)]);
    assert!(ok);
    let graph = map.get("graph.rs").cloned().unwrap_or_default();
    let runtime = map.get("runtime.rs").cloned().unwrap_or_default();
    let at = graph.find("pub struct EventGraph").expect("EventGraph");
    let before = &graph[at.saturating_sub(120)..at];
    let runner_derives_clone = runtime.contains("derive(Debug, Clone");
    assert!(
        !runner_derives_clone || (before.contains("Clone") && before.contains("Debug")),
        "GraphRunner derives Clone+Debug over an EventGraph that does not. Got graph.rs:\n{}\nruntime.rs:\n{}",
        graph,
        runtime
    );
}
