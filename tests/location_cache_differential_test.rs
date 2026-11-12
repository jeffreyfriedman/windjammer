//! Output is a pure function of the sources and the compiler: the same project built in two locations, cold and
//! warm, with and without pre-existing caches or dependency sidecars (planted or stale), writes byte-identical
//! output and no location. Gate for the stale-sidecar finding (a dependency's `.wj-cache/*.wj.meta` widened a
//! `u32` formal to `i32`) and for output that depended on where the checkout lives.
//! Run: cargo test --test location_cache_differential

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LIB_TOML: &str = "[package]\nname = \"c\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n";
const MOD: &str = "pub mod text\npub mod util\n";
const TEXT: &str = "use std::strings\nuse crate::util::clean\n\npub fn strip_label(line: string) -> string {\n    let a = strings::replace(line, \"source=\", \"\")\n    let b = clean(a)\n    strings::replace(b, \"x\", \"\")\n}\n\npub fn count_up(limit: u32) -> u32 {\n    let mut k = 1\n    let mut total = 0\n    while k <= 32 {\n        total = total + bump(k, limit)\n        k = k + 1\n    }\n    total\n}\n\nfn bump(a: u32, b: u32) -> u32 {\n    a + b\n}\n";
const UTIL: &str = "use std::strings\n\npub fn clean(s: string) -> string {\n    strings::replace(s, \" \", \"\")\n}\n";

const DEP_MOD: &str = "pub mod table\n";
const DEP_TABLE: &str = "pub struct Entry {\n    pub id: u32,\n    pub leader: string,\n}\n\npub struct Table {\n    pub entries: Vec<Entry>,\n}\n\nimpl Table {\n    pub fn new() -> Table {\n        Table { entries: Vec::new() }\n    }\n\n    pub fn register(self, id: u32, leader: string) -> Table {\n        let mut entries = self.entries\n        entries.push(Entry { id: id, leader: leader })\n        Table { entries: entries }\n    }\n}\n";
const APP_M: &str = "use crate_a::Table\n\nfn build() -> Table {\n    let mut t = Table::new()\n    let mut k = 1\n    while k <= 32 {\n        t = t.register(k, \"leader:9000\")\n        k = k + 1\n    }\n    t\n}\n";

fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
}

fn wj(dir: &Path, args: &[&str]) {
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(dir)
        .args(["build"])
        .args(args)
        .args(["--library", "--module-file", "--no-cargo", "--no-generate-cargo-toml", "--no-lint"])
        .output()
        .expect("spawn wj");
    assert!(o.status.success(), "wj build {args:?} in {dir:?}:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

/// Every file under `dir` (relative path -> bytes as text), minus the compiler stamp.
fn tree(dir: &Path) -> BTreeMap<String, String> {
    fn go(d: &Path, base: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                go(&p, base, out);
            } else if p.file_name().is_some_and(|n| n != ".wj-compiler-stamp") {
                out.insert(p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"), fs::read_to_string(&p).unwrap_or_default());
            }
        }
    }
    let mut m = BTreeMap::new();
    go(dir, dir, &mut m);
    m
}

fn copy_dir(from: &Path, to: &Path, skip_caches: bool) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let (s, d) = (e.path(), to.join(e.file_name()));
        if skip_caches && (e.file_name() == ".wj-cache" || e.file_name() == "gen") {
            continue;
        }
        if s.is_dir() {
            copy_dir(&s, &d, skip_caches);
        } else {
            fs::copy(&s, &d).unwrap();
        }
    }
}

fn assert_same(label: &str, want: &BTreeMap<String, String>, got: &BTreeMap<String, String>, locations: &[&Path]) {
    assert_eq!(want.keys().collect::<Vec<_>>(), got.keys().collect::<Vec<_>>(), "{label}: file set differs");
    for (k, v) in want {
        assert_eq!(v, &got[k], "{label}: {k} differs");
    }
    for (k, v) in got {
        for loc in locations {
            let l = loc.to_string_lossy();
            assert!(!v.contains(&*l), "{label}: {k} embeds the checkout location {l}");
        }
    }
}

fn one_crate() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), &[("c/wj.toml", LIB_TOML), ("c/src/mod.wj", MOD), ("c/src/text.wj", TEXT), ("c/src/util.wj", UTIL)]);
    d
}

#[test]
fn a_crate_builds_identically_cold_warm_and_in_another_location() {
    let a = one_crate();
    let proj = a.path().join("c");
    wj(&proj, &["src", "-o", "out1"]);
    let cold = tree(&proj.join("out1"));
    assert!(proj.join(".wj-cache").is_dir(), "the build leaves its cache behind");
    // warm: every cache and sidecar the first build wrote is present
    wj(&proj, &["src", "-o", "out2"]);
    assert_same("warm, same location", &cold, &tree(&proj.join("out2")), &[a.path()]);
    // incremental: the same output directory again, with its previous output, stamp and metadata in place
    wj(&proj, &["src", "-o", "out1"]);
    assert_same("rebuilt in place", &cold, &tree(&proj.join("out1")), &[a.path()]);
    // another location, sources only
    let b = tempfile::tempdir().unwrap();
    copy_dir(&proj, &b.path().join("moved"), true);
    wj(&b.path().join("moved"), &["src", "-o", "out"]);
    assert_same("moved, cold", &cold, &tree(&b.path().join("moved/out")), &[a.path(), b.path()]);
    // another location with the caches copied along (their sidecars were written elsewhere)
    let c = tempfile::tempdir().unwrap();
    copy_dir(&proj, &c.path().join("moved"), false);
    let _ = fs::remove_dir_all(c.path().join("moved/out1"));
    let _ = fs::remove_dir_all(c.path().join("moved/out2"));
    wj(&c.path().join("moved"), &["src", "-o", "out"]);
    assert_same("moved, warm caches copied", &cold, &tree(&c.path().join("moved/out")), &[a.path(), c.path()]);
}

fn two_crates() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        &[
            ("crate_a/wj.toml", "[package]\nname = \"crate-a\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n"),
            ("crate_a/src/mod.wj", DEP_MOD),
            ("crate_a/src/table.wj", DEP_TABLE),
            ("crate_b/src/m.wj", APP_M),
        ],
    );
    d
}

/// Build the dependency, then the dependent against its `gen/metadata.json`; returns the dependent's output.
fn build_pair(root: &Path, out: &str) -> BTreeMap<String, String> {
    if !root.join("crate_a/gen/metadata.json").is_file() {
        wj(&root.join("crate_a"), &["src", "-o", "gen"]);
    }
    let meta = format!("crate_a={}", root.join("crate_a/gen/metadata.json").display());
    wj(root, &["crate_b/src", "-o", out, "--metadata", &meta]);
    tree(&root.join(out))
}

#[test]
fn a_dependency_sidecar_never_changes_the_dependent_output() {
    let a = two_crates();
    let root = a.path();
    let with_sidecar = build_pair(root, "b_with");
    assert!(root.join("crate_a/.wj-cache").is_dir(), "the dependency build wrote sidecars");
    assert!(with_sidecar["m.rs"].contains("k <= 32_u32"), "the counter takes the declared u32 formal:\n{}", with_sidecar["m.rs"]);
    // no sidecars at all
    fs::remove_dir_all(root.join("crate_a/.wj-cache")).unwrap();
    assert_same("dependency without sidecars", &with_sidecar, &build_pair(root, "b_without"), &[root]);
    // a stale sidecar: the dependency source changed after the sidecar was written
    wj(&root.join("crate_a"), &["src", "-o", "gen"]);
    let table = root.join("crate_a/src/table.wj");
    let src = fs::read_to_string(&table).unwrap();
    fs::write(&table, format!("{src}\n// edited after the sidecar was written\n")).unwrap();
    assert_same("dependency with a stale sidecar", &with_sidecar, &build_pair(root, "b_stale"), &[root]);
    // a sidecar planted with a different signature than the declared crate metadata: the declaration wins
    let sidecar = root.join("crate_a/.wj-cache/table.wj.meta");
    let text = fs::read_to_string(&sidecar).unwrap().replace("Custom(\\\"u32\\\")", "Custom(\\\"i32\\\")");
    fs::write(&sidecar, text).unwrap();
    assert_same("dependency with a contradicting sidecar", &with_sidecar, &build_pair(root, "b_planted"), &[root]);
    // another location
    let moved = tempfile::tempdir().unwrap();
    copy_dir(root, &moved.path().join("w"), true);
    let got = build_pair(&moved.path().join("w"), "b_moved");
    assert_same("moved", &with_sidecar, &got, &[root, moved.path()]);
}
