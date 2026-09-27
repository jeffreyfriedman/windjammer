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

//! Empty string literals into owned `string` formals must emit `String`, not `""`.
//!
//! Ecosystem `wj-notes-api` `handle_request`:
//! `app.handle(method, path, "", "", "", 0, body)` — `origin` / `accept_encoding`
//! / `client_key` stay `String` but the call emits `""` (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn owned_string_formals_must_own_empty_literals() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::write(
        &src,
        r#"
fn keep(value: string) -> string {
    value
}

struct App {
    n: int,
}

impl App {
    pub fn handle(
        self,
        origin: string,
        accept_encoding: string,
        client_key: string,
    ) -> string {
        // Consume into owned storage so formals stay String like notes-api handle.
        let origin = keep(origin)
        let accept_encoding = keep(accept_encoding)
        let client_key = keep(client_key)
        "${origin}:${accept_encoding}:${client_key}"
    }
}

pub fn handle_request() -> string {
    let mut app = App { n: 0 }
    app.handle("", "", "")
}
"#,
    )
    .unwrap();

    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("t.rs"))
        .or_else(|_| fs::read_to_string(out.join("lib.rs")))
        .unwrap_or_else(|_| {
            let mut acc = String::new();
            if let Ok(entries) = fs::read_dir(&out) {
                for e in entries.flatten() {
                    if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                        acc.push_str(&fs::read_to_string(e.path()).unwrap_or_default());
                    }
                }
            }
            acc
        });
    eprintln!("generated:\n{rs}");

    let owned_empty = rs.contains("String::from(\"\")")
        || rs.contains("String::new()")
        || rs.contains("\"\".to_string()");
    let formals_owned = rs.contains("origin: String")
        && rs.contains("accept_encoding: String")
        && rs.contains("client_key: String");
    if formals_owned {
        assert!(
            owned_empty,
            "owned String formals must own empty literals, not pass \"\":\n{rs}"
        );
    }

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "empty literals into owned String formals must cargo-check.\n{rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
