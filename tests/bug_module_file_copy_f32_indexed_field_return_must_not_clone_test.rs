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

//! Copy `f32` **indexed field** on return must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct GpuPassTiming {
    pub avg_ms: f32,
}

pub struct GpuProfiler {
    pub pass_timings: Vec<GpuPassTiming>,
}

impl GpuProfiler {
    pub fn timing_avg_ms_at(self, index: int) -> f32 {
        if index >= 0 && index < self.pass_timings.len() {
            return self.pass_timings[index].avg_ms
        }
        0.0
    }
}
"#;

#[test]
fn module_file_copy_f32_indexed_field_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("avg_ms.clone()");
    assert!(
        !bad,
        "RED Copy f32 indexed field cloned on return:\n{rs}"
    );
    let _ = test.cargo_check();
}
