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

//! After `method` demotes to `&str`, empty lits into later owned `String`
//! formals must still own (`"".to_string()`).
//!
//! P3.489 / P3.499 isolates kept every formal owned (`keep(method)`) and
//! already emitted `"".to_string()`. Product `handle_request` emits
//! `app.handle(&method, path, "", "", "", 0_i64, body)` because
//! `parse_method(method)` demotes the first slot.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn demoted_method_then_owned_empty_lits_must_own() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
fn parse_method(method: string) -> bool {
    method == "GET"
}

fn own(value: string) -> string {
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
        if !parse_method(method) {
            return ""
        }
        let origin = own(origin)
        let accept_encoding = own(accept_encoding)
        let client_key = own(client_key)
        let path = own(path)
        let body = own(body)
        "${path}:${origin}:${accept_encoding}:${client_key}:${now_ms}:${body}"
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
        !api_rs.contains("path, \"\", \"\", \"\""),
        "demoted method must not leave later owned String formals as bare empty lits:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "demoted-method then owned empty lits must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
