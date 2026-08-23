//! WDB-102 / Phase 151 dogfood: keyword `bound` cannot be used as an identifier.
//!
//! Windjammer lexes `bound` as `Token::Bound` (bound-alias feature). That means
//! ordinary bindings like `Ok(bound) =>` or `fn f(bound: T)` fail to parse with
//! "Expected pattern, got Bound" / "Expected parameter name".
//!
//! Desired (soft keyword): `bound` is reserved only in `bound alias` positions;
//! elsewhere it should be a normal identifier so SQL binder code can say
//! `Ok(bound) =>` naturally.
//!
//! DO NOT fix in this PR from the WindjammerDB agent — another agent owns Windjammer.
//! This test documents the gap for that agent.

use std::process::Command;
use std::fs;
use std::path::PathBuf;

fn write_temp_wj(src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wj_bound_ident_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("bound_ident.wj");
    fs::write(&path, src).expect("write wj");
    path
}

fn wj_bin() -> PathBuf {
    // Prefer local release wj; fall back to PATH.
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    if local.exists() {
        return local;
    }
    PathBuf::from("wj")
}

/// Soft-keyword gap: pattern binding named `bound` should parse as an identifier.
#[test]
fn bound_pattern_binding_should_parse_as_identifier() {
    let path = write_temp_wj(
        r#"
pub fn pick(x: Option<i64>) -> i64 {
    match x {
        Some(bound) => bound,
        None => 0,
    }
}
"#,
    );
    let out_dir = path.parent().unwrap().join("out");
    let status = Command::new(wj_bin())
        .args([
            "build",
            path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
            "--library",
            "--no-generate-cargo-toml",
        ])
        .status()
        .expect("spawn wj");
    assert!(
        status.success(),
        "keyword `bound` should be soft outside bound-alias syntax so `Some(bound) =>` parses (WDB-102)"
    );
}

/// Soft-keyword gap: function parameter named `bound` should parse.
#[test]
fn bound_parameter_name_should_parse_as_identifier() {
    let path = write_temp_wj(
        r#"
pub fn use_bound(bound: i64) -> i64 {
    bound
}
"#,
    );
    let out_dir = path.parent().unwrap().join("out");
    let status = Command::new(wj_bin())
        .args([
            "build",
            path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
            "--library",
            "--no-generate-cargo-toml",
        ])
        .status()
        .expect("spawn wj");
    assert!(
        status.success(),
        "keyword `bound` should be soft so `fn use_bound(bound: i64)` parses (WDB-102)"
    );
}
