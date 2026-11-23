//! ADR-0040: a module re-exports a name only through an explicit `pub use`.
//!
//! `tests/fixtures/explicit_reexports/before` relies on the implicit rule (`adapters::serve`, `use adapters::Server`
//! with both items declared in the child `adapters::http`); `after` is the same program with the `pub use` lines.
//! `wj check`, `wj run --interpret` and `wj build` refuse `before` with WJ0194, `wj fix` turns it into `after` (and a
//! second run changes nothing), and `after` passes all three with the same output. The pair is the gate's own
//! mutation check: with the resolver rule restored, `before` passes `wj check` and the first test fails.

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/explicit_reexports").join(name)
}

fn run(dir: &Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(WJ).args(args).current_dir(dir).output().unwrap();
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
