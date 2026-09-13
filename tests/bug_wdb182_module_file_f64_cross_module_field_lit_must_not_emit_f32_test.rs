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

//! WDB-182: cross-module `f64` struct field literal `0.0` must not emit `0.0_f32`.
//!
//! Product residual (~15× f64←f32), tip-out `cross_signal_fusion_port`:
//!   `FusedMemoryScoreInput { …, graph_score: 0.0_f32 }` while field is `f64`
//! → E0308. Same class as WDB-130/150, but product still RED after those fixtures
//! go tip-GREEN (cross-crate / multipass field init into external struct).
//!
//! Expected: `0.0_f64` (or unsuffixed lit inferred as f64 from field type).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
    // Product: graph_score: 0.0 into f64 field — must not emit 0.0_f32
    let input = FusedScoreInput {
        vector_score: vec_score,
        keyword_score: kw_score,
        graph_score: 0.0,
    }
    fused_score(input)
}
"#;

fn wdb182_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("fusion.wj", FUSION);
    test.add_file("hybrid.wj", HYBRID);
    test
}

#[test]
fn wdb182_module_file_f64_cross_module_field_lit_must_not_emit_f32() {
    let test = wdb182_fixture();
    let map = test
        .compile()
        .expect("WDB-182 multipass compile should succeed");
    let hybrid_rs = map.get("hybrid.rs").expect("hybrid.rs");

    eprintln!("WDB-182 hybrid.rs:\n{hybrid_rs}");

    let bad = hybrid_rs.contains("graph_score: 0.0_f32")
        || (hybrid_rs.contains("graph_score:") && hybrid_rs.contains("0.0_f32"));
    let good = hybrid_rs.contains("graph_score: 0.0_f64")
        || (hybrid_rs.contains("graph_score: 0.0") && !hybrid_rs.contains("_f32"));

    if bad {
        panic!(
            "WDB-182 RED: f64 cross-module field lit emitted _f32. \
             Product: FusedMemoryScoreInput.graph_score. Got:\n{hybrid_rs}"
        );
    }

    assert!(
        good || !bad,
        "WDB-182: graph_score f64 lit must not be f32. Got:\n{hybrid_rs}"
    );
}

#[test]
fn wdb182_tip_out_cross_signal_must_not_emit_f32_graph_score() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let fusion = if tip.join("cross_signal_fusion_port.rs").exists() {
        tip.join("cross_signal_fusion_port.rs")
    } else {
        gen.join("observability/cross_signal_fusion_port.rs")
    };
    let input_ty = gen.join("semantic/agent_memory_fusion_port.rs");
    if !fusion.exists() {
        eprintln!("WDB-182: skip tip-out — fusion port missing");
        return;
    }
    let fusion_text = std::fs::read_to_string(&fusion).expect("fusion");
    let field_f64 = if input_ty.exists() {
        let t = std::fs::read_to_string(&input_ty).expect("input");
        t.contains("pub graph_score: f64")
    } else {
        true
    };
    let bad = field_f64
        && fusion_text.contains("graph_score: 0.0_f32")
        && !fusion_text.contains("graph_score: 0.0_f64");
    eprintln!(
        "WDB-182 tip-out field_f64={} bad={} path={}",
        field_f64,
        bad,
        fusion.display()
    );
    assert!(
        !bad,
        "WDB-182 RED: tip-out/product still emits graph_score: 0.0_f32 into f64 field. {}",
        fusion.display()
    );
}
