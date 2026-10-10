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

//! Regression: module file f64 cross module field lit must not emit f32.
//!
//! Expected: `0.0_f64` (or unsuffixed lit inferred as f64 from field type).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod fusion
pub mod hybrid
"#;

const FUSION: &str = r#"
pub struct FusedScoreInput {
    pub vector_score: f64,
    pub keyword_score: f64,
    pub graph_score: f64,
}

pub fn fused_score(input: FusedScoreInput) -> f64 {
    input.vector_score + input.keyword_score + input.graph_score
}
"#;

const HYBRID: &str = r#"
use crate::fusion::FusedScoreInput
use crate::fusion::fused_score

pub fn hybrid_topk_score(vec_score: f64, kw_score: f64) -> f64 {
    // graph_score: 0.0 into f64 field — must not emit 0.0_f32
    let input = FusedScoreInput {
        vector_score: vec_score,
        keyword_score: kw_score,
        graph_score: 0.0,
    }
    fused_score(input)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("fusion.wj", FUSION);
    test.add_file("hybrid.wj", HYBRID);
    test
}

#[test]
fn module_file_f64_cross_module_field_lit_must_not_emit_f32() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let hybrid_rs = map.get("hybrid.rs").expect("hybrid.rs");

    eprintln!("hybrid.rs:\n{hybrid_rs}");

    let bad = hybrid_rs.contains("graph_score: 0.0_f32")
        || (hybrid_rs.contains("graph_score:") && hybrid_rs.contains("0.0_f32"));
    let good = hybrid_rs.contains("graph_score: 0.0_f64")
        || (hybrid_rs.contains("graph_score: 0.0") && !hybrid_rs.contains("_f32"));

    if bad {
        panic!(
            "RED: f64 cross-module field lit emitted _f32. \
             Got:\n{hybrid_rs}"
        );
    }

    assert!(
        good || !bad,
        "graph_score f64 lit must not be f32. Got:\n{hybrid_rs}"
    );
}
