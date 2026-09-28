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

//! Product `check_rate` does `self.buckets = buckets_from_limit(...)` after a
//! helper consumes `self.buckets`, then `handle_method` still calls
//! `self.dispatch`. Tip emits `check_rate(mut self)` (owned) so rustc E0382
//! `self` used after move.
//!
//! `wj-notes-api` `$WJ test` on tip p3505 (20:11): query last-use greened;
//! remaining includes this move. Do not reshape the app.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::collections::HashMap

struct NotesApp {
    n: int,
    buckets: HashMap<string, int>,
}

fn buckets_for_limit(buckets: HashMap<string, int>) -> HashMap<string, int> {
    buckets
}

fn buckets_from_limit(buckets: HashMap<string, int>) -> HashMap<string, int> {
    buckets
}

impl NotesApp {
    fn check_rate(self, client_key: string, now_ms: int) -> Option<string> {
        if now_ms < 0 {
            return Some("429")
        }
        let _ = client_key
        let checked = buckets_for_limit(self.buckets)
        self.buckets = buckets_from_limit(checked)
        None
    }

    fn dispatch(self, path: string) -> string {
        path
    }

    pub fn handle_method(self, path: string, client_key: string, now_ms: int) -> string {
        match self.check_rate(client_key, now_ms) {
            Some(limit) => return limit,
            None => {},
        }
        self.dispatch(path)
    }
}
"#;

fn check_rate_takes_owned_self(rs: &str) -> bool {
    rs.lines().any(|line| {
        let t = line.trim();
        (t.contains("fn check_rate(mut self") || t.contains("fn check_rate(self,"))
            && !t.contains("&mut self")
            && !t.contains("& self")
    })
}

#[test]
fn check_rate_field_replace_must_not_move_self() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "wj build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    if !check.status.success() || check_rate_takes_owned_self(&generated) {
        eprintln!("P3.520 RED:\n{generated}\n{err}");
    }
    assert!(
        !check_rate_takes_owned_self(&generated),
        "check_rate must be &mut self when the caller uses self after None:\n{generated}"
    );
    assert!(
        check.status.success(),
        "handle_method after check_rate field replace must cargo-check:\n{generated}\n{err}"
    );
}
