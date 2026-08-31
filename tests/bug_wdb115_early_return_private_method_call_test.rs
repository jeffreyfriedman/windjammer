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

//! WDB-115: early `return self.private_method()` must emit the called method, not a sibling.
//!
//! WindjammerDB wdb-reducer `reducer_runtime.wj` (Phase 220 regen):
//!   `return self.handle_reducer_panic()` in `invoke_reducer`
//! mis-emits as:
//!   `return self.target_label();` (E0308: expected `ReducerOutcome`, found `String`)

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use windjammer::build_project;
use windjammer::CompilationTarget;

const SOURCE: &str = r#"
pub struct Outcome {
    pub code: i64,
    pub label: string,
}

pub struct Runtime {
    pub panic_safe: bool,
}

impl Runtime {
    pub fn target_label(self) -> string {
        "native"
    }

    fn handle_reducer_panic(self) -> Outcome {
        Outcome {
            code: 1,
            label: "panic",
        }
    }

    pub fn invoke(self, simulate_panic: bool) -> Outcome {
        if simulate_panic {
            return self.handle_reducer_panic()
        }
        Outcome {
            code: 0,
            label: self.target_label(),
        }
    }
}
"#;

#[test]
fn wdb115_early_return_private_method_must_emit_correct_callee() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("wdb115_runtime.wj");
    fs::write(&src, SOURCE).expect("write source");
    let out = tmp.path().join("out");
    build_project(&src, &out, CompilationTarget::Rust, false).expect("WDB-115 compile should succeed");
    let rs_path = out.join("wdb115_runtime.rs");
    let rust = fs::read_to_string(&rs_path).expect("read generated rs");
    assert!(
        rust.contains("handle_reducer_panic"),
        "WDB-115: generated Rust must call handle_reducer_panic. Got:\n{rust}"
    );
    assert!(
        !rust.contains("return self.target_label()"),
        "WDB-115 RED: early return must not emit sibling target_label(). Got:\n{rust}"
    );
}
