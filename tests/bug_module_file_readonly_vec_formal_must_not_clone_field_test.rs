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

//! Regression: module file readonly vec formal must not clone field.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Meta {
    pub name: string,
}

pub struct Pass {
    pub metas: Vec<Meta>,
}

fn contains_name(metas: Vec<Meta>, needle: string) -> bool {
    for m in metas {
        if m.name == needle {
            return true
        }
    }
    false
}

impl Pass {
    pub fn has_wh(self) -> bool {
        let mut pb = self
        if !contains_name(pb.metas, "w") {
            return false
        }
        if !contains_name(pb.metas, "h") {
            return false
        }
        true
    }
}
"#;

#[test]
fn module_file_readonly_vec_formal_must_not_clone_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned_field = rs.contains("pb.metas.clone()")
        || rs.contains("contains_name(pb.metas.clone()");
    assert!(
        !cloned_field,
        "RED: read-only Vec formal forced field.clone() at call site:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
