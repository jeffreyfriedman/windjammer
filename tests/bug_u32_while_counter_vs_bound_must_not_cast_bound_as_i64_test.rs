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

//! P3.348: `let mut i = 0` + `while i < count` / `while i < half` must not cast bounds to `i64`
//! when both sides are u32-width (frame_analysis, weight_paint mirror loop).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod frame
pub mod paint
"#;

const FRAME: &str = r#"
pub struct FramePixels {
    pub data: Vec<f32>,
}

impl FramePixels {
    pub fn pixel_count(self) -> u32 {
        self.data.len() as u32 / 4
    }
}

pub fn is_black(frame: FramePixels) -> bool {
    let count = frame.pixel_count()
    let mut i = 0
    while i < count {
        i = i + 1
    }
    count > 0
}
"#;

const PAINT: &str = r#"
pub fn mirror_half_loop(vertex_count: u32) {
    let n = vertex_count
    let half = n / 2
    let mut i = 0
    while i < half {
        i = i + 1
    }
}
"#;

fn bad_i64_bound(rs: &str) -> bool {
    rs.contains("as i64") || rs.contains("_i64)")
}

#[test]
fn u32_while_counter_vs_bound_must_not_cast_bound_as_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("frame.wj", FRAME);
    test.add_file("paint.wj", PAINT);
    let map = test.compile().expect("P3.348 compile");
    let combined = format!(
        "{}\n{}",
        map.get("frame.rs").expect("frame.rs"),
        map.get("paint.rs").expect("paint.rs")
    );
    if bad_i64_bound(&combined) {
        eprintln!("P3.348 RED:\n{combined}");
    }
    assert!(
        !bad_i64_bound(&combined),
        "P3.348: u32 while counters must not cast bounds to i64:\n{combined}"
    );
    test.cargo_check().expect("P3.348 cargo-check");
}
