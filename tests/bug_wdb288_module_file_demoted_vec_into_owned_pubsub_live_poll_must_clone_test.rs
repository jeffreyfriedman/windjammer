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
    feature = "codegen_tests",
))]

//! WDB-288: demoted `&backlog` into owned `observability_pubsub_live_poll` must clone.
//!
//! Twin of WDB-241/285. Tip-out still emits:
//!   `observability_pubsub_live_poll(sub, &backlog, live)`
//! while formal is `backlog: Vec<PubSubDeltaEnvelope>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb288_tip_out_pubsub_must_clone_ref_vec_into_owned_live_poll() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("observability_pubsub_live_subscribe_port.rs"),
        gen.join("graph/observability_pubsub_live_subscribe_port.rs"),
        gen.join("relational/observability_pubsub_live_subscribe_port.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("pubsub");
        if text.contains(
            "fn observability_pubsub_live_poll(sub: PubSubLiveSubscription, backlog: Vec<PubSubDeltaEnvelope>"
        ) {
            owned = true;
            break;
        }
    }
    assert!(owned, "WDB-288: owned Vec observability_pubsub_live_poll formal missing");

    let engine_paths = if tip.join("observability_pubsub_live_subscribe_port.rs").exists() {
        vec![tip.join("observability_pubsub_live_subscribe_port.rs")]
    } else {
        vec![
            gen.join("graph/observability_pubsub_live_subscribe_port.rs"),
            gen.join("relational/observability_pubsub_live_subscribe_port.rs"),
        ]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pubsub");
        let bad = text.contains("observability_pubsub_live_poll(") && text.contains(", &backlog,");
        eprintln!("WDB-288 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-288 RED: tip-out/product passes &Vec into owned observability_pubsub_live_poll. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-288: observability_pubsub_live_subscribe_port missing");
}
