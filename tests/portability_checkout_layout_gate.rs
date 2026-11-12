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

//! Portability gate: where a checkout lives, and what its directories are called, must not
//! change the output layout or the generated Rust. The same tiny program is built from
//! (a) a checkout directory named `src`, (b) `<tmp>/src/<repo>` with a stray `.wj` one level
//! down, (c) an ordinary directory, with and without a `wj.toml` project boundary.
//! Run: `cargo test --test portability_checkout_layout_gate`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use windjammer::project_paths::find_source_root;

const PROGRAM: &str = "fn main() {\n    let total = 40 + 2\n    println(\"{}\", total)\n}\n";

fn tree(dir: &Path, base: &Path, out: &mut BTreeMap<String, String>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            tree(&p, base, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.insert(
                p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"),
                fs::read_to_string(&p).unwrap(),
            );
        }
    }
}

/// Build `<checkout>/work/prog.wj` and return the generated `.rs` tree (relative path -> text).
fn build_in(checkout: &Path, with_manifest: bool, stray: bool) -> BTreeMap<String, String> {
    let work = checkout.join("work");
    fs::create_dir_all(&work).unwrap();
    if with_manifest {
        fs::write(checkout.join("wj.toml"), "[package]\nname = \"gate\"\nversion = \"0.1.0\"\n")
            .unwrap();
    }
    if stray {
        fs::create_dir_all(checkout.join("other")).unwrap();
        fs::write(checkout.join("other/stray.wj"), "pub fn stray() -> i64 {\n    1\n}\n").unwrap();
    }
    let file = work.join("prog.wj");
    fs::write(&file, PROGRAM).unwrap();
    let out = checkout.join("out");
    let res = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(checkout)
        .args(["build", "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .arg(&file)
        .output()
        .expect("spawn wj");
    assert!(
        res.status.success(),
        "wj build failed in {checkout:?}:\n{}{}",
        String::from_utf8_lossy(&res.stdout),
        String::from_utf8_lossy(&res.stderr)
    );
    let mut files = BTreeMap::new();
    tree(&out, &out, &mut files);
    assert!(!files.is_empty(), "no .rs generated in {checkout:?}");
    files
}

fn layouts() -> Vec<(&'static str, TempDir, PathBuf, bool)> {
    let a = TempDir::new().unwrap();
    let a_dir = a.path().join("src"); // (a) checkout named `src`
    let b = TempDir::new().unwrap();
    let b_dir = b.path().join("src").join("some-repo"); // (b) <tmp>/src/<repo>
    let c = TempDir::new().unwrap();
    let c_dir = c.path().join("plain-checkout"); // (c) ordinary name
    vec![
        ("a: checkout named src", a, a_dir, false),
        ("b: <tmp>/src/<repo> + stray .wj", b, b_dir, true),
        ("c: ordinary dir", c, c_dir, false),
    ]
}

#[test]
fn checkout_name_and_parent_dir_do_not_change_layout_or_codegen() {
    for with_manifest in [false, true] {
        let mut results: Vec<(&str, BTreeMap<String, String>)> = Vec::new();
        for (label, _guard, dir, stray) in layouts() {
            fs::create_dir_all(&dir).unwrap();
            results.push((label, build_in(&dir, with_manifest, stray)));
        }
        let (base_label, base) = &results[2];
        for (label, got) in &results {
            assert_eq!(
                got, base,
                "[manifest={with_manifest}] layout/codegen for `{label}` differs from `{base_label}`"
            );
        }
    }
}

#[test]
fn source_root_ignores_directory_names() {
    for with_manifest in [false, true] {
        for (label, _guard, dir, stray) in layouts() {
            fs::create_dir_all(&dir).unwrap();
            let work = dir.join("work");
            fs::create_dir_all(&work).unwrap();
            if with_manifest {
                fs::write(dir.join("wj.toml"), "[package]\nname = \"g\"\nversion = \"0.1.0\"\n")
                    .unwrap();
            }
            if stray {
                fs::create_dir_all(dir.join("other")).unwrap();
                fs::write(dir.join("other/stray.wj"), "pub fn s() {}\n").unwrap();
            }
            let file = work.join("prog.wj");
            fs::write(&file, PROGRAM).unwrap();
            assert_eq!(
                find_source_root(&file),
                Some(work.as_path()),
                "[manifest={with_manifest}] `{label}`: a standalone file's root is its own dir"
            );
        }
    }
}

#[test]
fn project_boundary_stops_the_ancestor_walk() {
    // A real `src` ABOVE a project boundary must not capture files inside the project.
    let tmp = TempDir::new().unwrap();
    let outer_src = tmp.path().join("src");
    let proj = outer_src.join("proj");
    let sub = proj.join("lib");
    fs::create_dir_all(&sub).unwrap();
    fs::write(outer_src.join("main.wj"), "fn main() {}\n").unwrap(); // would qualify `src` by content
    fs::write(proj.join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\n").unwrap();
    let file = sub.join("m.wj");
    fs::write(&file, "pub fn f() {}\n").unwrap();
    assert_eq!(find_source_root(&file), Some(sub.as_path()));
}
