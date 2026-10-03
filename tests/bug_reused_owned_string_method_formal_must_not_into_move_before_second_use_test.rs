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

//! P3.615: product `wj-proxy` `complete_proxy` — reused `client_key: string`
//! must not emit `impl Into<String>` then `check_rate(client_key.into())`
//! before a later `client_key` use (E0382).
//!
//! Flat isolates false-GREEN (`client_key: String` + `check_rate(&client_key)`).
//! Gate the hexagonal product package. Do not reshape proxy with manual clones.
//! Distinct from P3.599 (private free-fn Into) / P3.598 (builder Into) /
//! P3.614 (borrowed for-in tuple field clone).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn proxy_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-proxy");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-proxy");
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
fn reused_owned_string_method_formal_must_not_into_move_before_second_use() {
    let app = proxy_app().unwrap_or_else(|| {
        panic!(
            "wj-proxy src not found by walking from {}",
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
        .expect("wj build wj-proxy");
    assert!(
        build.status.success(),
        "P3.615 wj-proxy transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let proxy_rs = fs::read_to_string(out.join("domain").join("proxy.rs")).unwrap_or_default();
    let complete = proxy_rs
        .lines()
        .find(|l| l.contains("fn complete_proxy("))
        .unwrap_or("")
        .to_string();
    eprintln!("P3.615 complete_proxy: {complete}");

    assert!(
        !complete.contains("client_key: impl Into<String>"),
        "P3.615 RED: reused client_key formal must not be impl Into<String>:\n{complete}"
    );
    assert!(
        !proxy_rs.contains("check_rate(client_key.into()"),
        "P3.615 RED: must not move client_key via .into() into check_rate before later use:\n{proxy_rs}"
    );
}
