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

//! P3.338: i32 loop counters (`while`/`for`) must not peer `+ 1` as `_i64` or compare via `len() as i64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod track
pub mod scene
pub mod debug_draw
pub mod viewer
"#;

const TRACK: &str = r#"
pub struct Keyframe {
    time: f32,
    data: string,
}

impl Keyframe {
    pub fn new(time: f32, data: string) -> Keyframe {
        Keyframe { time, data }
    }
    pub fn time(self) -> f32 {
        self.time
    }
}

pub struct Track {
    pub keyframes: Vec<Keyframe>,
}

impl Track {
    pub fn sort_keyframes(self) {
        let len = self.keyframes.len() as i32
        let mut i = 0
        while i < len - 1 {
            let mut j = 0
            while j < len - 1 - i {
                let t1 = self.keyframes[j as usize].time()
                let t2 = self.keyframes[(j + 1) as usize].time()
                if t1 > t2 {
                    self.keyframes.swap(j as usize, (j + 1) as usize)
                }
                j = j + 1
            }
            i = i + 1
        }
    }
}
"#;

const SCENE: &str = r#"
pub fn count_under(params: Vec<f32>) -> i32 {
    let mut n = 0
    for i in 0..14 {
        if i < params.len() {
            n = n + 1
        }
    }
    n
}

pub fn scan_params(values: Vec<f32>) -> i32 {
    let mut n = 0
    for i in 0..values.len() {
        if values[i as usize] > 0.0 {
            n = n + 1
        }
    }
    n
}
"#;


const VIEWER: &str = r#"
pub fn edge_check(dy: i32) -> bool {
    if dy == 1 {
        return true
    }
    false
}

pub fn door_left(cx: i32, door_w: i32) -> i32 {
    cx - door_w / 2 - 1
}
"#;

const DEBUG_DRAW: &str = r#"
pub fn circle_steps(segments: i32) -> i32 {
    let seg = if segments < 4 { 4 } else { segments }
    let mut i = 0
    let mut lines = 0
    while i < seg {
        let a1 = ((i + 1) as f32)
        if a1 > 0.0 {
            lines = lines + 1
        }
        i = i + 1
    }
    lines
}
"#;

fn bad_i64_peers(rs: &str) -> bool {
    rs.contains("== 1_i64")
        || rs.contains("/ 2_i64")
        ||
    rs.contains("+ 1_i64")
        || rs.contains("len() as i64")
        || rs.contains(".len() as i64")
        || rs.contains("0_i32..values.len()")
}

#[test]
fn i32_loop_arith_and_len_compare_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("track.wj", TRACK);
    test.add_file("scene.wj", SCENE);
    test.add_file("debug_draw.wj", DEBUG_DRAW);
    test.add_file("viewer.wj", VIEWER);
    let map = test.compile().expect("P3.338 compile");
    let combined = format!(
        "{}\n{}\n{}\n{}",
        map.get("track.rs").expect("track.rs"),
        map.get("scene.rs").expect("scene.rs"),
        map.get("debug_draw.rs").expect("debug_draw.rs"),
        map.get("viewer.rs").expect("viewer.rs")
    );
    if bad_i64_peers(&combined) {
        eprintln!("P3.338 RED:\n{combined}");
    }
    assert!(
        !bad_i64_peers(&combined),
        "P3.338: i32 loops must not emit i64 literal peers or len-as-i64 compares:\n{combined}"
    );
    test.cargo_check().expect("P3.338 cargo-check");
}
