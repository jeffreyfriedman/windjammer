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

//! Same-crate `query: String` must not receive `&mut query`.
//!
//! Ecosystem `wj-notes-api` `note_get_reply(note, if_none_match, query)`:
//! formal stays `query: String` (consumed by `query_wants_base64`) but the
//! GET arm emits `note_get_reply(&mut note, if_none_match, &mut query)` (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn owned_string_formal_must_not_receive_mut_query() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
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
    if query_wants_base64(query) {
        return note.uid
    }
    "${note.title}:${note.body}"
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
        !api_rs.contains("note_get_reply(") || !api_rs.contains("&mut query"),
        "owned query: String must not receive &mut query:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "owned String formal must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
