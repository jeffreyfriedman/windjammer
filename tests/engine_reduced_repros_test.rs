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

const D_SHAPE: &str = r#"pub enum ShapeKind {
    Sphere(f32),
    Hull(Vec<f32>),
}

pub struct Body {
    pub shape: ShapeKind,
    pub mass: f32,
}
"#;

fn d_world(methods: &str) -> String {
    let src = format!("use crate::shape::Body\n\npub struct World {{\n    pub handle: u64,\n}}\n\nimpl World {{\n{methods}\n}}\n");
    gen(&[("shape.wj", D_SHAPE), ("world.wj", &src)], "world.rs")
}

/// D: `if let` over a by-reference scrutinee: the Copy payload goes into a tuple by value, and the names the tuple
/// pattern binds are values too (no `*a` later).
#[test]
fn test_if_let_copy_payload_in_tuple_is_dereferenced_and_its_bindings_are_values() {
    let generated = d_world(
        r#"    pub fn go(self, body: Body) -> f32 {
        if let crate::shape::ShapeKind::Sphere(r) = body.shape {
            let (a, b) = (r, 2.0)
            return a + b
        }
        0.0
    }"#,
    );
    assert!(generated.contains("(*r, 2.0_f32)"), "payload is dereferenced into the tuple. Got:\n{generated}");
    assert!(generated.contains("return a + b"), "`a` is an f32 value, not a reference. Got:\n{generated}");
}

/// D: `while let` over a by-reference scrutinee.
#[test]
fn test_while_let_copy_payload_in_tuple_is_dereferenced() {
    let generated = d_world(
        r#"    pub fn go(self, body: Body) -> f32 {
        let mut total = 0.0
        let mut i = 0
        while let crate::shape::ShapeKind::Sphere(r) = body.shape {
            let (a, b) = (r, 1.0)
            total = total + a + b
            i = i + 1
            if i > 2 {
                break
            }
        }
        total
    }"#,
    );
    assert!(generated.contains("(*r, 1.0_f32)"), "payload is dereferenced into the tuple. Got:\n{generated}");
    assert!(!generated.contains("*a"), "`a` is an f32 value. Got:\n{generated}");
}

/// D: a tuple nested in a tuple, in a match arm, next to an arm that binds the non-Copy payload.
#[test]
fn test_nested_tuple_arm_copy_payload_is_dereferenced_next_to_non_copy_arm() {
    let generated = d_world(
        r#"    pub fn go(self, body: Body) -> f32 {
        let pair = match body.shape {
            crate::shape::ShapeKind::Sphere(r) => ((r, body.mass), body.mass),
            crate::shape::ShapeKind::Hull(points) => ((points.len() as f32, body.mass), body.mass),
        }
        pair.0.0 + pair.1
    }"#,
    );
    assert!(generated.contains("((*r, body.mass), body.mass)"), "the inner tuple holds the value. Got:\n{generated}");
}

/// D: the same enum matched by value (an owned local, bindings are values) keeps by-value bindings: no deref is added.
#[test]
fn test_owned_scrutinee_copy_payload_in_tuple_needs_no_deref() {
    let generated = d_world(
        r#"    pub fn go(self) -> f32 {
        let kind = crate::shape::ShapeKind::Sphere(2.0)
        let (x, y) = match kind {
            crate::shape::ShapeKind::Sphere(r) => (r, 0.0),
            crate::shape::ShapeKind::Hull(_) => (1.0, 1.0),
        }
        x + y
    }"#,
    );
    assert!(!generated.contains("*r"), "an owned scrutinee binds values. Got:\n{generated}");
}

fn derive_line_before(rs: &str, decl: &str) -> String {
    let at = rs.find(decl).unwrap_or_else(|| panic!("{decl} in:\n{rs}"));
    rs[..at].lines().rev().take(3).collect::<Vec<_>>().join("\n")
}

fn assert_derives_debug_clone(rs: &str, decl: &str) {
    let before = derive_line_before(rs, decl);
    assert!(before.contains("Debug") && before.contains("Clone"), "{decl} must derive Debug and Clone. Got:\n{rs}");
}

const F_GRAPH: &str = "pub struct Connection {\n    from_node: u32,\n    to_node: u32,\n}\n\npub struct EventGraph {\n    connections: Vec<Connection>,\n}\n";

/// F: a chain of three files; every struct derives Debug and Clone, whichever file the compiler reaches first.
#[test]
fn test_derives_agree_over_a_three_file_chain() {
    for names in [["a_graph", "b_mid", "c_top"], ["c_graph", "b_mid", "a_top"], ["b_graph", "c_mid", "a_top"]] {
        let [g, m, t] = names;
        let mid = format!("use crate::{g}::EventGraph\n\npub struct Mid {{\n    pub graph: EventGraph,\n}}\n");
        let top = format!("use crate::{m}::Mid\n\npub struct Top {{\n    pub mid: Mid,\n}}\n");
        let (map, ok) = test_utils::compile_project_dir(&[(&format!("{g}.wj"), F_GRAPH), (&format!("{m}.wj"), &mid), (&format!("{t}.wj"), &top)]);
        assert!(ok, "{names:?} must transpile");
        assert_derives_debug_clone(&map[&format!("{g}.rs")], "pub struct EventGraph");
        assert_derives_debug_clone(&map[&format!("{m}.rs")], "pub struct Mid");
        assert_derives_debug_clone(&map[&format!("{t}.rs")], "pub struct Top");
    }
}

/// F: a struct that cannot derive Clone (it holds a trait object) in one file keeps its dependents in other files from
/// deriving it, in either file order.
#[test]
fn test_a_struct_holding_a_trait_object_blocks_the_derive_of_its_holder_in_another_file() {
    for (scene_file, game_file) in [("a_scene", "z_game"), ("z_scene", "a_game")] {
        let scene = "pub trait Drawable {\n    fn draw(self)\n}\n\npub struct Scene {\n    pub items: Vec<Box<dyn Drawable>>,\n}\n";
        let game = format!("use crate::{scene_file}::Scene\n\npub struct Game {{\n    pub scene: Scene,\n}}\n");
        let (map, ok) = test_utils::compile_project_dir(&[(&format!("{scene_file}.wj"), scene), (&format!("{game_file}.wj"), &game)]);
        assert!(ok);
        let game_rs = &map[&format!("{game_file}.rs")];
        let before = derive_line_before(game_rs, "pub struct Game");
        assert!(!before.contains("Clone"), "Game holds a Scene that is not Clone ({scene_file}/{game_file}). Got:\n{game_rs}");
    }
}

/// F: two structs that hold each other through an `Option<Box<..>>` (a cycle) in two files both derive.
#[test]
fn test_derives_agree_over_a_cycle_between_two_files() {
    let a = "use crate::b_file::Right\n\npub struct Left {\n    pub next: Option<Box<Right>>,\n    pub id: u32,\n}\n";
    let b = "use crate::a_file::Left\n\npub struct Right {\n    pub back: Option<Box<Left>>,\n    pub id: u32,\n}\n";
    let (map, ok) = test_utils::compile_project_dir(&[("a_file.wj", a), ("b_file.wj", b)]);
    assert!(ok);
    assert_derives_debug_clone(&map["a_file.rs"], "pub struct Left");
    assert_derives_debug_clone(&map["b_file.rs"], "pub struct Right");
}

/// F: a field of a generic type over a struct of another file: the generic holder and the struct that holds it derive.
#[test]
fn test_derives_agree_for_a_field_of_a_generic_type() {
    let graph = format!("{F_GRAPH}\npub struct Holder<T> {{\n    pub items: Vec<T>,\n}}\n");
    let user = "use crate::a_graph::EventGraph\nuse crate::a_graph::Holder\n\npub struct Runner {\n    pub held: Holder<EventGraph>,\n}\n";
    let (map, ok) = test_utils::compile_project_dir(&[("a_graph.wj", &graph), ("b_user.wj", user)]);
    assert!(ok);
    assert_derives_debug_clone(&map["a_graph.rs"], "pub struct EventGraph");
    assert_derives_debug_clone(&map["b_user.rs"], "pub struct Runner");
}

/// F: a user struct named like a runtime type that is not Clone (`Connection`) is the user's type.
#[test]
fn test_user_struct_named_like_a_runtime_non_clone_type_derives_clone_in_every_file() {
    let user = "use crate::z_graph::Connection\n\npub struct Link {\n    pub conn: Connection,\n}\n";
    let (map, ok) = test_utils::compile_project_dir(&[("a_link.wj", user), ("z_graph.wj", F_GRAPH)]);
    assert!(ok);
    assert_derives_debug_clone(&map["a_link.rs"], "pub struct Link");
}
