//! ADR-0040: a module re-exports a name only through an explicit `pub use`.
//!
//! `tests/fixtures/explicit_reexports/before` relies on the implicit rule (`adapters::serve`, `use adapters::Server`
//! with both items declared in the child `adapters::http`); `after` is the same program with the `pub use` lines.
//! `wj check`, `wj run --interpret` and `wj build` refuse `before` with WJ0194, `wj fix` turns it into `after` (and a
//! second run changes nothing), and `after` passes all three with the same output. The pair is the gate's own
//! mutation check: with the resolver rule restored, `before` passes `wj check` and the first test fails.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The `wj` under test: a private copy named by `CARGO_BIN_EXE_wj` at run time, else the one cargo built.
fn wj_bin() -> String {
    std::env::var("CARGO_BIN_EXE_wj").unwrap_or_else(|_| env!("CARGO_BIN_EXE_wj").to_string())
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/explicit_reexports").join(name)
}

fn run(dir: &Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(wj_bin()).args(args).current_dir(dir).output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (p, q) = (e.path(), to.join(e.file_name()));
        if p.is_dir() {
            copy_dir(&p, &q);
        } else {
            std::fs::copy(&p, &q).unwrap();
        }
    }
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wj-reexports-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}

#[test]
fn implicit_reexport_is_refused_by_check_interpreter_and_build() {
    let dir = fixture("before");
    let out = scratch("build");
    let out_s = out.to_string_lossy().to_string();
    for (label, args) in [
        ("check", vec!["check"]),
        ("interpret", vec!["run", "--interpret"]),
        ("build", vec!["build", "src", "--no-cargo", "-o", out_s.as_str()]),
    ] {
        let (ok, text) = run(&dir, &args);
        assert!(!ok, "[{label}] must fail:\n{text}");
        assert!(text.contains("WJ0194"), "[{label}] code missing:\n{text}");
        assert!(text.contains("is not re-exported by `adapters`"), "[{label}] message missing:\n{text}");
        assert!(text.contains("pub use http::"), "[{label}] fix hint missing:\n{text}");
    }
    let (_, text) = run(&dir, &["check"]);
    assert!(text.contains("`serve` is not re-exported") && text.contains("`Server` is not re-exported"), "{text}");
    assert!(!text.contains("`encode` is not re-exported"), "an explicit `pub use` is accepted:\n{text}");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn explicit_reexport_passes_and_prints_the_same() {
    let dir = fixture("after");
    let out = scratch("after-build");
    let out_s = out.to_string_lossy().to_string();
    let (ok, text) = run(&dir, &["check"]);
    assert!(ok && !text.contains("WJ0194"), "{text}");
    let (ok, text) = run(&dir, &["run", "--interpret"]);
    assert!(ok, "{text}");
    assert!(text.contains(read(&dir.join("expect.stdout")).trim()), "{text}");
    let (ok, text) = run(&dir, &["build", "src", "--no-cargo", "-o", out_s.as_str()]);
    assert!(ok, "{text}");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn naming_the_declaring_module_needs_no_reexport() {
    let tmp = scratch("declaring");
    copy_dir(&fixture("before"), &tmp);
    std::fs::write(tmp.join("src/main.wj"), "mod adapters\n\nuse adapters::http::Server\n\nfn main() {\n    let s = Server::new(1)\n    println(\"${s.port} ${adapters::http::serve()}\")\n}\n").unwrap();
    let (ok, text) = run(&tmp, &["check"]);
    assert!(ok && !text.contains("WJ0194"), "{text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn wj_fix_adds_the_missing_pub_use_lines_and_is_idempotent() {
    let tmp = scratch("fix");
    copy_dir(&fixture("before"), &tmp);
    let (ok, text) = run(&tmp, &["fix", "--check", "."]);
    assert!(!ok, "--check must fail while a line is missing:\n{text}");
    assert_eq!(read(&tmp.join("src/adapters/mod.wj")), read(&fixture("before").join("src/adapters/mod.wj")), "--check writes nothing");

    let (ok, text) = run(&tmp, &["fix", "."]);
    assert!(ok, "{text}");
    let fixed = read(&tmp.join("src/adapters/mod.wj"));
    assert_eq!(fixed, read(&fixture("after").join("src/adapters/mod.wj")), "after wj fix:\n{fixed}");
    for f in ["src/main.wj", "src/adapters/http.wj", "src/adapters/codec.wj"] {
        assert_eq!(read(&tmp.join(f)), read(&fixture("before").join(f)), "{f} must not change");
    }

    let (ok, text) = run(&tmp, &["fix", "--check", "."]);
    assert!(ok, "second fix would still change a file:\n{text}");
    let (ok, text) = run(&tmp, &["fix", "."]);
    assert!(ok, "{text}");
    assert_eq!(read(&tmp.join("src/adapters/mod.wj")), fixed, "idempotent");

    let (ok, text) = run(&tmp, &["check"]);
    assert!(ok && !text.contains("WJ0194"), "{text}");
    let (ok, text) = run(&tmp, &["run", "--interpret"]);
    assert!(ok && text.contains("8080 7 20"), "{text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn inline_module_has_no_file_to_fix_and_is_left_alone() {
    let tmp = scratch("inline");
    std::fs::create_dir_all(&tmp).unwrap();
    let src = "mod adapters {\n    pub mod http {\n        pub fn serve() -> int {\n            1\n        }\n    }\n}\n\nfn main() {\n    println(\"${adapters::serve()}\")\n}\n";
    std::fs::write(tmp.join("main.wj"), src).unwrap();
    let (ok, text) = run(&tmp, &["check", "main.wj"]);
    assert!(!ok && text.contains("WJ0194"), "{text}");
    let (_, text) = run(&tmp, &["fix", "main.wj"]);
    assert!(text.contains("manual"), "{text}");
    assert_eq!(read(&tmp.join("main.wj")), src);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn glob_import_of_a_module_does_not_reach_a_childs_items() {
    let tmp = scratch("glob");
    copy_dir(&fixture("before"), &tmp);
    std::fs::write(tmp.join("src/main.wj"), "mod adapters\n\nuse adapters::*\n\nfn main() {\n    println(\"${serve()}\")\n}\n").unwrap();
    std::fs::write(tmp.join("src/adapters/mod.wj"), "pub mod http\npub mod codec\n").unwrap();
    let (ok, text) = run(&tmp, &["check"]);
    assert!(!ok && text.contains("`serve` is not re-exported by `adapters`"), "{text}");
    let (ok, text) = run(&tmp, &["fix", "."]);
    assert!(ok, "{text}");
    assert_eq!(read(&tmp.join("src/adapters/mod.wj")), "pub mod http\npub mod codec\npub use http::serve\n");
    // a deliberate glob in the module is an explicit re-export
    std::fs::write(tmp.join("src/adapters/mod.wj"), "pub mod http\npub mod codec\npub use http::*\n").unwrap();
    let (ok, text) = run(&tmp, &["check"]);
    assert!(ok && !text.contains("WJ0194"), "{text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn a_name_that_is_a_module_and_a_function_is_explicit_as_the_module() {
    // `use crate::inflate` imports the module `inflate` (which also declares `pub fn inflate`): not an implicit re-export
    let tmp = scratch("modfn");
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(tmp.join("wj.toml"), "[package]\nname = \"modfn\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(tmp.join("src/main.wj"), "mod inflate\nmod zip\n\nfn main() {\n    println(\"${zip::run()}\")\n}\n").unwrap();
    std::fs::write(tmp.join("src/zip.wj"), "use crate::inflate\n\npub fn run() -> int {\n    inflate::inflate(3)\n}\n").unwrap();
    std::fs::write(tmp.join("src/inflate.wj"), "pub fn inflate(n: int) -> int {\n    n + 1\n}\n").unwrap();
    let (ok, text) = run(&tmp, &["check"]);
    assert!(ok && !text.contains("WJ0194"), "{text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn wj0194_module_paths_never_carry_the_checked_files_location() {
    // the same file checked as `wj check x.wj` and from a directory root (the file nested below it) names modules the same way
    let tmp = scratch("relative");
    std::fs::create_dir_all(tmp.join("deep/er")).unwrap();
    let src = "mod adapters {\n    pub mod http {\n        pub fn serve() -> int {\n            1\n        }\n    }\n}\n\nfn main() {\n    println(\"${adapters::serve()}\")\n}\n";
    std::fs::write(tmp.join("deep/er/prog.wj"), src).unwrap();
    let want = "`serve` is not re-exported by `adapters`; it is declared in `adapters::http`";
    for (label, dir, args) in [("file", tmp.join("deep/er"), vec!["check", "prog.wj"]), ("dir root", tmp.clone(), vec!["check", "."])] {
        let (ok, text) = run(&dir, &args);
        assert!(!ok && text.contains(want), "[{label}] module path must be relative to the crate:\n{text}");
        assert!(!text.contains("prog::") && !text.contains("deep::"), "[{label}] checked file's path leaked:\n{text}");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}
