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

//! Regression: module file f64 struct field lit must not emit f32.
//!
//! Expected: `0.1_f64` / `2.0_f64` (or un-suffixed lits that rustc infers as f64).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod hits
pub mod host
"#;

const HITS: &str = r#"
pub struct VectorTopKHit {
    pub node_id: string
    pub distance: f64
}

pub struct Bm25TopKHit {
    pub document_id: string
    pub score: f64
}
"#;

const HOST: &str = r#"
use crate::hits::VectorTopKHit
use crate::hits::Bm25TopKHit

pub fn host_cap_hits() -> (Vec<VectorTopKHit>, Vec<Bm25TopKHit>) {
    let mut vector: Vec<VectorTopKHit> = Vec::new()
    vector.push(VectorTopKHit { node_id: "doc_a", distance: 0.1 })
    vector.push(VectorTopKHit { node_id: "doc_b", distance: 0.5 })
    let mut bm25: Vec<Bm25TopKHit> = Vec::new()
    bm25.push(Bm25TopKHit { document_id: "doc_a", score: 2.0 })
    bm25.push(Bm25TopKHit { document_id: "doc_b", score: 1.0 })
    (vector, bm25)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("hits.wj", HITS);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn module_file_f64_struct_field_lit_must_not_emit_f32() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let host_rs = map.get("host.rs").expect("host.rs");

    let bad = host_rs.contains("_f32")
        && (host_rs.contains("distance:") || host_rs.contains("score:"));

    eprintln!("host.rs:\n{host_rs}");
    eprintln!("bad={bad}");

    if bad {
        panic!(
            "RED: f64 struct field literals must not emit _f32. \
            "
        );
    }

    test.cargo_check().expect(
        "f64 field literals must cargo-check without f32 suffixes.",
    );
}
