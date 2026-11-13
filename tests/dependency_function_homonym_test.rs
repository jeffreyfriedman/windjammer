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

//! A file's own function is the contract for its bare calls. `rowlib::rows` declares `is_row_key(k: u8)` and a
//! constant; the app imports only the constant and declares its own private `is_row_key(prefix_len: usize, key)`.
//! Numeric inference must type the app's call from the app's declaration (usize), never from the dependency's
//! metadata (u8), whose same-named function the app never imports. (The project-corpus case
//! `own_function_beside_a_same_named_module_function` is the same program in one crate, compiled by rustc.)

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn wj(args: &[&str], cwd: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(cwd).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "wj {args:?} failed:\n{text}");
    text
}

#[test]
fn own_function_is_not_typed_from_a_dependency_homonym() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    write(t, "rowlib/wj.toml", "[package]\nname = \"rowlib\"\nversion = \"0.1.0\"\n\n[lib]\n");
    write(t, "rowlib/src/mod.wj", "pub mod rows\n");
    write(t, "rowlib/src/rows.wj", "pub fn is_row_key(k: u8) -> bool {\n    k >= 4\n}\n\npub const KIND_ROW: u8 = 1\n");
    write(t, "app/src/mod.wj", "pub mod m\n");
    write(
        t,
        "app/src/m.wj",
        "use rowlib::rows::KIND_ROW\n\nfn is_row_key(prefix_len: usize, key: Vec<u8>) -> bool {\n    key.len() == prefix_len + 8\n}\n\n\
         pub fn count(prefix_len: usize, keys: Vec<Vec<u8>>) -> usize {\n    let mut n: usize = 0\n    let mut i: usize = 0\n    \
         while i < keys.len() {\n        if is_row_key(prefix_len, keys[i].clone()) {\n            n = n + 1\n        }\n        i = i + 1\n    }\n    n\n}\n\n\
         pub fn kind() -> u8 {\n    KIND_ROW\n}\n",
    );
    let lib_args = ["--library", "--module-file", "--no-cargo", "--no-generate-cargo-toml", "--no-lint"];
    let mut args = vec!["build", "src", "-o", "gen"];
    args.extend(lib_args);
    wj(&args, &t.join("rowlib"));
    let meta = t.join("rowlib/gen/metadata.json");
    let meta_arg = format!("rowlib={}", meta.display());
    let mut args = vec!["build", "src", "-o", "gen"];
    args.extend(lib_args);
    args.extend(["--metadata", meta_arg.as_str()]);
    let out = wj(&args, &t.join("app"));
    assert!(!out.contains("inference error"), "the app's own is_row_key takes a usize:\n{out}");
    let m_rs = fs::read_to_string(t.join("app/gen/m.rs")).unwrap();
    assert!(m_rs.contains("is_row_key(prefix_len"), "the call names the app's own function:\n{m_rs}");
}
