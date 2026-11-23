#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! `[dev-dependencies]` path dependencies are visible to a package's tests and never to its library build:
//! a test may `use` a dev-dependency, while the same `use` in `src/` is not resolved.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, text) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    dir
}

const DEP: [(&str, &str); 2] = [
    ("helper/wj.toml", "[package]\nname = \"helper_kit\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n"),
    ("helper/src/lib.wj", "pub fn answer() -> int { 42 }\n"),
];

fn project(manifest_deps: &str, test_body: &str) -> TempDir {
    let manifest = format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n\n{manifest_deps}");
    write_project(&[
        DEP[0],
        DEP[1],
        ("app/wj.toml", &manifest),
        ("app/src/lib.wj", "pub fn one() -> int { 1 }\n"),
        ("app/tests/a_test.wj", test_body),
    ])
}

const TEST: &str = "use helper_kit::answer\n\n@test\nfn test_answer() {\n    assert_eq(answer(), 42)\n}\n";

#[test]
fn interpreted_test_sees_dev_dependency() {
    let dir = project("[dev-dependencies]\nhelper_kit = { path = \"../helper\" }\n", TEST);
    let o = Command::new(WJ).args(["test", "--interpret"]).current_dir(dir.path().join("app")).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success() && text.contains("1 passed"), "dev-dependency not visible to the interpreted test:\n{text}");
}

#[test]
fn interpreted_test_without_the_dev_dependency_is_rejected() {
    let dir = project("", TEST);
    let o = Command::new(WJ).args(["test", "--interpret"]).current_dir(dir.path().join("app")).output().unwrap();
    assert!(!o.status.success(), "a test using an undeclared crate must fail");
}

#[test]
fn library_run_does_not_see_dev_dependency() {
    let dir = project("[dev-dependencies]\nhelper_kit = { path = \"../helper\" }\n", TEST);
    fs::write(dir.path().join("app/src/main.wj"), "use helper_kit::answer\n\nfn main() {\n    println(\"${answer()}\")\n}\n").unwrap();
    let o = Command::new(WJ).args(["run", "--interpret", "src/main.wj"]).current_dir(dir.path().join("app")).output().unwrap();
    assert!(!o.status.success(), "a dev-dependency must not be visible outside tests");
}
