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

//! Regression: module file owned enum must not clone before as float.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Copy-only enum (legacy isolate) — still must not clone.
const SRC_COPY: &str = r#"
pub enum Val {
    F(f32),
    I(i32),
}

impl Val {
    pub fn as_float(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::I(v) => v as f32,
        }
    }
}

pub fn add(a: Val, b: Val) -> f32 {
    a.as_float() + b.as_float()
}
"#;

/// Shape: non-Copy enum + method + exclusive match arms.
const SRC_ENUM: &str = r#"
pub enum Val {
    F(f32),
    S(string),
}

impl Val {
    pub fn as_float(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::S(_) => 0.0,
        }
    }
}

pub enum Op {
    Add,
    Sub,
    Neg,
}

pub struct Eval {
    pub last: f32,
}

impl Eval {
    pub fn evaluate(self, op: Op, a: Val, b: Val) -> f32 {
        match op {
            Op::Add => {
                let fa = a.as_float()
                let fb = b.as_float()
                fa + fb
            },
            Op::Sub => {
                let fa = a.as_float()
                let fb = b.as_float()
                fa - fb
            },
            Op::Neg => {
                -a.as_float()
            },
        }
    }
}
"#;

fn assert_no_clone_before_as_float(rs: &str, label: &str) {
    let demoted = rs.contains("a: &Val") || rs.contains("b: &Val");
    let cloned = rs.contains(".clone().as_float()")
        || rs.contains("a.clone()")
        || rs.contains("b.clone()");
    assert!(
        !demoted && !cloned,
        "RED ({label}): owned enum demoted/cloned before as_float:\n{rs}"
    );
}

#[test]
fn module_file_owned_enum_must_not_clone_before_as_float() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_COPY);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile (copy) lib.rs:\n{rs}");
    assert_no_clone_before_as_float(rs, "copy-only");
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_noncopy_enum_match_must_not_demote_before_as_float() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_ENUM);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert_no_clone_before_as_float(rs, "noncopy+match");
    assert!(
        rs.contains("a: Val") && rs.contains("b: Val"),
        "RED: evaluate formals must stay owned Val:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
