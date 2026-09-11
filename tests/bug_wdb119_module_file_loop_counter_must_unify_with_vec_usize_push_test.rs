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

//! WDB-119: `let mut i = 0` used as `Vec<usize>::push(i)` stays `i64` → E0308.
//!
//! WindjammerDB CQ-C5: `document_dremel_index_build` has
//!   `let mut indices: Vec<usize> = Vec::new(); let mut i = 0; while ... { indices.push(i); i = i + 1 }`
//! cargo/tip emit `i` as `i64` while `push` expects `usize` (~1000+ product E0308 of this class).
//!
//! Expected: unify loop counter with `Vec<usize>` element type (or emit `0_usize`).
//! Gate: multipass cargo-check (RED until fixed).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod index
"#;

const INDEX: &str = r#"
pub fn build_indices(n: usize) -> Vec<usize> {
    let mut indices: Vec<usize> = Vec::new()
    let mut i = 0
    while i < n {
        indices.push(i)
        i = i + 1
    }
    indices
}

pub fn cap() -> Vec<usize> {
    build_indices(3)
}
"#;

fn wdb119_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("index.wj", INDEX);
    test
}

#[test]
fn wdb119_module_file_loop_counter_must_unify_with_vec_usize_push() {
    let mut test = wdb119_fixture();
    let map = test
        .compile()
        .expect("WDB-119 multipass compile should succeed (codegen may still be wrong)");
    let index_rs = map.get("index.rs").expect("index.rs must be generated");

    let i_as_i64 = index_rs.contains("let mut i = 0_i64")
        || index_rs.contains("let mut i: i64")
        || (index_rs.contains("0_i64") && index_rs.contains("indices.push(i)"));

    if i_as_i64 {
        eprintln!("WDB-119 RED emit index.rs:\n{index_rs}");
    }

    test.cargo_check().expect(
        "WDB-119 RED: loop counter pushed into Vec<usize> must be usize (not i64). Product: document_dremel_index_port and ~1k E0308 sites.",
    );
}
