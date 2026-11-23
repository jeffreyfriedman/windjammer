#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! A project's `src/main.wj` built alone (`wj build src/main.wj --no-cargo -o out`) with `use crate::adapters::...` /
//! `use crate::domain::...` paths into sibling directories (`adapters/mod.wj`, `domain/mod.wj`, no `mod` line in
//! main.wj): the modules those paths name are generated next to `main.rs` and declared there (`mod adapters;`), so the
//! output compiles. A `use crate::name` with no such file keeps its path as written, without an error.

use std::path::Path;
use std::process::{Command, Output};

fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn fixture(dir: &Path) {
    write(dir, "src/main.wj", "use crate::adapters::serve\nuse crate::domain::port_from_env\n\nfn main() {\n    let port = port_from_env()\n    match serve(port) {\n        Ok(_) => {},\n        Err(err) => println(\"error: ${err}\"),\n    }\n}\n");
    write(dir, "src/mod.wj", "pub mod adapters\npub mod domain\n");
    write(dir, "src/adapters/mod.wj", "pub mod http_server\n\npub use http_server::serve\n");
    write(dir, "src/adapters/http_server.wj", "use crate::domain::Config\n\npub fn serve(port: int) -> Result<(), string> {\n    let cfg = Config { port: port }\n    println(\"listening on ${cfg.port}\")\n    Ok(())\n}\n");
    write(dir, "src/domain/mod.wj", "pub mod config\n\npub use config::Config\npub use config::port_from_env\n");
    write(dir, "src/domain/config.wj", "pub struct Config {\n    pub port: int,\n}\n\npub fn port_from_env() -> int {\n    8080\n}\n");
    write(dir, "wj.toml", "[package]\nname = \"lone-main-repro\"\nversion = \"0.1.0\"\nedition = \"2025\"\n");
}

fn build(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "src/main.wj", "--no-cargo", "-o", "out"])
        .current_dir(dir)
        .env("WJ_CARGO_TARGET_ROOT", dir.join(".cache"))
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

#[test]
fn entry_built_alone_generates_and_declares_the_sibling_modules_its_use_paths_name() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path());
    let b = build(d.path());
    assert!(b.status.success(), "{}", text(&b));
    let out = d.path().join("out");
    let main = std::fs::read_to_string(out.join("main.rs")).unwrap();
    for m in ["adapters", "domain"] {
        assert!(main.contains(&format!("mod {m};")), "main.rs does not declare `mod {m};`:\n{main}");
        assert!(out.join(m).join("mod.rs").is_file(), "{m}/mod.rs not generated");
    }
    assert!(out.join("adapters/http_server.rs").is_file() && out.join("domain/config.rs").is_file());
    // the output is a compilable crate root
    let rc = Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "bin", "--emit", "metadata", "-o"])
        .arg(d.path().join("check.rmeta"))
        .arg(out.join("main.rs"))
        .output()
        .unwrap();
    let t = text(&rc);
    assert!(!t.contains("E0432") && !t.contains("E0583"), "unresolved module in the generated crate:\n{t}");
}

#[test]
fn a_used_crate_name_with_no_file_keeps_its_path_and_gives_no_error() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "src/main.wj", "use crate::ghost::boo\n\nfn main() {\n    println(\"hi\")\n}\n");
    write(d.path(), "wj.toml", "[package]\nname = \"ghosty\"\nversion = \"0.1.0\"\nedition = \"2025\"\n");
    let b = build(d.path());
    let t = text(&b);
    assert!(b.status.success(), "a name with no file must not be an error:\n{t}");
    let main = std::fs::read_to_string(d.path().join("out/main.rs")).unwrap();
    assert!(main.contains("use crate::ghost::boo;") && !main.contains("mod ghost;"), "path not preserved as written:\n{main}");
}

/// In a test file `use crate::name` names an item of the LIBRARY under test (here a function), never a sibling
/// module beside the test file: compiled `wj test` must not report a missing module.
#[test]
fn wj_test_use_crate_item_of_the_library_is_not_a_missing_module() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "wj.toml", "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2025\"\n");
    write(d.path(), "src/lib.wj", "pub fn basic_auth_header(user: string) -> string {\n    \"Basic ${user}\"\n}\n");
    write(d.path(), "tests/x_test.wj", "use crate::basic_auth_header\n\npub fn test_header() {\n    assert_eq(basic_auth_header(\"a\"), \"Basic a\")\n}\n");
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["test", "tests/x_test.wj"])
        .current_dir(d.path())
        .env("WJ_CARGO_TARGET_ROOT", d.path().join(".cache"))
        .output()
        .unwrap();
    let t = text(&o);
    assert!(!t.contains("names module"), "a library item was reported as a missing module:\n{t}");
}

/// `use crate::util` (the module itself) beside the `mod util;` the build declares for it is rustc E0255 (the name
/// defined twice): the emitted entry keeps only the declaration, and rustc accepts it with the sibling file beside it.
#[test]
fn whole_module_import_of_a_sibling_is_not_defined_twice() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "wj.toml", "[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2025\"\n");
    write(d.path(), "src/util.wj", "pub fn label(s: string) -> int {\n    s.len()\n}\n");
    write(d.path(), "src/main.wj", "use crate::util\n\nfn main() {\n    let n = util::label(\"abc\")\n    println(\"${n}\")\n}\n");
    let b = build(d.path());
    assert!(b.status.success(), "{}", text(&b));
    let main = d.path().join("out/main.rs");
    let rc = Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "bin", "--emit", "metadata", "-A", "warnings", "-o"])
        .arg(d.path().join("check.rmeta"))
        .arg(&main)
        .output()
        .unwrap();
    assert!(rc.status.success(), "{}\n{}", text(&rc), std::fs::read_to_string(&main).unwrap());
}
