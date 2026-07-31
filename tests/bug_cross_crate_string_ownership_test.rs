#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Cross-crate WJ `string` ownership must match dependency metadata:
//! - Callee emits `&str` (`emitted_rust_ref_params: true`) → caller passes `&field`, not `.clone()`
//!
//! Wire body is read-only (`.len()`) so formals demote to `&str` and metadata publishes
//! shared-ref flags. Owned-concat APIs remain covered by owned-formal call-site tests.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn build_wire_crate(wire_src: &std::path::Path, wire_gen: &std::path::Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wire_src.to_str().unwrap(),
            "--output",
            wire_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wire build");
    assert!(
        out.status.success(),
        "wire library build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        wire_gen.join("metadata.json").exists(),
        "wire must emit metadata.json"
    );
}

fn build_consumer(
    consumer_wj: &std::path::Path,
    consumer_gen: &std::path::Path,
    wire_gen: &std::path::Path,
) -> String {
    let metadata = format!("wire={}", wire_gen.join("metadata.json").display());
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            consumer_wj.to_str().unwrap(),
            "--output",
            consumer_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &metadata,
        ])
        .output()
        .expect("consumer build");
    assert!(
        out.status.success(),
        "consumer build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read_to_string(consumer_gen.with_file_name("consumer.rs"))
        .or_else(|_| fs::read_to_string(consumer_gen.join("consumer.rs")))
        .expect("consumer.rs")
}

#[test]
fn test_cross_crate_borrowed_string_method_no_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let wire_src = tmp.path().join("wire_src");
    let wire_gen = tmp.path().join("wire_gen");
    fs::create_dir_all(&wire_src).unwrap();

    // Read-only string use → `&str` formals + emitted_rust_ref_params in metadata.
    fs::write(
        wire_src.join("client.wj"),
        r##"
pub struct WireClient {
    endpoint: string,
}

impl WireClient {
    pub fn spice_db(endpoint: string) -> WireClient {
        WireClient { endpoint: endpoint }
    }

    pub fn check_live(
        self,
        resource_type: string,
        resource_id: string,
        permission: string,
    ) -> bool {
        resource_type.len() > 0 && resource_id.len() > 0 && permission.len() > 0
            && self.endpoint.len() > 0
    }
}
"##,
    )
    .unwrap();

    build_wire_crate(&wire_src, &wire_gen);

    let meta_text = fs::read_to_string(wire_gen.join("metadata.json")).expect("metadata.json");
    let meta: serde_json::Value =
        serde_json::from_str(&meta_text).expect("metadata.json parses as JSON");
    let check_live = meta
        .pointer("/functions/WireClient::check_live/emitted_rust_ref_params")
        .expect("check_live emitted_rust_ref_params");
    assert_eq!(
        check_live,
        &serde_json::json!([false, true, true, true]),
        "wire metadata must publish shared-ref emission for string formals. Got:\n{meta_text}"
    );

    let consumer_src = tmp.path().join("consumer_src");
    fs::create_dir_all(&consumer_src).unwrap();
    fs::write(
        consumer_src.join("consumer.wj"),
        r##"
use wire::client::WireClient

pub struct ObjectRef {
    object_type: string,
    object_id: string,
}

pub fn check_object(client: WireClient, relation: string, object: ObjectRef) -> bool {
    client.check_live(object.object_type, object.object_id, relation)
}
"##,
    )
    .unwrap();

    let consumer_gen = tmp.path().join("consumer_gen");
    let rs = build_consumer(
        &consumer_src.join("consumer.wj"),
        &consumer_gen,
        &wire_gen,
    );

    for line in rs.lines() {
        if line.contains("check_live(") {
            assert!(
                !line.contains("object_type.clone()"),
                "borrowed string formal must not clone at cross-crate call site.\nLine: {line}\nFull:\n{rs}"
            );
            assert!(
                !line.contains("object_id.clone()"),
                "borrowed string formal must not clone at cross-crate call site.\nLine: {line}\nFull:\n{rs}"
            );
            assert!(
                !line.contains("relation.clone()"),
                "borrowed string formal must not clone at cross-crate call site.\nLine: {line}\nFull:\n{rs}"
            );
            assert!(
                line.contains("&object.object_type") || line.contains("& object.object_type"),
                "expected &object.object_type at shared-ref call site.\nLine: {line}\nFull:\n{rs}"
            );
        }
    }
}
