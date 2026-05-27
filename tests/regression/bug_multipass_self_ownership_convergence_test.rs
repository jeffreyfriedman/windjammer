#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: Multipass convergence for cross-file self.field.method() mutation
///
/// Bug: In multi-file compilation, File A's method calls self.field.method() where
/// the method is defined in File B. During pass 1, File B's method may not yet be
/// in the registry (or may be registered as Borrowed). File A then infers &self.
/// In pass 2, even though File B is now correctly registered as MutBorrowed,
/// File A already converged to Borrowed and isn't re-checked.
///
/// This creates a stable wrong fixed point in the multipass convergence loop.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_cross_file_self_field_user_method_mut() {
    let files = vec![
        (
            "subsystem.wj",
            r#"
pub struct SubSystem {
    pub counter: i32,
}

impl SubSystem {
    pub fn tick(self) {
        self.counter = self.counter + 1
    }

    pub fn get_value(self) -> i32 {
        return self.counter
    }
}
"#,
        ),
        (
            "orchestrator.wj",
            r#"
use crate::subsystem::SubSystem

pub struct Orchestrator {
    pub sub: SubSystem,
}

impl Orchestrator {
    pub fn update(self) {
        self.sub.tick()
    }

    pub fn read_value(self) -> i32 {
        return self.sub.get_value()
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(&files)
        .expect("Multi-file compilation should succeed");

    let orch_rs = results
        .get("orchestrator.rs")
        .expect("orchestrator.rs should exist");

    assert!(
        orch_rs.contains("fn update(&mut self"),
        "Cross-file: update() calls self.sub.tick() which mutates -- must be &mut self.\nGenerated orchestrator.rs:\n{}",
        orch_rs
    );

    assert!(
        orch_rs.contains("fn read_value(&self"),
        "Cross-file: read_value() calls self.sub.get_value() which only reads -- should be &self.\nGenerated orchestrator.rs:\n{}",
        orch_rs
    );
}

#[test]
fn test_cross_file_transitive_three_levels() {
    let files = vec![
        (
            "leaf.wj",
            r#"
pub struct Leaf {
    pub data: i32,
}

impl Leaf {
    pub fn modify(self) {
        self.data = self.data + 1
    }
}
"#,
        ),
        (
            "branch.wj",
            r#"
use crate::leaf::Leaf

pub struct Branch {
    pub leaf: Leaf,
}

impl Branch {
    pub fn process(self) {
        self.leaf.modify()
    }
}
"#,
        ),
        (
            "root.wj",
            r#"
use crate::branch::Branch

pub struct Root {
    pub branch: Branch,
}

impl Root {
    pub fn execute(self) {
        self.branch.process()
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(&files)
        .expect("Multi-file compilation should succeed");

    let root_rs = results.get("root.rs").expect("root.rs should exist");
    let branch_rs = results.get("branch.rs").expect("branch.rs should exist");

    assert!(
        branch_rs.contains("fn process(&mut self"),
        "branch.process() -> leaf.modify() (mutates) -- must be &mut self.\nGenerated branch.rs:\n{}",
        branch_rs
    );

    assert!(
        root_rs.contains("fn execute(&mut self"),
        "root.execute() -> branch.process() (mutates) -- must propagate &mut self across 3 files.\nGenerated root.rs:\n{}",
        root_rs
    );
}
