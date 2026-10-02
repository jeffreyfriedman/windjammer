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

//! P3.570: product `wj-notes-api` tip emits `handle_method(&mut self)` +
//! `check_rate(&mut self)` but `dispatch(mut self)` → E0507 at
//! `self.dispatch(...)`.
//!
//! Product shape: dispatch moves `self.store` into a read helper (`fetch_note`)
//! and also mutates store via create/update — after mut check_rate, dispatch
//! must be `&mut self` (and the read helper must borrow, not take owned store).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::collections::HashMap

struct NoteStore {
    next_id: int,
    label: string,
}

impl NoteStore {
    fn create(self, title: string) -> string {
        self.next_id = self.next_id + 1
        title
    }
}

fn fetch_note(store: NoteStore, id: int) -> string {
    let _ = id
    store.label
}

struct NotesApp {
    store: NoteStore,
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

    fn dispatch(self, path: string, create: bool) -> string {
        if create {
            return self.store.create(path)
        }
        fetch_note(self.store, 1)
    }

    pub fn handle_method(self, path: string, client_key: string, now_ms: int, create: bool) -> string {
        match self.check_rate(client_key, now_ms) {
            Some(limit) => return limit,
            None => {},
        }
        self.dispatch(path, create)
    }
}
"#;

fn dispatch_takes_owned_self(rs: &str) -> bool {
    rs.lines().any(|line| {
        let t = line.trim();
        (t.contains("fn dispatch(mut self") || t.contains("fn dispatch(self,"))
            && !t.contains("&mut self")
            && !t.contains("& self")
    })
}

#[test]
fn notes_api_dispatch_from_mut_handle_must_not_be_owned() {
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
        "P3.570 transpile failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.570 lib.rs:\n{generated}");

    let check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3570_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    assert!(
        !dispatch_takes_owned_self(&generated),
        "P3.570 RED: dispatch must not stay owned when called from &mut handle_method:\n{generated}"
    );
    assert!(
        check.status.success(),
        "P3.570 RED: handle_method + check_rate + dispatch must cargo-check:\n{generated}\n{err}"
    );
}
