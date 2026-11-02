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
//! Generic regression gates from the ecosystem health sweep: package-root `wj check` and `??` on a `Result`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    dir
}

const MANIFEST: &str = "[package]\nname = \"repro\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n";

/// `pub mod shapes` + `pub use shapes::area` in `src/lib.wj`: the module path of `src/shapes.wj` is `shapes`
/// whether `wj check` starts at the package root or at `src`.
#[test]
fn check_at_package_root_resolves_pub_use_of_a_pub_mod() {
    let dir = write_project(&[
        ("wj.toml", MANIFEST),
        ("src/lib.wj", "pub mod shapes\n\npub use shapes::area\n"),
        ("src/shapes.wj", "pub fn area(w: int, h: int) -> int {\n    w * h\n}\n"),
        ("tests/area_test.wj", "use crate::area\n\nfn test_area() {\n    assert_eq(area(2, 3), 6)\n}\n"),
    ]);
    for target in [".", "src"] {
        let o = Command::new(WJ).arg("check").arg(target).current_dir(dir.path()).output().unwrap();
        let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        assert!(o.status.success() && !out.contains("WJ0006"), "wj check {target} reported unresolved imports:\n{out}");
    }
}

/// `(result ?? Vec::new())` on a `Result` lowers to a closure that takes the error, so rustc accepts it.
#[test]
fn coalesce_on_a_result_with_a_lazy_default_type_checks() {
    let dir = write_project(&[(
        "m.wj",
        "pub fn versions(ok: bool) -> Result<Vec<int>, string> {\n    if ok {\n        let mut v = Vec::new()\n        v.push(1)\n        return Ok(v)\n    }\n    Err(\"none\")\n}\n\npub fn count(ok: bool) -> int {\n    (versions(ok) ?? Vec::new()).len()\n}\n",
    )]);
    let o = Command::new(WJ).args(["build", "m.wj", "--no-cargo", "--output", "out"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&o.stderr));
    let r = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "-o"])
        .arg(dir.path().join("o.rmeta"))
        .arg(dir.path().join("out/m.rs"))
        .output()
        .unwrap();
    assert!(r.status.success(), "generated Rust does not type-check:\n{}", String::from_utf8_lossy(&r.stderr));
}
