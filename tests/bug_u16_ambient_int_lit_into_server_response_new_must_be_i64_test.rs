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

//! P3.596: product `wj-proxy` multipass — int lits in `base_response(status: u16)`
//! / `server_error` must emit i64 into `ServerResponse::new(status: i64)`.
//!
//! Flat single-file isolates false-GREEN (`404_i64`). Product hexagonal emit:
//!   `let code = 404_u16` / `let status = 500_u16` → E0308.
//! Do not reshape proxy to drop `u16` status types.

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
fn u16_ambient_int_lit_into_server_response_new_must_be_i64() {
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
        "P3.596 wj-proxy transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let http_rs = fs::read_to_string(out.join("adapters").join("http_server.rs"))
        .unwrap_or_else(|_| fs::read_to_string(out.join("http_server.rs")).unwrap_or_default());
    eprintln!("P3.596 base_response region:\n{}", {
        let start = http_rs.find("fn base_response").unwrap_or(0);
        &http_rs[start..http_rs.len().min(start + 900)]
    });

    assert!(
        !http_rs.contains("let code = 404_u16")
            && !http_rs.contains("let code = 429_u16")
            && !http_rs.contains("let code = 502_u16")
            && !http_rs.contains("let status = 500_u16"),
        "P3.596 RED: product proxy must not paint int lits as u16 into ServerResponse::new i64:\n{http_rs}"
    );
    assert!(
        http_rs.contains("404_i64") || http_rs.contains("let code = 404"),
        "P3.596 RED: expected i64 status lit in base_response:\n{http_rs}"
    );
}
