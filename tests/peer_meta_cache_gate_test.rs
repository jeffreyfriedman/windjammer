//! A cached `.wj.meta` of another file is a hidden input unless it is proven current and reached: no cache entry may
//! change what a clean build emits. A build reads a peer entry only when it carries a fingerprint written by the same
//! compiler (same binary bytes), its source still exists with the text it was written for, and the build's sources
//! name the module (`use` / `mod`, closed over the reached modules' own declarations).
//!
//! The probe program calls `.len()` on a value whose type the compiler does not know; a peer declaring a free
//! function `len(..) -> int` used to turn the emitted `0` into `0_i64` once the peer had been built (or had left an
//! entry behind). Every case builds the probe into the same output directory and compares the emitted tree with a
//! clean build of the same project.
//!
//! The project is written by the test (not checked in as `.wj` files: the probe is an intentionally ill-typed program
//! that repository-wide scans must not pick up).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const MANIFEST: &str = "[package]\nname = \"peer_meta_cache\"\nversion = \"0.1.0\"\n";

const LAYOUT: &str = r#"// Probe: `cur` is declared with the wrong type, so `.len()` has no known receiver and its result width comes from
// whatever signature the build can see. A peer's unrelated free function `len` must not be one of them.
fn wrap(text: string, width: int) -> Vec<string> {
    let mut lines: Vec<string> = []
    let mut cur: int = ""
    for w in text.split_whitespace() {
        if cur.len() > 0 && (cur.len() + 1 + w.len()) as int > width {
            lines.push(cur)
            cur = w.to_string()
        } else if cur.len() == 0 {
            cur = w.to_string()
        } else {
            cur = cur + " " + w
        }
    }
    if cur.len() > 0 { lines.push(cur) }
    lines
}

fn main() {
    for l in wrap("alpha beta gamma delta epsilon zeta eta theta", 12) {
        println(l)
    }
}
"#;

const CHAIN: &str = r#"// Peer: declares a free function named like a method the probe calls.
enum Chain {
    Link(int, Box<Chain>),
    End,
}

fn len(c: Chain) -> int {
    match c {
        Chain::Link(_, rest) => 1 + len(*rest),
        Chain::End => 0,
    }
}

fn main() {
    println("${len(Chain::Link(1, Box::new(Chain::End)))}")
}
"#;

/// The project sources (relative path, text).
const SOURCES: &[(&str, &str)] = &[("src/layout.wj", LAYOUT), ("src/chain.wj", CHAIN)];

/// A fresh project (wj.toml, src/layout.wj, src/chain.wj) under `dir`.
fn project(dir: &Path) -> PathBuf {
    let p = dir.join("proj");
    std::fs::create_dir_all(p.join("src")).unwrap();
    std::fs::write(p.join("wj.toml"), MANIFEST).unwrap();
    for (rel, text) in SOURCES {
        std::fs::write(p.join(rel), text).unwrap();
    }
    p
}

fn build_with(wj: &Path, project: &Path, file: &str, cache_root: &Path) {
    let out = project.join("out");
    let _ = std::fs::remove_dir_all(&out);
    // manifest-less trees keep their entries in <cache root>/meta: each case has its own root
    let o = Command::new(wj)
        .env("WJ_CARGO_TARGET_ROOT", cache_root)
        .args(["build", file, "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .current_dir(project)
        .output()
        .expect("run wj build");
    assert!(project.join("out").exists(), "wj build {file} wrote nothing:\n{}", String::from_utf8_lossy(&o.stderr));
}

fn build(project: &Path, file: &str, cache_root: &Path) {
    build_with(Path::new(WJ), project, file, cache_root);
}

/// The emitted Rust of the last build (relative path -> text).
fn emitted(project: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, d: &Path, out: &mut BTreeMap<String, String>) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.insert(p.strip_prefix(base).unwrap().display().to_string(), std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    let mut m = BTreeMap::new();
    walk(&project.join("out"), &project.join("out"), &mut m);
    assert!(!m.is_empty(), "no Rust emitted");
    m
}

/// The probe's emitted Rust from a clean project and an empty cache.
fn clean_probe(dir: &Path) -> BTreeMap<String, String> {
    let p = project(&dir.join("clean"));
    let cache = dir.join("clean-cache");
    build(&p, "src/layout.wj", &cache);
    emitted(&p)
}

fn assert_same(case: &str, clean: &BTreeMap<String, String>, got: &BTreeMap<String, String>) {
    assert_eq!(clean, got, "{case}: a cache entry changed the emitted Rust");
}

/// The cached sidecar of `rel` (a `src/` path) in a project with a manifest.
fn meta_of(project: &Path, rel: &str) -> PathBuf {
    project.join(".wj-cache").join(format!("{rel}.meta"))
}

#[test]
fn an_unreached_peer_built_earlier_does_not_change_the_output() {
    // The tip reproduction: building the unrelated peer once used to change the probe's output.
    let d = tempfile::tempdir().unwrap();
    let clean = clean_probe(d.path());
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    build(&p, "src/chain.wj", &cache);
    assert!(meta_of(&p, "chain.wj").is_file(), "the peer build left no cache entry: the gate tests nothing");
    build(&p, "src/layout.wj", &cache);
    assert_same("unreached peer", &clean, &emitted(&p));
}

#[test]
fn a_peer_entry_whose_source_was_deleted_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    let clean = clean_probe(d.path());
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    build(&p, "src/chain.wj", &cache);
    std::fs::remove_file(p.join("src/chain.wj")).unwrap();
    build(&p, "src/layout.wj", &cache);
    assert_same("unrelated file (source gone)", &clean, &emitted(&p));
}

#[test]
fn a_peer_entry_with_a_wrong_source_hash_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    let clean = clean_probe(d.path());
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    build(&p, "src/chain.wj", &cache);
    let meta = meta_of(&p, "chain.wj");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    let h = v["analysis_fingerprint"]["content_hash"].as_u64().expect("the entry carries a source hash");
    v["analysis_fingerprint"]["content_hash"] = serde_json::json!(h ^ 1);
    std::fs::write(&meta, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    // reach it, so only the hash can reject it
    std::fs::write(p.join("src/layout.wj"), format!("use chain\n{}", std::fs::read_to_string(p.join("src/layout.wj")).unwrap())).unwrap();
    let reached_clean = {
        let q = project(&d.path().join("q"));
        std::fs::copy(p.join("src/layout.wj"), q.join("src/layout.wj")).unwrap();
        build(&q, "src/layout.wj", &d.path().join("q-cache"));
        emitted(&q)
    };
    build(&p, "src/layout.wj", &cache);
    assert_same("wrong source hash", &reached_clean, &emitted(&p));
    let _ = clean;
}

#[test]
fn a_peer_entry_from_another_compiler_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    let clean = clean_probe(d.path());
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    build(&p, "src/chain.wj", &cache);
    let meta = meta_of(&p, "chain.wj");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&meta).unwrap()).unwrap();
    v["analysis_fingerprint"]["compiler_version"] = serde_json::json!("0.0.0:another-build");
    std::fs::write(&meta, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    build(&p, "src/layout.wj", &cache);
    assert_same("other compiler", &clean, &emitted(&p));
}

#[test]
fn a_peer_entry_made_stale_by_a_source_edit_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    build(&p, "src/chain.wj", &cache);
    // the peer's `len` no longer exists, and the probe reaches the peer: only the stale entry could supply it
    let edited = std::fs::read_to_string(p.join("src/chain.wj")).unwrap().replace("len", "depth");
    std::fs::write(p.join("src/chain.wj"), &edited).unwrap();
    std::fs::write(p.join("src/layout.wj"), format!("use chain\n{}", std::fs::read_to_string(p.join("src/layout.wj")).unwrap())).unwrap();
    let q = project(&d.path().join("q"));
    std::fs::copy(p.join("src/chain.wj"), q.join("src/chain.wj")).unwrap();
    std::fs::copy(p.join("src/layout.wj"), q.join("src/layout.wj")).unwrap();
    build(&q, "src/layout.wj", &d.path().join("q-cache"));
    build(&p, "src/layout.wj", &cache);
    assert_same("stale after edit", &emitted(&q), &emitted(&p));
}

#[test]
fn a_manifest_less_tree_emits_the_same_in_two_locations() {
    // No manifest: entries live under <cache root>/meta keyed by the entry file's module closure (the files it can reach), never
    // by the path. Two entry files (chain.wj, layout.wj) give two entries; building both in a second location must add none.
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let mut outs = Vec::new();
    for at in ["first/place", "second"] {
        let tree = d.path().join(at).join("tree");
        std::fs::create_dir_all(&tree).unwrap();
        for (rel, text) in SOURCES {
            std::fs::write(tree.join(rel.trim_start_matches("src/")), text).unwrap();
        }
        build(&tree, "chain.wj", &cache);
        build(&tree, "layout.wj", &cache);
        outs.push(emitted(&tree));
    }
    assert_eq!(outs[0], outs[1], "the same tree emitted different Rust in two locations");
    let meta = cache.join("meta");
    let keys: Vec<_> = std::fs::read_dir(&meta).expect("entries under <cache root>/meta").flatten().collect();
    assert_eq!(keys.len(), 2, "one cache key per entry file's module closure, the same in every location (4 builds, 2 locations)");
    let clean = clean_probe(d.path());
    assert_eq!(outs[0], clean, "an unreached peer changed a manifest-less build");
}

#[test]
fn rebuilding_a_file_writes_identical_cache_bytes() {
    let d = tempfile::tempdir().unwrap();
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    build(&p, "src/chain.wj", &cache);
    let mut seen = Vec::new();
    for _ in 0..3 {
        build(&p, "src/layout.wj", &cache);
        seen.push(std::fs::read(meta_of(&p, "layout.wj")).expect("the build writes its cache entry"));
    }
    assert!(seen.windows(2).all(|w| w[0] == w[1]), "the cache entry bytes differ between identical builds");
}

#[test]
fn a_copied_compiler_accepts_the_entries_the_original_wrote() {
    // The compiler's identity is its build key (tests/build_content_key_test.rs), not its executable: a copy at another
    // path and mtime (`wj install-dev` re-copying an unchanged build) reads the entries the original wrote.
    let d = tempfile::tempdir().unwrap();
    let copy = d.path().join("bin/wj");
    std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
    std::fs::copy(WJ, &copy).unwrap();
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    std::fs::write(p.join("src/layout.wj"), format!("use chain\n{}", std::fs::read_to_string(p.join("src/layout.wj")).unwrap())).unwrap();
    build(&p, "src/chain.wj", &cache);
    let entry = std::fs::read(meta_of(&p, "chain.wj")).unwrap();
    build(&p, "src/layout.wj", &cache);
    let by_original = emitted(&p);
    build_with(&copy, &p, "src/layout.wj", &cache);
    assert_eq!(by_original, emitted(&p), "the copied compiler answered differently");
    assert_eq!(entry, std::fs::read(meta_of(&p, "chain.wj")).unwrap(), "the copy rewrote the peer's entry");
}

/// A REACHED peer (the probe does `use chain`) that declares a free `fn len`: the probe's `cur.len() > 0` is a method
/// call and never resolves to that free function, so the literal stays `0` (it was `0_i64` while the bare-name method
/// bug was open; the expected-failure ledger that pinned it is closed).
#[test]
fn reached_peer_free_function_is_not_the_probe_method() {
    let d = tempfile::tempdir().unwrap();
    let p = project(&d.path().join("p"));
    let cache = d.path().join("cache");
    std::fs::write(p.join("src/layout.wj"), format!("use chain\n{}", std::fs::read_to_string(p.join("src/layout.wj")).unwrap())).unwrap();
    build(&p, "src/chain.wj", &cache);
    build(&p, "src/layout.wj", &cache);
    let rust: String = emitted(&p).into_values().collect();
    assert!(rust.contains("cur.len() > 0") && !rust.contains("cur.len() > 0_i64"), "a method call resolved to the reached peer's free fn len:\n{rust}");
}
