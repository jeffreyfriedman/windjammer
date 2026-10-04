#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! P3.620: cross-crate owned handle reassignment must not pass `&mut` into owned formal.
//!
//! Product `wj-webhook` `queue_via_event_bus`:
//! ```ignore
//! let bus = subscribe(new_bus(), …)
//! let bus = emit(bus, own(name), own(payload))  // emit(bus: EventBus, …) -> EventBus
//! ```
//! Tip emits `let mut bus = emit(&mut bus, …)` → E0308 (expected `EventBus`,
//! found `&mut EventBus`). Same class as P3.619 (`&mut CronExpr` into owned
//! CronExpr) but product-shaped for the event-bus reassignment pattern.
//! Do not reshape webhook with `.clone()` / manual borrow.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn webhook_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-webhook");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-webhook");
            if uncle.join("src").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn cross_crate_owned_bus_formal_must_not_receive_mut_ref() {
    let app = webhook_app().unwrap_or_else(|| {
        panic!(
            "wj-webhook src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&app)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-webhook");
    assert!(
        build.status.success(),
        "P3.620 wj-webhook transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let webhook = fs::read_to_string(out.join("domain").join("webhook.rs")).unwrap_or_default();
    let emit_lines: Vec<&str> = webhook
        .lines()
        .filter(|l| l.contains("emit("))
        .collect();
    eprintln!("P3.620 emit sites:\n{}", emit_lines.join("\n"));

    assert!(
        !webhook.contains("emit(&mut bus"),
        "P3.620 RED: owned EventBus formal must receive moved bus, not `&mut bus`:\n{webhook}"
    );
}
