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

//! Regression: module file copy enum formal vec push must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Key {
    W,
    A,
    S,
    D,
    Space,
    Escape,
    Unknown,
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub struct Input {
    pub keys_down: Vec<Key>,
    pub keys_pressed: Vec<Key>,
    pub mouse_buttons_down: Vec<MouseButton>,
}

impl Input {
    pub fn new() -> Input {
        Input {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),
            mouse_buttons_down: Vec::new(),
        }
    }

    pub fn press_key(self, key: Key) {
        self.keys_down.push(key)
        self.keys_pressed.push(key)
    }

    pub fn press_mouse_button(self, button: MouseButton) {
        self.mouse_buttons_down.push(button)
    }
}
"#;

#[test]
fn module_file_copy_enum_formal_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("key.clone()");
    assert!(
        !bad,
        "RED: Copy unit enum formal cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}
