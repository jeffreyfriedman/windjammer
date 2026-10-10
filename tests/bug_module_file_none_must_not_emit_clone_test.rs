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

//! `None` must not emit `None.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Sprite {
    pub src_x: Option<f32>,
    pub src_y: Option<f32>,
}

pub struct StateId {
    pub id: i32,
}

pub struct Machine {
    pub current_state: Option<StateId>,
    pub previous_state: Option<StateId>,
    pub initial_state: Option<StateId>,
}

pub struct Tile {
    pub solid: bool,
}

pub fn new_sprite() -> Sprite {
    Sprite {
        src_x: None,
        src_y: None,
    }
}

pub fn clear(s: Sprite) {
    s.src_x = None
}

pub fn clear_machine(m: Machine) {
    m.current_state = None
    m.previous_state = None
    m.initial_state = None
}

pub fn fill_tiles(tiles: Vec<Option<Tile>>) -> Vec<Option<Tile>> {
    let mut tiles = tiles
    let mut i = 0
    while i < 3 {
        tiles.push(None)
        i = i + 1
    }
    tiles
}

pub enum Value {
    None,
    Float(f32),
}

pub struct Node {
    pub inputs: Vec<Value>,
}

impl Node {
    pub fn set_input(self, index: usize, value: Value) {
        while self.inputs.len() <= index {
            self.inputs.push(Value::None)
        }
        self.inputs[index] = value
    }
}
"#;

#[test]
fn module_file_none_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("None.clone()"),
        "RED None emitted with .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
