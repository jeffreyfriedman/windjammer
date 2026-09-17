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

//! P3.327: u32 locals / struct u32 fields ± untyped int literals must not emit `_u64` peers.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod editor
pub mod ai
pub mod tile
"#;

const EDITOR: &str = r#"
pub struct HalfEdge {
    pub id: u32,
    pub vertex: u32,
    pub face: i32,
    pub next: u32,
    pub prev: u32,
    pub twin: i32,
}

pub fn build_triangle(base: u32, i0: u32, i1: u32, i2: u32, f: i32) -> HalfEdge {
    HalfEdge {
        id: base,
        vertex: i1,
        face: f,
        next: base + 1,
        prev: base + 2,
        twin: 0,
    }
}

pub fn loop_edges(tri_count: u32) -> u32 {
    let mut last = 0u32
    let mut f = 0u32
    while f < tri_count {
        let base = (3 * f) as u32
        let he0 = HalfEdge {
            id: base,
            vertex: 1,
            face: f as i32,
            next: base + 1,
            prev: base + 2,
            twin: 0,
        }
        last = he0.next
        f = f + 1
    }
    last
}

pub fn bump_index(i0: u32) -> u32 {
    i0 + 1
}
"#;

const TILE: &str = r#"
pub fn has_tile(tile_id: u32) -> bool {
    tile_id != 0
}
"#;

const AI: &str = r#"
pub fn jitter(r: u32) -> f32 {
    ((r % 1000) as f32 / 500.0) - 1.0
}
"#;

fn bad_u32_arith_u64_literal(rs: &str) -> bool {
    rs.contains("+ 1_u64")
        || rs.contains("+ 2_u64")
        || rs.contains("3_u64 *")
        || rs.contains("+= 1_u64")
        || rs.contains("% 1000_u64")
        || rs.contains("!= 0_u64")
}

#[test]
fn u32_arith_int_literal_must_not_emit_u64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("editor.wj", EDITOR);
    test.add_file("ai.wj", AI);
    test.add_file("tile.wj", TILE);
    let map = test.compile().expect("P3.327 compile");
    let editor_rs = map.get("editor.rs").expect("editor.rs");
    let ai_rs = map.get("ai.rs").expect("ai.rs");
    let tile_rs = map.get("tile.rs").expect("tile.rs");
    let combined = format!("{editor_rs}\n{ai_rs}\n{tile_rs}");
    if bad_u32_arith_u64_literal(&combined) {
        eprintln!("P3.327 RED:\n{combined}");
    }
    assert!(
        !bad_u32_arith_u64_literal(&combined),
        "P3.327: u32 arith/mod with int literals must stay u32 width:\n{combined}"
    );
    test.cargo_check().expect("P3.327 cargo-check");
}
