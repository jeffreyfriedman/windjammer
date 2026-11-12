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

/// Where `prog.rs` lands relative to the output dir, per layout, when there is NO manifest above the file: the
/// nearest ancestor named `src` is then the only structure hint (`<dir>/src/ecs/entity.wj` keeps `ecs/`).
fn no_manifest_expected(label: &str) -> &'static str {
    match label.chars().next().unwrap() {
        'a' => "work/prog.rs",              // <checkout = src>/work/prog.wj
        'b' => "some-repo/work/prog.rs",    // <tmp>/src/<repo>/work/prog.wj
        _ => "prog.rs",                     // no `src` anywhere: the file's own directory
    }
}

#[test]
fn checkout_name_and_parent_dir_do_not_change_layout_or_codegen() {
    // With a manifest the project boundary decides: a checkout named `src`, or one under `<tmp>/src`, lays out and
    // generates exactly like an ordinary directory.
    let mut results: Vec<(&str, BTreeMap<String, String>)> = Vec::new();
    for (label, _guard, dir, stray) in layouts() {
        fs::create_dir_all(&dir).unwrap();
        results.push((label, build_in(&dir, true, stray)));
    }
    let (base_label, base) = &results[2];
    for (label, got) in &results {
        assert_eq!(got, base, "[manifest=true] layout/codegen for `{label}` differs from `{base_label}`");
    }
    // Without a manifest the only hint is a directory named `src` above the file: the layout follows it, the
    // generated code does not change.
    let mut baseline_text = None;
    for (label, _guard, dir, stray) in layouts() {
        fs::create_dir_all(&dir).unwrap();
        let got = build_in(&dir, false, stray);
        let want = no_manifest_expected(label);
        assert_eq!(got.keys().map(String::as_str).collect::<Vec<_>>(), vec![want], "[manifest=false] `{label}`");
        let text = got.values().next().unwrap().clone();
        assert_eq!(*baseline_text.get_or_insert(text.clone()), text, "[manifest=false] `{label}`: codegen differs");
    }
}

#[test]
fn source_root_follows_the_manifest_then_a_src_name_only_without_one() {
    for with_manifest in [true, false] {
        for (label, _guard, dir, stray) in layouts() {
            fs::create_dir_all(&dir).unwrap();
            let work = dir.join("work");
            fs::create_dir_all(&work).unwrap();
            if with_manifest {
                fs::write(dir.join("wj.toml"), "[package]\nname = \"g\"\nversion = \"0.1.0\"\n").unwrap();
            }
            if stray {
                fs::create_dir_all(dir.join("other")).unwrap();
                fs::write(dir.join("other/stray.wj"), "pub fn s() {}\n").unwrap();
            }
            let file = work.join("prog.wj");
            fs::write(&file, PROGRAM).unwrap();
            let want: PathBuf = if with_manifest {
                work.clone() // a standalone file's root is its own dir, whatever the checkout is called
            } else {
                match label.chars().next().unwrap() {
                    'a' => dir.clone(),                       // the checkout itself is named `src`
                    'b' => dir.parent().unwrap().to_path_buf(), // <tmp>/src
                    _ => work.clone(),
                }
            };
            assert_eq!(find_source_root(&file), Some(want.as_path()), "[manifest={with_manifest}] `{label}`");
        }
    }
}

#[test]
fn nested_file_without_a_manifest_respects_src_structure() {
    // <tmp>/src/ecs/entity.wj with no wj.toml/Cargo.toml anywhere above: `src` is the root, `ecs/` is kept.
    let tmp = TempDir::new().unwrap();
    let ecs = tmp.path().join("src").join("ecs");
    fs::create_dir_all(&ecs).unwrap();
    let file = ecs.join("entity.wj");
    fs::write(&file, "pub fn entity() -> i64 {\n    1\n}\n").unwrap();
    assert_eq!(find_source_root(&file), Some(tmp.path().join("src").as_path()));
    let out = tmp.path().join("out");
    let res = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(tmp.path())
        .args(["build", "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .arg(&file)
        .output()
        .expect("spawn wj");
    assert!(res.status.success(), "{}{}", String::from_utf8_lossy(&res.stdout), String::from_utf8_lossy(&res.stderr));
    assert!(out.join("ecs/entity.rs").is_file(), "expected out/ecs/entity.rs, got {:?}", {
        let mut m = BTreeMap::new();
        tree(&out, &out, &mut m);
        m.into_keys().collect::<Vec<_>>()
    });
    // the same file under a manifest: the boundary is the project dir, `src/` its source root, the layout is the same
    fs::write(tmp.path().join("wj.toml"), "[package]\nname = \"n\"\nversion = \"0.1.0\"\n").unwrap();
    assert_eq!(find_source_root(&file), Some(tmp.path().join("src").as_path()));
    // and a checkout named `src` that holds the manifest is never mistaken for a source dir
    let co = TempDir::new().unwrap();
    let checkout = co.path().join("src");
    let sub = checkout.join("work");
    fs::create_dir_all(&sub).unwrap();
    fs::write(checkout.join("wj.toml"), "[package]\nname = \"c\"\nversion = \"0.1.0\"\n").unwrap();
    let f2 = sub.join("prog.wj");
    fs::write(&f2, PROGRAM).unwrap();
    assert_eq!(find_source_root(&f2), Some(sub.as_path()));
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
