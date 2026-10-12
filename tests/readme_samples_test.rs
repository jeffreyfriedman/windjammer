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

//! The README's side-by-side sample: the Windjammer program and the hand-written Rust program must both
//! compile, print the same thing, and appear verbatim in README.md (so the page cannot drift from the test).
//! Oracle: `playlist.expected` is typed by hand, independent of either compiler path.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme_samples");
const WJ_SRC: &str = include_str!("readme_samples/playlist.wj");
const RS_SRC: &str = include_str!("readme_samples/playlist.rs");
const EXPECTED: &str = include_str!("readme_samples/playlist.expected");
const README: &str = include_str!("../README.md");

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().expect("spawn");
    assert!(
        out.status.success(),
        "{:?} failed: {}{}",
        cmd,
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn rustc(src: &Path, bin: &Path) {
    run(Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(bin)
        .arg(src));
}

#[test]
fn rust_version_compiles_and_prints_expected() {
    let d = tempdir().unwrap();
    let bin = d.path().join("rs_bin");
    rustc(Path::new(DIR).join("playlist.rs").as_path(), &bin);
    assert_eq!(run(&mut Command::new(&bin)), EXPECTED);
}

#[test]
fn windjammer_version_compiles_to_rust_and_prints_expected() {
    let d = tempdir().unwrap();
    let src = d.path().join("playlist.wj");
    fs::write(&src, WJ_SRC).unwrap();
    let out = d.path().join("out");
    run(Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(&out)
        .arg(&src));
    let rs = out.join("playlist.rs");
    assert!(rs.exists(), "expected {} to exist", rs.display());
    let generated = fs::read_to_string(&rs).unwrap();
    for sig in [
        "fn add_song(list: &mut Playlist, song: String)",
        "fn count(list: &Playlist)",
        "fn describe(list: &Playlist)",
    ] {
        assert!(
            generated.contains(sig),
            "inferred signature `{sig}` missing from:\n{generated}"
        );
        assert!(
            README.contains(sig),
            "README quotes the inferred signature `{sig}`"
        );
    }
    let bin = d.path().join("wj_bin");
    rustc(&rs, &bin);
    assert_eq!(run(&mut Command::new(&bin)), EXPECTED);
}

#[test]
fn windjammer_version_runs_in_the_interpreter_with_the_same_output() {
    let d = tempdir().unwrap();
    let src = d.path().join("playlist.wj");
    fs::write(&src, WJ_SRC).unwrap();
    let got = run(Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["run", "--interpret"])
        .arg(&src));
    assert_eq!(got, EXPECTED);
}

#[test]
fn readme_contains_both_samples_verbatim() {
    assert!(
        README.contains(WJ_SRC.trim_end()),
        "README.md must contain tests/readme_samples/playlist.wj verbatim"
    );
    assert!(
        README.contains(RS_SRC.trim_end()),
        "README.md must contain tests/readme_samples/playlist.rs verbatim"
    );
}
