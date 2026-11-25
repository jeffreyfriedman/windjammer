//! A manifest-less program keeps one cache entry per module file, keyed by what THAT file depends on. Regenerating one
//! file (even after editing it) must still find the entries its peers wrote; changing a file must invalidate the
//! entries that depend on it and nothing else. Checked on the cache locations themselves (no code generation).

use std::path::{Path, PathBuf};
use windjammer::metadata::{meta_cache_path, peer_meta_entries};

/// `app.wj` (the entry) uses `geo`; `geo.wj` uses `util`; `side.wj` is unrelated to all of them.
fn tree(dir: &Path) -> PathBuf {
    let t = dir.join("tree");
    std::fs::create_dir_all(&t).unwrap();
    std::fs::write(t.join("app.wj"), "use geo\nfn main() {\n    println(\"a\")\n}\n").unwrap();
    std::fs::write(t.join("geo.wj"), "use util\npub fn area() -> int {\n    1\n}\n").unwrap();
    std::fs::write(t.join("util.wj"), "pub fn one() -> int {\n    1\n}\n").unwrap();
    std::fs::write(t.join("side.wj"), "pub fn side() -> int {\n    2\n}\n").unwrap();
    t
}

fn entry(t: &Path, name: &str, cache: &Path) -> PathBuf {
    meta_cache_path(&t.join(name), cache)
}

fn append(file: &Path, text: &str) {
    let mut s = std::fs::read_to_string(file).unwrap();
    s.push_str(text);
    std::fs::write(file, s).unwrap();
}

#[test]
fn touching_the_entry_keeps_the_peers_entries_where_they_were() {
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let t = tree(d.path());
    let before: Vec<_> = ["geo.wj", "util.wj", "side.wj"].iter().map(|n| entry(&t, n, &cache)).collect();
    for p in &before {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "{}").unwrap();
    }
    append(&t.join("app.wj"), "// touch\n");
    let after: Vec<_> = ["geo.wj", "util.wj", "side.wj"].iter().map(|n| entry(&t, n, &cache)).collect();
    assert_eq!(before, after, "editing the entry file moved its peers' cache entries");
    let found = peer_meta_entries(&t, &cache);
    for p in &before {
        assert!(found.contains(p), "peer entry {} is not found from the program after the entry was edited: {found:?}", p.display());
    }
}

#[test]
fn changing_a_peer_invalidates_what_depends_on_it_and_only_that() {
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let t = tree(d.path());
    let names = ["app.wj", "geo.wj", "util.wj", "side.wj"];
    let before: Vec<_> = names.iter().map(|n| entry(&t, n, &cache)).collect();
    append(&t.join("util.wj"), "pub fn two() -> int {\n    2\n}\n");
    let after: Vec<_> = names.iter().map(|n| entry(&t, n, &cache)).collect();
    assert_ne!(before[0], after[0], "app depends on util through geo: its entry must be a new one");
    assert_ne!(before[1], after[1], "geo uses util: its entry must be a new one");
    assert_ne!(before[2], after[2], "util itself changed");
    assert_eq!(before[3], after[3], "an unrelated sibling's entry must not be invalidated");
}

#[test]
fn the_same_tree_in_two_locations_shares_its_entries() {
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let a = tree(&d.path().join("one/place"));
    let b = tree(&d.path().join("two"));
    for n in ["app.wj", "geo.wj", "util.wj", "side.wj"] {
        assert_eq!(entry(&a, n, &cache), entry(&b, n, &cache), "{n}: the entry depends on where the tree lives");
    }
}

#[test]
fn a_module_reached_by_use_does_not_depend_on_its_parents_other_children() {
    // graph/mod.wj declares both children; `a` uses the parent module (to name `b`'s sibling re-export) and so reaches
    // mod.wj, but editing the child `c` must not move `a`'s entry. The parent declaring `c` does depend on it.
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let t = d.path().join("src");
    std::fs::create_dir_all(t.join("graph")).unwrap();
    std::fs::write(t.join("mod.wj"), "pub mod graph\n").unwrap();
    std::fs::write(t.join("graph/mod.wj"), "pub mod a\npub mod c\npub use a::A\n").unwrap();
    std::fs::write(t.join("graph/a.wj"), "use crate::graph::A\npub struct A {\n    pub x: int,\n}\n").unwrap();
    std::fs::write(t.join("graph/c.wj"), "pub fn c() -> int {\n    3\n}\n").unwrap();
    let names = ["graph/a.wj", "graph/c.wj", "graph/mod.wj"];
    let before: Vec<_> = names.iter().map(|n| entry(&t, n, &cache)).collect();
    append(&t.join("graph/c.wj"), "// touch\n");
    let after: Vec<_> = names.iter().map(|n| entry(&t, n, &cache)).collect();
    assert_eq!(before[0], after[0], "a sibling child's edit moved a module that only uses the parent");
    assert_ne!(before[1], after[1]);
    assert_ne!(before[2], after[2], "the parent declares the edited child: its entry must be a new one");
}

const CHILD: &str = "WJ_PROJECTLESS_PEERS_CHILD";

/// The CLI registers the program entry (`set_program_entry`) once per process, which is process-global state: this test
/// re-runs itself in a fresh process (the helper below) so no other test sees the registration.
#[test]
fn touching_the_registered_program_entry_keeps_the_peers_entries() {
    let exe = std::env::current_exe().unwrap();
    let o = std::process::Command::new(exe)
        .args(["--exact", "projectless_peers_test::registered_entry_child", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(o.status.success(), "child failed:\n{}\n{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stdout).contains("child ran"), "the helper test did not run");
}

#[test]
fn registered_entry_child() {
    if std::env::var(CHILD).is_err() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let t = tree(d.path());
    windjammer::metadata::set_program_entry(&t.join("app.wj"));
    let before: Vec<_> = ["geo.wj", "util.wj"].iter().map(|n| entry(&t, n, &cache)).collect();
    append(&t.join("app.wj"), "// touch\n");
    let after: Vec<_> = ["geo.wj", "util.wj"].iter().map(|n| entry(&t, n, &cache)).collect();
    assert_eq!(before, after, "editing the registered entry moved its peers' entries");
    println!("child ran");
}
