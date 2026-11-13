//! Whether a library build writes a crate-root `lib.rs` is decided by the project layout (a manifest's `src/`, an
//! enclosing `mod.wj` / `mod.rs` module tree), never by what the output directory is called. The same crate built into
//! `out/`, `x/src/y/` and `src/` writes identical results when nothing in the layout says otherwise; a manifest or a
//! module tree around the output is the documented, only reason for a difference.
//! Run: cargo test --test all -- output_layout_location_free

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

const MOD: &str = "pub mod util\n";
const UTIL: &str = "pub fn double(x: int) -> int {\n    x * 2\n}\n";

fn build(src: &Path, out: &Path, cwd: &Path) {
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(cwd)
        .args(["build"])
        .arg(src)
        .args(["--library", "--module-file", "--no-cargo", "--no-generate-cargo-toml", "--no-lint", "-o"])
        .arg(out)
        .output()
        .expect("spawn wj");
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

fn files(dir: &Path) -> BTreeSet<String> {
    fn go(d: &Path, base: &Path, out: &mut BTreeSet<String>) {
        for e in fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                go(&p, base, out);
            } else if p.file_name().is_some_and(|n| n != ".wj-compiler-stamp") {
                out.insert(p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut s = BTreeSet::new();
    go(dir, dir, &mut s);
    s
}

fn source(root: &Path) -> std::path::PathBuf {
    let s = root.join("wjsrc");
    fs::create_dir_all(&s).unwrap();
    fs::write(s.join("mod.wj"), MOD).unwrap();
    fs::write(s.join("util.wj"), UTIL).unwrap();
    s
}

#[test]
fn output_directory_names_do_not_change_the_result() {
    let t = tempfile::tempdir().unwrap();
    let src = source(t.path());
    let names = ["out", "x/src/y", "src"];
    let mut results = Vec::new();
    for n in names {
        let out = t.path().join(n);
        build(&src, &out, t.path());
        results.push((n, files(&out)));
    }
    for (n, f) in &results {
        assert_eq!(f, &results[0].1, "`{n}` differs from `{}`", results[0].0);
    }
    assert!(results[0].1.contains("lib.rs"), "a crate-root output has lib.rs: {:?}", results[0].1);
}

#[test]
fn a_manifest_src_dir_or_a_module_tree_around_the_output_is_what_makes_it_a_submodule() {
    // manifest: <project>/src/<x>/ is inside the crate's source directory
    let t = tempfile::tempdir().unwrap();
    let src = source(t.path());
    fs::create_dir_all(t.path().join("proj/src")).unwrap();
    fs::write(t.path().join("proj/wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\n").unwrap();
    build(&src, &t.path().join("proj/src/gen"), t.path());
    assert!(!files(&t.path().join("proj/src/gen")).contains("lib.rs"), "inside the manifest's src/: a submodule, no crate-root lib.rs");
    build(&src, &t.path().join("proj/out"), t.path());
    assert!(files(&t.path().join("proj/out")).contains("lib.rs"), "beside src/, under the same manifest: a crate root");
    // module tree: an enclosing directory that is a module (holds mod.wj)
    let t2 = tempfile::tempdir().unwrap();
    let src2 = source(t2.path());
    fs::create_dir_all(t2.path().join("tree/components")).unwrap();
    fs::write(t2.path().join("tree/components/mod.wj"), "").unwrap();
    build(&src2, &t2.path().join("tree/components/generated"), t2.path());
    assert!(!files(&t2.path().join("tree/components/generated")).contains("lib.rs"));
}
