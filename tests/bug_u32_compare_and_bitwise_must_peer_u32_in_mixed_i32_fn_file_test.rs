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

//! P3.367: u32 fields / params must peer-drive literals in void+i32-coord files
//! (`render_width > 0`, `x << 16`, `count & 65535` — not `_i32`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod gpu
"#;

const GPU: &str = r#"
pub struct PassState {
    pub render_width: u32,
    pub frame_count: u32,
    pub ssao_sample_count: u32,
}

pub fn collides_stub(scale: i32) -> bool {
    scale == 0
}

impl PassState {
    pub fn update(self) {
        if self.render_width > 0 {
            let rw = self.render_width
        }
        let packed = (self.frame_count << 16) | (self.ssao_sample_count & 65535)
        let _ = packed
    }
}
"#;

fn bad_u32_i32_literal_peers(rs: &str) -> bool {
    rs.contains("> 0_i32")
        || rs.contains("<< 16_i32")
        || rs.contains("& 65535_i32")
        || rs.contains("| 65535_i32")
}

#[test]
fn u32_compare_and_bitwise_must_peer_u32_in_mixed_i32_fn_file() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("gpu.wj", GPU);
    let map = test.compile().expect("P3.367 compile");
    let rs = map.get("gpu.rs").expect("gpu.rs");
    if bad_u32_i32_literal_peers(rs) {
        eprintln!("P3.367 RED:\n{rs}");
    }
    assert!(
        !bad_u32_i32_literal_peers(rs),
        "P3.367: u32 compare/bitwise must not use _i32 literal peers:\n{rs}"
    );
    test.cargo_check().expect("P3.367 cargo-check");
}
