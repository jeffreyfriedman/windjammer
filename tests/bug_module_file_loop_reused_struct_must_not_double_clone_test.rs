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

//! Regression: module file loop reused struct must not double clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Message {
    pub text: string,
}

pub struct Squad {
    pub id: string,
}

impl Squad {
    pub fn send_message(self, message: Message) {
        let _kept = message.text
    }
}

pub struct Manager {
    pub squads: Vec<Squad>,
}

impl Manager {
    pub fn broadcast(self, message: Message) {
        for squad in self.squads {
            squad.send_message(message)
        }
    }
}
"#;

#[test]
fn module_file_loop_reused_struct_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED: loop-reused struct double-cloned into send:\n{rs}"
    );
    let _ = test.cargo_check();
}
