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

//! P3.532: product `wj-auth-api` leftover rustc after cookie/hash/jwt path
//! deps exist. Isolates false-green. Do not reshape the hexagonal app.
//!
//! 1. `server_serve(..., |req| dispatch(req, shared))` emits
//!    `move |req| dispatch(&mut req, &shared)` + `dispatch(req: &mut ServerRequest)`
//!    (E0596). Notes-api and small serve isolates emit owned `dispatch(req)`.
//! 2. `map.get("access_token")` after `parse_cookie_header` emits
//!    `get("access_token".to_string())` (E0308). Same-crate HashMap isolates
//!    stay borrowed.
//! 3. `resolve_token(authorization, cookie)` into `authorization: &str`
//!    (E0308). Same-crate token isolates demote the caller too.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn auth_api_app() -> Option<PathBuf> {
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
fn auth_api_product_dispatch_must_not_mut_req() {
    let app = auth_api_app().unwrap_or_else(|| {
        panic!(
            "wj-auth-api src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .current_dir(&app)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-auth-api");
    assert!(
        build.status.success(),
        "wj-auth-api product transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let http_rs = fs::read_to_string(out.join("adapters").join("http_server.rs")).unwrap_or_default();
    let auth_rs = fs::read_to_string(out.join("domain").join("auth.rs")).unwrap_or_default();
    eprintln!(
        "P3.532 dispatch: {}",
        http_rs
            .lines()
            .find(|l| l.contains("server_serve"))
            .unwrap_or("")
    );

    assert!(
        !http_rs.contains("dispatch(&mut req"),
        "P3.532 RED: serve closure must not pass &mut req (E0596):\n{http_rs}"
    );
    assert!(
        !auth_rs.contains("get(\"access_token\".to_string())"),
        "P3.532 RED: HashMap.get lit must stay borrowed after parse_cookie_header:\n{auth_rs}"
    );
    let resolve_call = auth_rs
        .lines()
        .find(|l| l.contains("resolve_token(") && !l.contains("fn resolve_token"))
        .unwrap_or("")
        .to_string();
    let resolve_sig = auth_rs
        .lines()
        .find(|l| l.contains("fn resolve_token("))
        .unwrap_or("")
        .to_string();
    assert!(
        !(resolve_sig.contains("authorization: &str")
            && resolve_call.contains("resolve_token(authorization,")
            && !resolve_call.contains("resolve_token(&authorization")),
        "P3.532 RED: owned authorization into demoted &str must borrow:\n{resolve_sig}\n{resolve_call}"
    );
}
