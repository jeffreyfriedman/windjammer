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

//! P3.323: Inferred i32 loop counters (no `: i32`) + i32 sentinel vs i32 field compare.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod camera
pub mod audio
"#;

const CAMERA: &str = r#"
pub fn collides_pivot(scale: i32) -> bool {
    let sf = scale as f32
    let mut dy = 0
    while dy < 3 {
        dy = dy + 1
    }
    let mut i = 0
    while i < 4 {
        i = i + 1
    }
    dy < 3 && i < 4 && sf > 0.0
}
"#;

const AUDIO: &str = r#"
pub struct Zone {
    pub priority: i32,
    pub active: bool,
}

pub fn pick_best(zones: Vec<Zone>) -> i32 {
    let mut best_priority = -1
    let mut best_idx = -1
    let mut i = 0i32
    while (i as usize) < zones.len() {
        let idx = i as usize
        if zones[idx].active && zones[idx].priority > best_priority {
            best_priority = zones[idx].priority
            best_idx = i
        }
        i = i + 1
    }
    best_idx
}
"#;

fn bad_emit(rs: &str) -> bool {
    rs.contains("while dy < 3_i64")
        || rs.contains("while dy < (3_i64")
        || rs.contains("while i < 4_i64")
        || rs.contains("priority as i64) > best_priority")
}

#[test]
fn i32_inferred_loop_counter_and_sentinel_priority_must_stay_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("camera.wj", CAMERA);
    test.add_file("audio.wj", AUDIO);
    let map = test.compile().expect("P3.323 compile");
    let camera_rs = map.get("camera.rs").expect("camera.rs");
    let audio_rs = map.get("audio.rs").expect("audio.rs");
    let combined = format!("{camera_rs}\n{audio_rs}");
    if bad_emit(&combined) {
        eprintln!("P3.323 RED:\n{combined}");
    }
    assert!(
        !bad_emit(&combined),
        "P3.323: inferred i32 loops and i32 priority sentinel must not widen to i64:\n{combined}"
    );
    test.cargo_check().expect("P3.323 cargo-check");
}
