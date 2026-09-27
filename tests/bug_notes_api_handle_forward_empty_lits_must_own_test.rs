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

//! Product-shaped `handle` forwards origin/accept_encoding/client_key into
//! another owned `string` slot. Empty literals at `handle_request` must own.
//!
//! P3.489 isolate with interpolation+keep already emits `"".to_string()`.
//! Ecosystem `wj-notes-api` `handle_request` still emits
//! `app.handle(&method, path, "", "", "", 0_i64, body)` (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn handle_forward_empty_lits_must_own() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
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
        method: string,
        path: string,
        origin: string,
        accept_encoding: string,
        client_key: string,
        now_ms: int,
        body: string,
    ) -> string {
        let method = keep(method)
        let path = keep(path)
        let body = keep(body)
        self.inner(origin, accept_encoding, client_key, now_ms, method, path, body)
    }

    fn inner(
        self,
        origin: string,
        accept_encoding: string,
        client_key: string,
        now_ms: int,
        method: string,
        path: string,
        body: string,
    ) -> string {
        let origin = keep(origin)
        let accept_encoding = keep(accept_encoding)
        let client_key = keep(client_key)
        "${method}:${path}:${origin}:${accept_encoding}:${client_key}:${now_ms}:${body}"
    }
}

pub fn handle_request(method: string, path: string, body: string) -> string {
    let mut app = App { n: 0 }
    app.handle(method, path, "", "", "", 0, body)
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("out");
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

    let api_rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_default();
    eprintln!("api.rs:\n{api_rs}");
    assert!(
        !api_rs.contains("handle(") || !api_rs.contains("path, \"\", \"\", \"\""),
        "forwarded owned String formals must own empty literals:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "handle forward empty lits must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
