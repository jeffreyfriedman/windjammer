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
))]

//! TDD: Vec::new() passed to a function expecting &Vec<T> must auto-borrow.
//!
//! Pattern: `scheduler.register("render", 10, Vec::new(), Vec::new())`
//! where formal is `after: Vec<string>, before: Vec<string>` but analyzer
//! infers Borrowed ownership → Rust formal becomes `&Vec<String>`.
//!
//! The compiler must insert `&` at the call site: `&Vec::new()`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vec_new_arg_borrows_when_formal_is_borrowed() {
    let source = r#"
pub struct Scheduler {
    pub tasks: Vec<string>,
}

impl Scheduler {
    pub fn register(self, name: string, priority: int, after: Vec<string>, before: Vec<string>) {
        // body reads after/before but doesn't mutate
        for dep in after {
            self.tasks.push(dep)
        }
    }
}

pub fn setup() {
    let mut scheduler = Scheduler { tasks: Vec::new() }
    scheduler.register("render", 10, Vec::new(), Vec::new())
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Vec::new() passed to borrowed formal must compile.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn string_arg_borrows_when_formal_expects_str_ref() {
    let source = r#"
use std::collections::HashMap

pub struct Registry {
    pub items: HashMap<string, int>,
}

impl Registry {
    pub fn get(self, key: string) -> Option<int> {
        self.items.get(key).copied()
    }
}

pub fn lookup(registry: Registry) -> Option<int> {
    let name = "test"
    registry.get(name)
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "String arg to borrowed formal should auto-borrow.\nGenerated:\n{}",
        generated
    );
}
