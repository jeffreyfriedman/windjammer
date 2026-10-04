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

//! P3.636: owned `String` locals into demoted `&str` method formals must auto-borrow.
//!
//! Product `wj-auth-api`:
//! ```ignore
//! if self.find_user(username) { … }           // find_user(…, username: &str)
//! match self.verify_user(username, password)  // both &str
//! ```
//! Tip emits bare `String` → E0308. Distinct from greened P3.621 (call-expr
//! `method_label(…)` into `&str`); this is **owned locals** into sibling methods.
//! Do not reshape auth with `.as_str()`.

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
fn owned_string_into_demoted_str_method_formal_must_auto_borrow() {
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
        "P3.636 wj-auth-api transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let auth = fs::read_to_string(out.join("domain").join("auth.rs")).unwrap_or_default();
    let find_sig = auth
        .lines()
        .find(|l| l.contains("fn find_user("))
        .unwrap_or("")
        .to_string();
    let verify_sig = auth
        .lines()
        .find(|l| l.contains("fn verify_user("))
        .unwrap_or("")
        .to_string();
    let find_call = auth
        .lines()
        .find(|l| l.contains("self.find_user(username)") || l.contains("self.find_user(&username)"))
        .unwrap_or("")
        .to_string();
    let verify_call = auth
        .lines()
        .find(|l| l.contains("self.verify_user("))
        .unwrap_or("")
        .to_string();
    eprintln!("P3.636 find_user: {find_sig}");
    eprintln!("P3.636 find call: {find_call}");
    eprintln!("P3.636 verify_user: {verify_sig}");
    eprintln!("P3.636 verify call: {verify_call}");

    let find_wants_str = find_sig.contains("username: &str");
    let verify_wants_str = verify_sig.contains("username: &str");
    let bad_find = find_wants_str && find_call.contains("find_user(username)") && !find_call.contains("find_user(&username)");
    let bad_verify = verify_wants_str
        && verify_call.contains("verify_user(username, password)")
        && !verify_call.contains("verify_user(&username");

    assert!(
        !bad_find && !bad_verify,
        "P3.636 RED: owned String locals into demoted &str method formals must auto-borrow:\n  {find_call}\n  {verify_call}"
    );
}
