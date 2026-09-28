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

//! P3.530: `use std::url` plus a local `Url` (extra `port`, `query: string`)
//! must not rewrite local `Url { … }` literals to `windjammer_runtime::url::Url`
//! (no `port`; `query: Option<String>`).
//!
//! Product `wj-url`: `join_url` thin-wraps `url.join`. Isolates without
//! `use std::url` stay GREEN. Do not reshape the package (rename Url / drop wrap).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::url

pub struct Url {
    pub scheme: string,
    pub host: string,
    pub port: string,
    pub path: string,
    pub query: string,
    pub fragment: string,
}

pub fn join_url(base: string, relative: string) -> Result<string, string> {
    url.join(base, relative)
}

pub fn parse_local() -> Url {
    Url {
        scheme: "https",
        host: "example.com",
        port: "443",
        path: "/",
        query: "",
        fragment: "",
    }
}
"#;

#[test]
fn local_url_struct_must_not_emit_runtime_url() {
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
        "P3.530 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.530 local Url emit:\n{rs}");
    assert!(
        !rs.contains("windjammer_runtime::url::Url {"),
        "P3.530 RED: local Url literal must not emit runtime url::Url:\n{rs}"
    );
    assert!(
        rs.contains("port:") || rs.contains("port,"),
        "P3.530: local Url has port:\n{rs}"
    );
}
