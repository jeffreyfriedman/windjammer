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

//! P3.533: after P3.530, local `Url { … }` is no longer prefixed
//! `windjammer_runtime::url::Url {`, but `use std::url` still emits
//! `use windjammer_runtime::url::Url` which **shadows** the local struct
//! (E0255 / E0560 port / E0308 Option vs String).
//!
//! Product `wj-url` `$WJ test` still 26 rustc. Isolates match. Keep
//! `join_url` → `url.join`. Do not rename the package `Url`.

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

fn imports_runtime_url_type(rs: &str) -> bool {
    rs.lines().any(|l| {
        let t = l.trim();
        t.starts_with("use ")
            && t.contains("windjammer_runtime::url::Url")
            && !t.contains("windjammer_runtime::url::{")
    }) || rs.contains("use windjammer_runtime::url::Url;")
}

#[test]
fn local_url_must_not_import_runtime_url_type() {
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
        "P3.533 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.533 Url import emit:\n{rs}");
    assert!(
        !imports_runtime_url_type(&rs),
        "P3.533 RED: use std::url must not import runtime Url when a local Url exists:\n{rs}"
    );
    assert!(
        rs.contains("url::join"),
        "P3.533: join must still call url::join:\n{rs}"
    );
}
