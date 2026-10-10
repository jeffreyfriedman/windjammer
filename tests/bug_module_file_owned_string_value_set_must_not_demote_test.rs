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

//! owned `string` + value formals that store into fields must stay owned.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Val {
    N(i32),
}

pub struct Binding {
    pub name: string,
    pub value: Val,
}

pub struct Scope {
    pub bindings: Vec<Binding>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope { bindings: Vec::new() }
    }

    pub fn set(self, name: string, value: Val) {
        let mut i: usize = 0
        while i < self.bindings.len() {
            if self.bindings[i].name == name {
                self.bindings[i].value = value
                return
            }
            i = i + 1
        }
        self.bindings.push(Binding { name: name, value: value })
    }
}

pub struct Runner {
    pub scope: Scope,
}

impl Runner {
    pub fn set_variable(self, name: string, value: Val) {
        self.scope.set(name, value)
    }
}
"#;

#[test]
fn module_file_owned_string_value_set_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let demoted_set = rs.contains("fn set(") && (rs.contains("name: &str")
        || rs.contains("name: &String")
        || rs.contains("value: &Val"));
    let demoted_wrapper = rs.contains("fn set_variable(")
        && (rs.contains("name: &String")
            || rs.contains("name: &str")
            || rs.contains("value: &Val"));
    assert!(
        !demoted_set && !demoted_wrapper,
        "RED owned string/value set() demoted to refs:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
