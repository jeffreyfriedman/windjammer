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

//! P3.616: product `wj-auth-api` — reused owned `path: string` must not emit
//! `path.into().clone()` into a later owned-string call (E0282).
//!
//! Tip after Into-paint on sibling formals:
//!   `emit_access_log(…, path.into().clone(), …)` / `origin.into().clone()`
//! Distinct from P3.615 (proxy `client_key.into()` move before second use) —
//! here `.into().clone()` fails inference even when a clone is attempted.
//! Do not reshape auth to drop path reuse.

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
fn reused_owned_string_must_not_emit_into_clone() {
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
        "P3.616 wj-auth-api transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let auth_rs = fs::read_to_string(out.join("domain").join("auth.rs")).unwrap_or_default();
    eprintln!(
        "P3.616 into().clone hits:\n{}",
        auth_rs
            .lines()
            .filter(|l| l.contains(".into().clone()"))
            .take(12)
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        !auth_rs.contains("path.into().clone()")
            && !auth_rs.contains("origin.into().clone()")
            && !auth_rs.contains("accept_encoding.into().clone()"),
        "P3.616 RED: reused owned strings must not emit .into().clone() (wj-auth-api):\n{}",
        auth_rs
            .lines()
            .filter(|l| l.contains(".into().clone()") || l.contains("emit_access_log"))
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
    );
}
