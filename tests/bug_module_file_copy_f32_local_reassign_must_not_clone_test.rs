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

//! Copy `f32` **local** into **local reassignment** must not `.clone()`.
//!
//! Product `frame_analysis.rs` histogram loop:
//!   `min_val = lum.clone()` / `max_val = lum.clone()`
//! WJ uses bare `lum`.
//! Distinct from (f32 **formal** local reassign), (f32 formal into **let**),
//! (f32 local into **struct literal**), and (f32 formal into **field**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn track_local_extrema(samples: Vec<f32>) -> f32 {
    let mut min_val = 1000.0
    let mut max_val = -1000.0
    let mut i = 0
    while i < samples.len() {
        let lum = samples[i]
        if lum < min_val {
            min_val = lum
        }
        if lum > max_val {
            max_val = lum
        }
        i = i + 1
    }
    max_val - min_val
}
"#;

#[test]
fn module_file_copy_f32_local_reassign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("lum.clone()");
    assert!(
        !bad,
        "Copy f32 local cloned into local reassignment:\n{rs}"
    );
    let _ = test.cargo_check();
}
