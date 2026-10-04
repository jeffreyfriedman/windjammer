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

//! P3.636: `&mut self` field into owned `json::to_string` formal must clone (or borrow).
//!
//! Product `wj-webhook` `list_events` (source `fn list_events(self)`):
//! ```ignore
//! match json.to_string(self.events) { … }
//! ```
//! Tip demotes to `&mut self` then emits `json::to_string(self.events)` → E0507.
//! P3.620/621 greened on tip; this is the remaining webhook blocker.
//! Do not reshape webhook with manual `.clone()`.

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
fn mut_self_field_into_owned_json_formal_must_clone() {
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
        "P3.636 wj-webhook transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let webhook = fs::read_to_string(out.join("domain").join("webhook.rs")).unwrap_or_default();
    let list_fn = {
        let start = webhook.find("fn list_events").unwrap_or(0);
        &webhook[start..webhook.len().min(start + 500)]
    };
    eprintln!("P3.636 list_events region:\n{list_fn}");

    let bad_move = list_fn.contains("json::to_string(self.events)")
        || (list_fn.contains("to_string(self.events)") && !list_fn.contains("self.events.clone()"));
    let ok = list_fn.contains("self.events.clone()") || list_fn.contains("to_string(&self.events)");

    assert!(
        ok || !bad_move,
        "P3.636 RED: &mut self.events into owned json::to_string must clone or borrow:\n{list_fn}"
    );
}
