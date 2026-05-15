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

//! E0596 / ownership: methods that call `&mut self` methods on `self.field` must infer `&mut self`.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_method_calling_mut_method_on_sub_object() {
    let source = r#"
struct Keyboard {
    keys: Vec<bool>,
}

impl Keyboard {
    fn update_key(self, key: i32, down: bool) {
        self.keys[0] = down
    }
}

struct Game {
    keyboard: Keyboard,
}

impl Game {
    fn poll_input(self) {
        self.keyboard.update_key(0, true)
    }
}
"#;
    let code = test_utils::compile_single(source);
    assert!(
        code.contains("fn poll_input(&mut self"),
        "poll_input should be &mut self since it calls a mutating method on sub-object. Got:\n{}",
        code
    );
}

#[test]
fn test_match_arm_mut_method_on_sub_object() {
    let source = r#"
struct Companion {
    loyalty: f32,
}

impl Companion {
    fn adjust_loyalty(self, delta: f32) {
        self.loyalty = self.loyalty + delta
    }
}

struct Kestrel {
    companion: Companion,
}

impl Kestrel {
    pub fn respond_to_player_choice(self, choice: i32) {
        match choice {
            0 => self.companion.adjust_loyalty(-5.0),
            1 => self.companion.adjust_loyalty(5.0),
            _ => {}
        }
    }
}
"#;
    let code = test_utils::compile_single(source);
    assert!(
        code.contains("fn respond_to_player_choice(&mut self"),
        "respond_to_player_choice should be &mut self (match arms call mutating sub-object methods). Got:\n{}",
        code
    );
}
