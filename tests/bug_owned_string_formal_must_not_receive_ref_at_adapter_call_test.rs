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

//! P3.638: owned `String` method formal must not receive `&String` at adapter call.
//!
//! Product `wj-auth-api` adapter:
//! ```ignore
//! app.handle_http(…, meta.0, meta.1, meta.2, …)  // authorization: string (owned)
//! ```
//! Tip emits `handle_http(…, &meta.2, …)` → E0308 expected `String`, found `&String`.
//! Sibling formals `meta.0`/`meta.1` move correctly; only authorization over-borrows.
//! Do not reshape adapter with `.clone()` / change formal ownership.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn auth_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-auth-api");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-auth-api");
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
fn owned_string_formal_must_not_receive_ref_at_adapter_call() {
    let app = auth_app().unwrap_or_else(|| {
        panic!(
            "wj-auth-api src not found by walking from {}",
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
        .expect("wj build wj-auth-api");
    assert!(
        build.status.success(),
        "P3.638 wj-auth-api transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let adapter =
        fs::read_to_string(out.join("adapters").join("http_server.rs")).unwrap_or_default();
    let auth = fs::read_to_string(out.join("domain").join("auth.rs")).unwrap_or_default();
    let handle_sig = auth
        .lines()
        .find(|l| l.contains("fn handle_http("))
        .unwrap_or("")
        .to_string();
    let call = adapter
        .lines()
        .find(|l| l.contains("handle_http("))
        .unwrap_or("")
        .to_string();
    eprintln!("P3.638 handle_http: {handle_sig}");
    eprintln!("P3.638 call: {call}");

    let authz_owned = handle_sig.contains("authorization: String")
        || handle_sig.contains("authorization: impl Into<String>");
    let bad = authz_owned && call.contains("&meta.2");

    assert!(
        !bad,
        "P3.638 RED: owned authorization formal must receive moved meta.2, not &meta.2:\n  {call}\n{adapter}"
    );
}
