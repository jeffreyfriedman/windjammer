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

//! WDB-198: tip-out/gen search_host `distance: 0.1` must not emit `0.1_f32`.
//!
//! Product residual (~14× f64←f32). WDB-182/188 greened fusion `graph_score`,
//! but `cross_signal_search_host_port` still emits `distance: 0.1_f32` /
//! `0.5_f32` into `VectorTopKHit.distance: f64` (tip-out unchanged).
//! Same class as WDB-150/182 for search_host path.

use std::path::PathBuf;

#[test]
fn wdb198_tip_out_search_host_must_not_emit_f32_distance() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let host = if tip.join("cross_signal_search_host_port.rs").exists() {
        tip.join("cross_signal_search_host_port.rs")
    } else {
        gen.join("observability/cross_signal_search_host_port.rs")
    };
    if !host.exists() {
        eprintln!("WDB-198: skip — search_host missing");
        return;
    }
    let text = std::fs::read_to_string(&host).expect("host");
    let bad = text.contains("distance: 0.1_f32")
        || text.contains("distance: 0.5_f32")
        || (text.contains("distance:") && text.contains("_f32") && text.contains("VectorTopKHit"));
    eprintln!("WDB-198 tip-out bad={} path={}", bad, host.display());
    assert!(
        !bad,
        "WDB-198 RED: tip-out/product search_host still emits distance _f32 into f64 field. {}",
        host.display()
    );
}
