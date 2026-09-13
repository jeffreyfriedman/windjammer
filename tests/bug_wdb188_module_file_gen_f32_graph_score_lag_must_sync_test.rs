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

//! WDB-188: product **gen** `graph_score: 0.0` must not emit `0.0_f32` (tip-out lag).
//!
//! WDB-182 tip-out is GREEN (`0.0_f64`) after tip float fix, but gitignored gen still
//! emits `graph_score: 0.0_f32` → ~15× f64←f32 census residual. Prefer tip-out → gen
//! sync; this gate blocks declaring f32 closed while gen lags.

use std::path::PathBuf;

#[test]
fn wdb188_product_gen_cross_signal_must_not_emit_f32_graph_score() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let fusion = gen.join("observability/cross_signal_fusion_port.rs");
    let input_ty = gen.join("semantic/agent_memory_fusion_port.rs");
    if !fusion.exists() {
        eprintln!("WDB-188: skip — gen fusion missing");
        return;
    }
    let fusion_text = std::fs::read_to_string(&fusion).expect("fusion");
    let field_f64 = if input_ty.exists() {
        std::fs::read_to_string(&input_ty)
            .unwrap_or_default()
            .contains("pub graph_score: f64")
    } else {
        true
    };
    let bad = field_f64
        && fusion_text.contains("graph_score: 0.0_f32")
        && !fusion_text.contains("graph_score: 0.0_f64");
    eprintln!(
        "WDB-188 product gen field_f64={} bad={} path={}",
        field_f64,
        bad,
        fusion.display()
    );
    assert!(
        !bad,
        "WDB-188 RED: product gen still emits graph_score: 0.0_f32 (tip-out already f64). {}",
        fusion.display()
    );
}
