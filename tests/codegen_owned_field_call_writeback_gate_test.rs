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
))]

//! Gate — owned `self.field` passed into a helper that returns a writeback field.
//!
//! Pattern (wdb-sim `poll` / `pop_ready`):
//! ```text
//! let popped = pop_ready(self.queue, now)
//! self.queue = popped.remaining
//! ```
//!
//! Behind `&mut self`, moving `self.queue` into an owned formal must use
//! `std::mem::take(&mut self.queue)` — not `.clone()` — when the field is
//! written back from the call result. Bare `self.queue` is only valid for
//! owned `self`.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

struct GateResult {
    ok: bool,
    generated: String,
    wj_stderr: String,
    cargo_stderr: String,
}

fn read_emitted_rs(out_dir: &Path, stem: &str) -> String {
    fs::read_to_string(out_dir.join(format!("{stem}.rs"))).unwrap_or_else(|_| {
        fs::read_dir(out_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    })
}

fn cargo_check_rs(crate_name: &str, rs_body: &str, tmp_root: &Path) -> (bool, String) {
    let crate_dir = tmp_root.join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{crate_name}"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "{crate_name}"
path = "src/main.rs"
"#
        ),
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!("#![allow(dead_code, unused_imports, unused_variables, unused_mut)]\n{rs_body}"),
    )
    .unwrap();

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", tmp_root.join("target"))
        .output()
        .expect("cargo check");
    (
        check.status.success(),
        String::from_utf8_lossy(&check.stderr).to_string(),
    )
}

fn build_and_check(source: &str, stem: &str) -> GateResult {
    let tmp = TempDir::new().unwrap();
    let src_path = tmp.path().join(format!("{stem}.wj"));
    fs::write(&src_path, source).unwrap();
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .args([
            "build",
            src_path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    let wj_stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return GateResult {
            ok: false,
            generated: String::new(),
            wj_stderr,
            cargo_stderr: String::new(),
        };
    }
    let generated = read_emitted_rs(&out, stem);
    let (ok, cargo_stderr) = cargo_check_rs(stem, &generated, tmp.path());
    GateResult {
        ok,
        generated,
        wj_stderr,
        cargo_stderr,
    }
}

const POLL_POP_READY_SOURCE: &str = r#"
struct QueuedMsg {
    ready_at: int,
    payload: int,
}

struct PopResult {
    ready: Option<int>,
    remaining: Vec<QueuedMsg>,
}

fn pop_ready(queue: Vec<QueuedMsg>, now: int) -> PopResult {
    let mut remaining: Vec<QueuedMsg> = Vec::new()
    let mut ready: Option<int> = None
    for msg in queue {
        if ready.is_none() && msg.ready_at <= now {
            ready = Some(msg.payload)
        } else {
            remaining.push(msg)
        }
    }
    PopResult { ready: ready, remaining: remaining }
}

struct SimNetwork {
    queue: Vec<QueuedMsg>,
}

impl SimNetwork {
    fn poll(self, now: int) -> Option<int> {
        let popped = pop_ready(self.queue, now)
        self.queue = popped.remaining
        popped.ready
    }
}

fn main() {
    let mut net = SimNetwork { queue: Vec::new() }
    let _ = net.poll(0)
}
"#;

/// Call-arg + writeback must not clone `self.queue`; prefer `mem::take` behind `&mut self`.
#[test]
fn owned_field_call_writeback_must_mem_take_not_clone() {
    let result = build_and_check(POLL_POP_READY_SOURCE, "poll_writeback");
    assert!(
        result.wj_stderr.is_empty() || result.generated.len() > 0,
        "wj build failed:\n{}",
        result.wj_stderr
    );
    let rust = &result.generated;
    assert!(
        !rust.contains("self.queue.clone()"),
        "must NOT clone self.queue for call-arg writeback:\n{rust}"
    );
    let uses_take = rust.contains("std::mem::take(&mut self.queue)");
    let bare_owned = rust.contains("pop_ready(self.queue,")
        && !rust.contains("&mut self")
        && !rust.contains("& self");
    assert!(
        uses_take || bare_owned,
        "must emit mem::take(&mut self.queue) behind &mut self, or bare self.queue if owned self:\n{rust}"
    );
    assert!(
        result.ok,
        "emitted Rust must cargo check. cargo_stderr=\n{}\ngenerated=\n{rust}",
        result.cargo_stderr
    );
}

/// Let-extract writeback (existing path) must keep working: `let q = self.queue; …; self.queue = q`.
#[test]
fn owned_field_let_extract_writeback_still_mem_take() {
    let source = r#"
struct SimNetwork {
    queue: Vec<int>,
}

impl SimNetwork {
    fn push_then_restore(self, v: int) {
        let mut q = self.queue
        q.push(v)
        self.queue = q
    }
}

fn main() {
    let mut net = SimNetwork { queue: Vec::new() }
    net.push_then_restore(1)
}
"#;
    let result = build_and_check(source, "let_extract_writeback");
    let rust = &result.generated;
    assert!(
        !rust.contains("self.queue.clone()"),
        "let-extract writeback must not clone:\n{rust}"
    );
    let uses_take = rust.contains("std::mem::take(&mut self.queue)");
    let bare_owned = rust.contains("let mut q = self.queue")
        && !rust.contains("&mut self")
        && !rust.contains("& self");
    assert!(
        uses_take || bare_owned,
        "let-extract should mem::take behind &mut self:\n{rust}"
    );
    assert!(
        result.ok,
        "let-extract writeback must cargo check. stderr=\n{}\ngenerated=\n{rust}",
        result.cargo_stderr
    );
}
