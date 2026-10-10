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

//! `.clone().clone()`. One clone into the call is enough.
//!
//!   `let pass = sorted[step.pass_indices[pi]].clone();`
//!   `self.bind_pass_buffers(pass.clone().clone())`
//!   then `pass.indirect_args_buffer`
//! WJ is `self.bind_pass_buffers(pass)` and the field reads come after.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct CompiledPass {
    pub enabled: bool,
    pub shader_id: int,
    pub indirect: int,
}

pub struct Exec {
    pub n: int,
}

impl Exec {
    pub fn bind(self, pass: CompiledPass) {
        self.n = pass.shader_id
    }

    pub fn run(self, passes: Vec<CompiledPass>) {
        for i in 0..passes.len() {
            let pass = passes[i]
            if pass.enabled {
                self.bind(pass)
                if pass.indirect != 0 {
                    self.n = pass.shader_id
                }
            }
        }
    }
}
"#;

#[test]
fn module_file_reused_local_call_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED: reused local double-cloned into call:\n{rs}"
    );
    let _ = test.cargo_check();
}

