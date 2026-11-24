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

//! `wj build src/main.wj` in a project builds the modules main.wj declares exactly as `wj build src` does. A module
//! re-exports the public items of its child modules (the resolver's `lookup_child_reexports`, which `wj check` and
//! the interpreter follow), so `use crate::adapters::serve` names `adapters::http_server::serve`: the generated
//! `adapters/mod.rs` declares `pub use self::http_server::*`. Before, building the entry file alone emitted only
//! main.rs and rustc rejected the import (E0432).

use std::path::Path;
use std::process::{Command, Output};

fn project(dir: &Path) {
    let w = |rel: &str, text: &str| {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    w("wj.toml", "[package]\nname = \"rx\"\nversion = \"0.1.0\"\n");
    w("src/main.wj", "pub mod adapters\n\nuse crate::adapters::serve\n\nfn main() {\n    println(\"${serve(2)}\")\n}\n");
    w("src/adapters/mod.wj", "pub mod http_server\npub use http_server::serve\n");
    w("src/adapters/http_server.wj", "pub fn serve(n: int) -> int {\n    n\n}\n");
}

fn wj(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(dir).env("WJ_CARGO_TARGET_ROOT", dir.join(".cache")).output().unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

#[test]
fn entry_file_build_generates_its_declared_modules_as_the_project_build_does() {
    let d = tempfile::tempdir().unwrap();
    project(d.path());
    let check = wj(d.path(), &["check", "src/main.wj"]);
    assert!(check.status.success(), "the program follows the explicit re-export rule (ADR 0040):\n{}", text(&check));
    let alone = wj(d.path(), &["build", "src/main.wj", "-o", "out-entry"]);
    let t = text(&alone);
    assert!(alone.status.success(), "wj build src/main.wj (with cargo) failed:\n{t}");
    for leak in ["error[E0", "rustc --explain"] {
        assert!(!t.contains(leak), "rustc text leaked (`{leak}`):\n{t}");
    }
    let whole = wj(d.path(), &["build", "src", "--no-cargo", "-o", "out-project"]);
    assert!(whole.status.success(), "wj build src failed:\n{}", text(&whole));
    for rel in ["adapters/mod.rs", "adapters/http_server.rs"] {
        let a = std::fs::read_to_string(d.path().join("out-entry").join(rel)).unwrap_or_default();
        let b = std::fs::read_to_string(d.path().join("out-project").join(rel)).unwrap_or_default();
        assert!(!a.is_empty() && a == b, "{rel} differs from the project build:\n--- entry\n{a}\n--- project\n{b}");
    }
    let m = std::fs::read_to_string(d.path().join("out-entry/adapters/mod.rs")).unwrap();
    assert!(m.contains("pub use self::http_server::serve;"), "the module re-exports `serve` through its explicit pub use (ADR 0040):\n{m}");
    let run = wj(d.path(), &["run", "--compiled", "src/main.wj"]);
    assert!(run.status.success() && String::from_utf8_lossy(&run.stdout).trim() == "2", "compiled run:\n{}", text(&run));
}
