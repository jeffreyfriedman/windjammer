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

//! P3.621: owned `String` return into demoted `&str` method formal must auto-borrow.
//!
//! Product `wj-webhook` adapter:
//! ```ignore
//! let reply = app.handle(method_label(req.method), path, …)
//! // handle(self, method: string, …) demotes to method: &str
//! // method_label(…) -> String
//! ```
//! Tip passes bare `String` into `&str` → E0308. Do not reshape adapter with
//! `.as_str()` / change handle API — compiler must borrow at the call site.

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
fn owned_string_return_into_demoted_str_method_formal_must_auto_borrow() {
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
        "P3.621 wj-webhook transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let adapter =
        fs::read_to_string(out.join("adapters").join("http_server.rs")).unwrap_or_default();
    let webhook = fs::read_to_string(out.join("domain").join("webhook.rs")).unwrap_or_default();
    let handle_sig = webhook
        .lines()
        .find(|l| l.contains("fn handle("))
        .unwrap_or("")
        .to_string();
    let call = adapter
        .lines()
        .find(|l| l.contains(".handle("))
        .unwrap_or("")
        .to_string();
    eprintln!("P3.621 handle sig: {handle_sig}");
    eprintln!("P3.621 handle call: {call}");

    let formal_is_str = handle_sig.contains("method: &str") || handle_sig.contains("method: & str");
    // Bare `handle(method_label(...))` into `&str` is the bug. Tip must emit
    // `handle(&method_label(...), …)` (or keep `method: String` on the formal).
    let bad_owned_into_str = formal_is_str
        && call.contains("handle(method_label(")
        && !call.contains("handle(&method_label(");

    assert!(
        !bad_owned_into_str,
        "P3.621 RED: demoted `&str` method formal must auto-borrow owned String \
         from method_label (or keep owned formal):\n  sig: {handle_sig}\n  call: {call}\n{adapter}"
    );
}
