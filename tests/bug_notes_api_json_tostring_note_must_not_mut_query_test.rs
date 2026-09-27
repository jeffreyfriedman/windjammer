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

//! `json.to_string(note)` after `query_wants_base64(query)` must not emit
//! `&mut query` into `query: String`.
//!
//! P3.489 isolate without `json.to_string` stayed GREEN. Product `note_get_reply`
//! serializes `note` after consuming `query` (`note: &mut Note`, `query: String`).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn json_tostring_note_must_not_mut_borrow_query() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
use std::json

struct Note {
    uid: string,
    title: string,
    body: string,
}

fn own(value: string) -> string {
    value
}

fn query_wants_base64(query: string) -> bool {
    let query = own(query)
    query == "encoding=base64"
}

pub fn note_get_reply(note: Note, query: string) -> string {
    let want_base64 = query_wants_base64(query)
    match json.to_string(note) {
        Ok(text) => {
            if want_base64 {
                return text
            }
            text
        },
        Err(_) => "",
    }
}

pub fn dispatch(note: Note, query: string) -> string {
    let query = own(query)
    note_get_reply(note, query)
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
        !api_rs.contains("&mut query") && !api_rs.contains("& mut query"),
        "json.to_string(note) must not force &mut query into String formal:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "json.to_string + owned query must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
