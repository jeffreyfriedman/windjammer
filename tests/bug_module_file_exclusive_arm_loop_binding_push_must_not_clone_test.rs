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

//! DB-478: a `for` binding moved into `Vec::push` on exclusive `if let` arms
//! must not `.clone()`, including `.clone().clone()` on the nested arms.
//!
//! Product `dialogue_system.rs` `filter_choices`:
//!   `available.push(choice.clone().clone())` and `available.push(choice.clone())`
//! WJ is `available.push(choice)` once per arm. `choice` comes from
//! `for choice in choices` and is not used again after the push.
//! Distinct from DB-474 (one last-use local, no exclusive arms) and DB-471
//! (a reused `Vec` argument `.clone().clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Choice {
    pub id: int,
}

pub struct World {
    pub flag: int,
}

pub struct Player {
    pub level: int,
}

pub fn filter_choices(choices: Vec<Choice>, world: Option<World>, player: Option<Player>) -> Vec<Choice> {
    let mut available = Vec::new()
    for choice in choices {
        if let Some(w) = world {
            if let Some(p) = player {
                if w.flag > 0 && p.level > 0 {
                    available.push(choice)
                }
            } else {
                available.push(choice)
            }
        } else {
            available.push(choice)
        }
    }
    available
}
"#;

#[test]
fn module_file_exclusive_arm_loop_binding_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("choice.clone()"),
        "exclusive-arm loop binding cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}
