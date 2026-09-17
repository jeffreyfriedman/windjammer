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

//! P3.352: i32 loop counters vs `.len()` and small literal while bounds must not emit `as i64`
//! or `_i64` peers (csg emit_instruction, perlin perm init, for i < vec.len()).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod csg
pub mod noise
pub mod scan
"#;

const CSG: &str = r#"
pub fn pad_params(params: Vec<f32>) -> i32 {
    let mut n = 0
    for i in 0..14 {
        if i < params.len() {
            n = n + 1
        }
    }
    n
}

pub fn emit_slots(params: Vec<f32>) -> i32 {
    let mut count = 0
    let mut i = 0
    while i < params.len() {
        if params[i as usize] > 0.0 {
            count = count + 1
        }
        i = i + 1
    }
    count
}
"#;

const NOISE: &str = r#"
pub struct PermTable {
    pub seed: int,
    pub perm: Vec<i32>,
}

impl PermTable {
    pub fn new(seed: int) -> PermTable {
        let mut perm: Vec<i32> = vec![]
        let mut i = 0
        while i < 512 {
            perm.push(0)
            i = i + 1
        }
        PermTable { seed, perm }
    }
}
"#;

const SCAN: &str = r#"
pub fn count_pos(values: Vec<f32>) -> i32 {
    let mut n = 0
    for i in 0..values.len() {
        if values[i as usize] > 0.0 {
            n = n + 1
        }
    }
    n
}
"#;

fn bad_i64_len_or_bound(rs: &str) -> bool {
    rs.contains(".len() as i64")
        || rs.contains("while i < 512_i64")
        || rs.contains("while i < 256_i64")
        || rs.contains("while i < (params.len() as i64)")
        || rs.contains("< (params.len() as i64)")
}

#[test]
fn i32_while_len_and_literal_bound_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("csg.wj", CSG);
    test.add_file("noise.wj", NOISE);
    test.add_file("scan.wj", SCAN);
    let map = test.compile().expect("P3.350 compile");
    let combined = format!(
        "{}\n{}\n{}",
        map.get("csg.rs").expect("csg.rs"),
        map.get("noise.rs").expect("noise.rs"),
        map.get("scan.rs").expect("scan.rs")
    );
    if bad_i64_len_or_bound(&combined) {
        eprintln!("P3.350 RED:\n{combined}");
    }
    assert!(
        !bad_i64_len_or_bound(&combined),
        "P3.350: i32 loops vs len/literal bounds must not emit i64 casts:\n{combined}"
    );
    test.cargo_check().expect("P3.350 cargo-check");
}
