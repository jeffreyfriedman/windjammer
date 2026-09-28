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

//! P3.520 isolate now emits `check_rate(&mut self)` (GREEN). Product
//! `handle_method` / `handle_http` stay owned `self`, so rustc E0596
//! (borrow `self` as mut) and E0507 (move out of `MutexGuard<NotesApp>`).
//!
//! Do not reshape notes-api. Isolates that only contain check_rate+dispatch
//! false-green `handle_method(&mut self)` or `self.clone().check_rate`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn notes_api_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-notes-api");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-notes-api");
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

fn owned_self_receiver(sig: &str) -> bool {
    let t = sig.trim();
    (t.contains("(self,") || t.contains("(self ") || t.contains("(mut self"))
        && !t.contains("&mut self")
        && !t.contains("& self")
}

#[test]
fn handle_method_must_mut_self_for_check_rate() {
    let app = notes_api_app().unwrap_or_else(|| {
        panic!(
            "notes-api src not found by walking from {}",
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
        .expect("wj build notes-api");
    assert!(
        build.status.success(),
        "notes-api product transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let api_rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_default();
    let handle_method = api_rs
        .lines()
        .find(|l| l.contains("fn handle_method("))
        .unwrap_or("")
        .to_string();
    let handle_http = api_rs
        .lines()
        .find(|l| l.contains("fn handle_http("))
        .unwrap_or("")
        .to_string();
    eprintln!("handle_method: {handle_method}\nhandle_http: {handle_http}");

    assert!(
        !owned_self_receiver(&handle_method),
        "handle_method must be &mut self after P3.520 check_rate(&mut self):\n{handle_method}\n{api_rs}"
    );
    assert!(
        !owned_self_receiver(&handle_http),
        "handle_http must be &mut self so MutexGuard can call it:\n{handle_http}\n{api_rs}"
    );
}
