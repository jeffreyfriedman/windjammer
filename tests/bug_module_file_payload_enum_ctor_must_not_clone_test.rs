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

//! a payload enum constructor used once must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Value {
    Int(i32),
    Float(f32),
}

fn take(value: Value) -> i32 {
    match value {
        Value::Int(n) => n,
        Value::Float(_) => 0,
    }
}

pub fn go() -> i32 {
    take(Value::Int(0))
}
"#;

#[test]
fn module_file_payload_enum_ctor_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("Value::Int(0).clone()") && !rs.contains(".clone()"),
        "RED payload enum constructor cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}

const METHOD_SRC: &str = r#"
pub enum Value {
    Int(i32),
    String(string),
    None,
}

pub struct Runner {
    pub n: i32,
}

impl Runner {
    pub fn set_variable(self, name: string, value: Value) {
        match value {
            Value::Int(v) => {
                self.n = v
            },
            _ => {
                self.n = 0
            },
        }
    }
}

pub fn go() -> i32 {
    let mut runner = Runner { n: 1 }
    runner.set_variable("score", Value::Int(0))
    runner.n
}
"#;

#[test]
fn method_noncopy_payload_enum_ctor_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", METHOD_SRC);
    let map = test.compile().expect("method compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("method lib.rs:\n{rs}");
    assert!(
        !rs.contains("Value::Int(0).clone()") && !rs.contains(".clone()"),
        "RED method payload enum constructor cloned:\n{rs}"
    );
}
