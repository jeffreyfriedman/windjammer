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

//! Product-shaped `NotesApp::handle` + `handle_request` empty lits must own.
//!
//! P3.499 tiny `App { n: int }` isolate already emits `"".to_string()`.
//! Ecosystem `wj-notes-api` still emits `app.handle(&method, path, "", "", "", …)`
//! (E0308). This fixture matches the product graph: HashMap app, `parse_method`,
//! `handle_method(..., "")`, `own` rebinds.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn handle_request_empty_lits_hex_app_must_own() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
fn own(value: string) -> string {
    value
}

enum HttpMethod {
    GET,
    POST,
}

struct RateBucket {
    count: int,
}

struct NotesApp {
    n: int,
    buckets: HashMap<string, RateBucket>,
}

impl NotesApp {
    pub fn new() -> NotesApp {
        NotesApp {
            n: 0,
            buckets: HashMap::new(),
        }
    }

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
        let parsed = match parse_method(method) {
            Some(value) => value,
            None => return "405",
        }
        self.handle_method(parsed, path, origin, accept_encoding, client_key, now_ms, body, "")
    }

    fn handle_method(
        self,
        method: HttpMethod,
        path: string,
        origin: string,
        accept_encoding: string,
        client_key: string,
        now_ms: int,
        body: string,
        if_none_match: string,
    ) -> string {
        let path = own(path)
        let origin = own(origin)
        let accept_encoding = own(accept_encoding)
        let client_key = own(client_key)
        let body = own(body)
        let if_none_match = own(if_none_match)
        "${path}:${origin}:${accept_encoding}:${client_key}:${now_ms}:${body}:${if_none_match}"
    }
}

fn parse_method(label: string) -> Option<HttpMethod> {
    if label == "GET" {
        return Some(HttpMethod::GET)
    }
    if label == "POST" {
        return Some(HttpMethod::POST)
    }
    None
}

pub fn handle_request(method: string, path: string, body: string) -> string {
    let mut app = NotesApp::new()
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
        !api_rs.contains("path, \"\", \"\", \"\"")
            && !api_rs.contains("body, \"\"")
            && !api_rs.contains("body,\"\""),
        "hex NotesApp handle_request / handle_method must own empty literals:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "hex handle empty lits must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
