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

//! borrowed `string` into `&str` formal must not emit `&*ident`.
//!
//! Tip output for a game project `dialogue_tree.rs`:
//!   `world.get_flag(&*flag)` / `world.get_variable(&*key)`
//! Prefer `world.get_flag(flag)` / `world.get_variable(key)` (auto-deref / coerce).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct World {
    pub ok: bool,
}

impl World {
    pub fn get_flag(self, flag: string) -> bool {
        self.ok
    }
}

pub enum Condition {
    QuestFlag { flag: string, required: bool },
}

impl Condition {
    pub fn evaluate(self, world: World) -> bool {
        match self {
            Condition::QuestFlag { flag, required } => {
                let flag_value = world.get_flag(flag)
                flag_value == required
            },
        }
    }
}
"#;

#[test]
fn module_file_string_must_not_emit_star_deref_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("&*"),
        "emitted &* deref-ref:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
