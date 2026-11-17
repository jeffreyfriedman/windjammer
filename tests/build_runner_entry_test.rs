//! A program that imports sibling modules has a compiled form, and a root-level file with its own `fn main` is a
//! binary of the crate: the build of one file includes the modules it imports; a project build keeps a runner's
//! `fn main` and adds a `[[bin]]` for it (the documented targets: `src/main.wj`, `main.wj`, `[[bin]]`, `src/bin/*.wj`).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write_project(dir: &Path) {
    std::fs::write(dir.join("wj.toml"), "[package]\nname = \"runner_build\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(dir.join("lib_a.wj"), "pub fn double(x: int) -> int { x * 2 }\n").unwrap();
    std::fs::write(dir.join("runner.wj"), "use lib_a::double\n\nfn main() {\n    println(\"${double(21)}\")\n}\n").unwrap();
    // a sibling nothing imports: a file build must not declare it
    std::fs::write(dir.join("unrelated.wj"), "pub fn other() -> int { 7 }\n").unwrap();
}

fn build(target: &Path, out: &Path) {
    let o = Command::new(WJ).args(["build"]).arg(target).args(["--target", "rust", "--no-cargo", "-o"]).arg(out).output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn assert_runner_crate(out: &Path, expect_unrelated_built: bool) {
    let runner = read(&out.join("runner.rs"));
    assert!(runner.contains("fn main"), "the runner's main is kept:\n{runner}");
    assert!(runner.contains("runner_build::lib_a::double"), "the runner imports the library crate by name:\n{runner}");
    assert!(out.join("lib_a.rs").is_file(), "the imported module is built");
    let lib = read(&out.join("lib.rs"));
    assert!(lib.contains("pub mod lib_a;"), "{lib}");
    assert!(!lib.contains("mod runner"), "a binary entry is not a module of the library:\n{lib}");
    assert_eq!(lib.contains("mod unrelated"), expect_unrelated_built, "{lib}");
    let cargo = read(&out.join("Cargo.toml"));
    assert!(cargo.contains("[[bin]]\nname = \"runner\"\npath = \"runner.rs\""), "a [[bin]] for the runner:\n{cargo}");
    assert!(cargo.contains("[lib]"), "{cargo}");
}

#[test]
fn building_one_file_includes_the_modules_it_imports() {
    let d = Scratch::new("runner-file");
    write_project(d.path());
    let out = d.join("out");
    build(&d.join("runner.wj"), &out);
    assert_runner_crate(&out, false);
    assert!(!out.join("unrelated.rs").exists());
}

#[test]
fn a_project_build_keeps_a_runners_main_and_adds_a_bin_target() {
    let d = Scratch::new("runner-project");
    write_project(d.path());
    let out = d.join("out");
    build(d.path(), &out);
    assert_runner_crate(&out, true);
}

#[test]
fn a_file_without_imports_still_builds_alone() {
    let d = Scratch::new("runner-alone");
    std::fs::write(d.join("solo.wj"), "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    let out = d.join("out");
    build(&d.join("solo.wj"), &out);
    assert!(out.join("solo.rs").is_file());
    assert!(!out.join("lib.rs").exists(), "no library for a lone file");
}
