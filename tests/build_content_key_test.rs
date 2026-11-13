//! The compiler's identity is a build key over the content of every build input (build_support/content_key.rs),
//! baked in by build.rs: location-free, clock-free, git-free, and a constant at run time.
//!
//! Gates: two copies of the input tree at different paths have the same key; touching files changes nothing; a
//! content change to any input class (sources, std, the lock file, the build script) or to the profile, target,
//! toolchain or features changes the key; the run-time identity reads no executable path or mtime; a cache entry
//! written under another key is ignored, never an error.

#[path = "../build_support/content_key.rs"]
mod content_key;

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A small tree with one file in each input class.
fn tree(at: &Path) -> PathBuf {
    for (rel, text) in [
        ("src/lib.rs", "pub fn f() {}\n"),
        ("src/deep/mod.rs", "pub fn g() {}\n"),
        ("std/fs.wj", "pub fn read() {}\n"),
        ("stdlib_meta/string.wj.meta", "{}\n"),
        ("crates/windjammer-runtime/Cargo.toml", "[package]\nname = \"rt\"\n"),
        ("crates/windjammer-runtime/src/lib.rs", "pub fn rt() {}\n"),
        ("Cargo.toml", "[package]\nname = \"x\"\n"),
        ("Cargo.lock", "version = 4\n"),
        ("build.rs", "fn main() {}\n"),
        ("build_support/k.rs", "pub fn k() {}\n"),
        ("tests/not_an_input.rs", "fn t() {}\n"),
        ("README.md", "not an input\n"),
    ] {
        let p = at.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    at.to_path_buf()
}

fn key(t: &Path) -> String {
    content_key::content_key(t, "rustc 1.0.0\nhost: x\n", "x86_64-unknown-linux-gnu", "release", &["CLI".into(), "JIT".into()])
}

#[test]
fn the_same_inputs_at_two_paths_have_one_key() {
    let d = tempfile::tempdir().unwrap();
    let a = tree(&d.path().join("checkout-a"));
    let b = tree(&d.path().join("elsewhere/checkout-b"));
    assert_eq!(key(&a), key(&b));
    assert_eq!(content_key::input_files(&a).len(), 10, "every input class is hashed, nothing else");
}

#[test]
fn touching_inputs_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let t = tree(d.path());
    let before = key(&t);
    for f in content_key::input_files(&t) {
        let file = std::fs::File::options().write(true).open(t.join(&f)).unwrap();
        file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(3600)).unwrap();
    }
    assert_eq!(before, key(&t));
}

#[test]
fn a_content_change_to_any_input_changes_the_key() {
    let d = tempfile::tempdir().unwrap();
    let t = tree(d.path());
    let base = key(&t);
    for f in content_key::input_files(&t) {
        let p = t.join(&f);
        let old = std::fs::read(&p).unwrap();
        std::fs::write(&p, [old.as_slice(), b"//"].concat()).unwrap();
        assert_ne!(base, key(&t), "editing {f} kept the key");
        std::fs::write(&p, &old).unwrap();
    }
    assert_eq!(base, key(&t), "restoring the inputs restores the key");
    std::fs::write(t.join("README.md"), "changed\n").unwrap();
    std::fs::write(t.join("tests/not_an_input.rs"), "changed\n").unwrap();
    assert_eq!(base, key(&t), "files that are not build inputs must not enter the key");
    std::fs::write(t.join("src/new_module.rs"), "pub fn h() {}\n").unwrap();
    assert_ne!(base, key(&t), "a new source file changes the key");
    std::fs::remove_file(t.join("src/new_module.rs")).unwrap();
    let k = |rustc: &str, target: &str, profile: &str, features: &[&str]| {
        content_key::content_key(&t, rustc, target, profile, &features.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    };
    let reference = k("rustc 1.0.0\nhost: x\n", "x86_64-unknown-linux-gnu", "release", &["CLI", "JIT"]);
    assert_eq!(base, reference);
    assert_eq!(reference, k("rustc 1.0.0\nhost: x\n", "x86_64-unknown-linux-gnu", "release", &["JIT", "CLI"]), "feature order");
    assert_ne!(reference, k("rustc 1.0.1\nhost: x\n", "x86_64-unknown-linux-gnu", "release", &["CLI", "JIT"]), "toolchain");
    assert_ne!(reference, k("rustc 1.0.0\nhost: x\n", "aarch64-apple-darwin", "release", &["CLI", "JIT"]), "target");
    assert_ne!(reference, k("rustc 1.0.0\nhost: x\n", "x86_64-unknown-linux-gnu", "debug", &["CLI", "JIT"]), "profile");
    assert_ne!(reference, k("rustc 1.0.0\nhost: x\n", "x86_64-unknown-linux-gnu", "release", &["CLI"]), "features");
}

#[test]
fn build_rs_watches_exactly_the_hashed_inputs() {
    let build = std::fs::read_to_string(root().join("build.rs")).unwrap();
    assert!(build.contains("for rel in content_key::INPUTS {") && build.contains("cargo:rerun-if-changed={rel}"));
    for rel in content_key::INPUTS {
        assert!(root().join(rel).exists(), "{rel} is listed as a build input but does not exist");
    }
}

#[test]
fn the_run_time_identity_is_a_constant_of_the_build() {
    // Nothing to invalidate when `wj install-dev` re-copies an unchanged binary: the identity is baked in, not read
    // from the executable.
    let key = windjammer::check::shared_cache::compiler_key().expect("this build carries a build key");
    assert_eq!(key, windjammer::compiler::incremental::compiler_build_identity(), "one identity for every cache");
    let src = std::fs::read_to_string(root().join("src/check/shared_cache.rs")).unwrap();
    let body = &src[src.find("pub fn compiler_key()").unwrap()..];
    let body = &body[..body.find("\n}\n").unwrap()];
    for forbidden in ["current_exe", "metadata(", "modified", "SystemTime", "git"] {
        assert!(!body.contains(forbidden), "compiler_key reads `{forbidden}`: the identity must be a build constant");
    }
    let fp = std::fs::read_to_string(root().join("src/compiler/incremental/build_fingerprint.rs")).unwrap();
    let body = &fp[fp.find("pub fn compiler_build_identity()").unwrap()..];
    let body = &body[..body.find("\n}\n").unwrap()];
    assert!(body.contains("compiler_key()") && !body.contains("current_exe"), "compiler_build_identity must delegate");
}

#[test]
fn cache_entries_of_another_key_are_ignored_not_errors() {
    // Plant entries that no build key names (garbage and another build's tables) in the shared cache dirs; a check
    // must succeed exactly as with an empty cache.
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("cache");
    let proj = d.path().join("p");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join("main.wj"), "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    let run = || Command::new(WJ).env("WJ_CARGO_TARGET_ROOT", &cache).args(["check", "main.wj"]).current_dir(&proj).output().unwrap();
    let clean = run();
    assert!(clean.status.success(), "{}", String::from_utf8_lossy(&clean.stderr));
    for dir in ["registry", "analysis"] {
        let p = cache.join(dir);
        std::fs::create_dir_all(&p).unwrap();
        for e in std::fs::read_dir(&p).unwrap().flatten() {
            std::fs::write(e.path(), "{\"not\": \"this build\"").unwrap();
        }
        std::fs::write(p.join(format!("{}.json", "a".repeat(64))), "garbage").unwrap();
    }
    let planted = run();
    assert!(planted.status.success(), "{}", String::from_utf8_lossy(&planted.stderr));
    // the summary line carries a timing (and a cache note): compare the verdict
    let verdict = |o: &std::process::Output| String::from_utf8_lossy(&o.stdout).split(" (").next().unwrap_or_default().to_string();
    assert_eq!(verdict(&clean), verdict(&planted));
}
