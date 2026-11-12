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

//! Calls into a hand-written Rust module of the same crate (`src/host.rs`, no Windjammer signature).
//!
//! 1. `use crate::host` then `host::make("div")` names the same function as `use crate::host::make` then
//!    `make("div")`: a same-crate definition, so the call is emitted as written, never as a missing boundary
//!    signature (`compile_error!`).
//! 2. `Node::element(tag: string)` forwards `tag` to `make`, so its emitted formal is an owned `String`; a call
//!    with a literal (`Node::element("hr")`, alone or as the receiver of a chain) converts the literal, as a free
//!    function with the same formal does.
//!
//! Oracle: rustc compiles the generated crate with the real `host` module appended, and the program prints what
//! the source computes.

use std::fs;
use std::path::Path;
use std::process::Command;

const HOST_RS: &str = "pub fn make(tag: impl AsRef<str>) -> u64 { tag.as_ref().len() as u64 }\n";

/// Builds `lib_wj` as a library crate holding `src/host.rs`, wraps the generated `lib.rs` beside `host` and
/// `main_rs`, compiles it as a binary and returns (generated Rust, program output).
fn build_and_run(dir: &Path, lib_wj: &str, main_rs: &str) -> (String, String) {
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(dir.join("wj.toml"), "[package]\nname = \"host-calls\"\nversion = \"0.1.0\"\n").unwrap();
    fs::write(src.join("lib.wj"), lib_wj).unwrap();
    fs::write(src.join("host.rs"), HOST_RS).unwrap();
    let out = dir.join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "src", "--no-cargo", "-o"])
        .arg(&out)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(build.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&build.stderr));
    let generated = fs::read_to_string(out.join("lib.rs")).unwrap();
    let crate_rs = dir.join("crate.rs");
    // The generated library is the crate's `generated` module; `crate::host` is the hand-written module beside it.
    fs::write(&crate_rs, format!("pub mod host {{ {HOST_RS} }}\nmod generated {{\n{generated}\n}}\nuse generated::*;\n{main_rs}\n"))
        .unwrap();
    let exe = dir.join("prog");
    let rustc = Command::new("rustc").args(["--edition", "2021", "-A", "warnings", "-o"]).arg(&exe).arg(&crate_rs).output().unwrap();
    assert!(rustc.status.success(), "rustc rejected the crate:\n{}\n{generated}", String::from_utf8_lossy(&rustc.stderr));
    let run = Command::new(&exe).output().unwrap();
    (generated, String::from_utf8_lossy(&run.stdout).into_owned())
}

#[test]
fn module_qualified_call_into_a_rust_module_is_a_plain_call() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = "use crate::host\n\npub fn qualified() -> u64 {\n    host::make(\"div\")\n}\n";
    let (generated, out) = build_and_run(tmp.path(), lib, "fn main() { println!(\"{}\", qualified()); }");
    assert!(!generated.contains("compile_error!"), "{generated}");
    assert_eq!(out, "3\n");
}

#[test]
fn literal_into_owned_text_formal_of_an_associated_fn_is_converted() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = r#"use crate::host::make

pub struct Node {
    handle: u64,
}

impl Node {
    pub fn element(tag: string) -> Node {
        Node { handle: make(tag) }
    }
    pub fn style(self, css: string) -> Node {
        Node { handle: self.handle + css.len() as u64 }
    }
}

pub fn divider() -> Node {
    Node::element("hr").style("margin: 8px 0")
}

pub fn plain() -> Node {
    Node::element("img")
}
"#;
    let main = "fn main() { println!(\"{} {}\", divider().handle, plain().handle); }";
    let (_generated, out) = build_and_run(tmp.path(), lib, main);
    assert_eq!(out, "15 3\n");
}
