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

//! FAILING REPRO — cross-module call to decode helper with `&str` formal must auto-borrow.
//!
//! Dogfood (`adapters/inbound/routes.wj` → `login_decode.parse_login_request`):
//! ```ignore
//! parse_login_request(body + "")
//! ```
//! Tip emits `parse_login_request(_temp0)` (String) for `body: &str` formal (E0308).
//! Product workaround: `parse_login_request(clone_body(body + ""))` → `&clone_body(...)`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_module_decode_call_must_auto_borrow_owned_body() {
    let tmp = TempDir::new().unwrap();
    let lib_dir = tmp.path().join("lib");
    let routes_dir = tmp.path().join("routes");
    let decode_dir = tmp.path().join("decode");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::create_dir_all(&routes_dir).unwrap();
    fs::create_dir_all(&decode_dir).unwrap();
    fs::write(
        decode_dir.join("login.wj"),
        r#"
pub fn parse_login_request(body: string) -> int {
    strings.len(body + "")
}
"#,
    )
    .unwrap();
    fs::write(
        routes_dir.join("routes.wj"),
        r#"
use login::parse_login_request

pub fn handle(body: string) -> int {
    parse_login_request(body + "")
}
"#,
    )
    .unwrap();
    fs::write(
        lib_dir.join("mod.wj"),
        r#"
mod login { include "../decode/login.wj" }
mod routes { include "../routes/routes.wj" }

pub fn entry(body: string) -> int {
    routes::handle(body + "")
}

fn main() {}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let out = tmp.path().join("out");
    let build = Command::new(wj)
        .args([
            "build",
            lib_dir.join("mod.wj").to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--check",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "cross-module decode call must auto-borrow owned body. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
