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

//! Sidecar hygiene (docs/metadata-data-model.md): analysis metadata lives in the cache, never next to a
//! source file. Three gates:
//!   1. no tracked `*.wj.meta` outside the allowlist (hand-authored, embedded `stdlib_meta/`);
//!   2. `.gitignore` covers `*.wj.meta` with a negation only for the allowlist;
//!   3. `wj build/check/run/test` on a temp project add no file next to any source, with or without a
//!      manifest; a legacy sidecar is ignored (never read) and `wj clean --sidecars` removes it.
//! Each detector is mutation-checked: a planted stray must be reported.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// Directories whose tracked `*.wj.meta` files are intentional (see the MODULE.md next to them).
const ALLOWLIST: &[&str] = &["stdlib_meta/"];

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn tracked_offenders(files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter(|f| f.ends_with(".wj.meta") && !ALLOWLIST.iter().any(|a| f.starts_with(a)))
        .cloned()
        .collect()
}

#[test]
fn no_tracked_wj_meta_outside_allowlist() {
    let o = Command::new("git").args(["ls-files"]).current_dir(repo()).output().unwrap();
    if !o.status.success() {
        return; // source tarball without git metadata
    }
    let files: Vec<String> = String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect();
    assert!(files.iter().any(|f| f.starts_with("stdlib_meta/")), "allowlisted stdlib_meta vanished: gate would be vacuous");
    let bad = tracked_offenders(&files);
    assert!(bad.is_empty(), "tracked sidecars (git rm them; they are build output):\n{}", bad.join("\n"));
}

#[test]
fn tracked_sidecar_detector_reports_a_planted_stray() {
    let planted = vec!["tests/fixtures/a.wj".to_string(), "tests/fixtures/a.wj.meta".to_string(), "stdlib_meta/vec.wj.meta".to_string()];
    assert_eq!(tracked_offenders(&planted), vec!["tests/fixtures/a.wj.meta".to_string()]);
}

#[test]
fn gitignore_covers_sidecars_except_stdlib_meta() {
    let ignored = |p: &str| Command::new("git").args(["check-ignore", "-q", "--no-index", p]).current_dir(repo()).status().map(|s| s.success()).unwrap_or(false);
    assert!(ignored("tests/fixtures/anything.wj.meta"), "*.wj.meta must be git-ignored");
    assert!(ignored("examples/x/main.wj.meta"));
    assert!(!ignored("stdlib_meta/vec.wj.meta"), "stdlib_meta is intentional and must stay trackable");
}

// ---- no stray files next to sources -------------------------------------------------------------------

/// Directories the compiler may create (output, caches); everything else under the project is a source tree.
const OUTPUT_DIRS: &[&str] = &["build", "out", "target", ".wj-cache"];

fn snapshot(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if OUTPUT_DIRS.contains(&name.as_str()) {
                    continue;
                }
                walk(&p, root, out);
            } else if name != ".wj-compiler-stamp" {
                out.insert(p.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    let mut s = BTreeSet::new();
    walk(root, root, &mut s);
    s
}

fn strays(before: &BTreeSet<PathBuf>, after: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    after.difference(before).cloned().collect()
}

const MAIN: &str = "use crate::util\n\nfn main() {\n    let n = util::inc(2)\n    println(\"${n}\")\n}\n\n@test\nfn t_inc() {\n    assert_eq(util::inc(1), 2)\n}\n";
const UTIL: &str = "pub fn inc(x: int) -> int {\n    x + 1\n}\n";

fn project(with_manifest: bool) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    if with_manifest {
        std::fs::write(d.path().join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2025\"\n").unwrap();
    }
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/main.wj"), MAIN).unwrap();
    std::fs::write(d.path().join("src/util.wj"), UTIL).unwrap();
    d
}

fn wj(dir: &Path, cache: &Path, args: &[&str]) -> std::process::Output {
    Command::new(WJ).args(args).current_dir(dir).env("TMPDIR", cache).output().unwrap()
}

fn assert_no_strays(with_manifest: bool) {
    let d = project(with_manifest);
    let cache = tempfile::tempdir().unwrap();
    let before = snapshot(d.path());
    let mut ran = 0;
    for args in [
        &["build", "src", "--no-cargo", "--no-lint", "--quiet", "-o", "out"][..],
        &["check", "src"][..],
        &["run", "--interpret", "src/main.wj"][..],
        &["test", "src", "--no-runtime-copy"][..],
    ] {
        let o = wj(d.path(), cache.path(), args);
        // A command that fails still must not litter; but at least build must succeed so the gate is not vacuous.
        if args[0] == "build" {
            assert!(o.status.success(), "build failed: {}", String::from_utf8_lossy(&o.stderr));
        }
        ran += 1;
        let after = snapshot(d.path());
        let s = strays(&before, &after);
        assert!(s.is_empty(), "`wj {}` left files next to sources (manifest={with_manifest}): {s:?}", args.join(" "));
    }
    assert_eq!(ran, 4);
}

#[test]
fn commands_leave_nothing_next_to_sources_with_manifest() {
    assert_no_strays(true);
}

#[test]
fn commands_leave_nothing_next_to_sources_without_manifest() {
    assert_no_strays(false);
}

#[test]
fn stray_detector_reports_a_planted_file() {
    let d = project(true);
    let before = snapshot(d.path());
    std::fs::write(d.path().join("src/util.wj.meta"), "{}").unwrap();
    std::fs::write(d.path().join("src/util.wji"), "").unwrap();
    let s = strays(&before, &snapshot(d.path()));
    assert_eq!(s.len(), 2, "{s:?}");
    // and files inside output dirs are not strays
    std::fs::create_dir_all(d.path().join("build")).unwrap();
    std::fs::write(d.path().join("build/x.rs"), "").unwrap();
    assert_eq!(strays(&before, &snapshot(d.path())).len(), 2);
}

// ---- migration ------------------------------------------------------------------------------------------

#[test]
fn legacy_sidecar_is_never_read() {
    // A poisoned sidecar next to a source must not change the output. The poison claims `util::label`
    // takes an owned string, which (if read) makes the call site emit `"abc".to_string()`; the real
    // analysis borrows. The oracle is the clean build, and the poison is proven live by the cache control below (single-file builds consult peer metadata).
    let build = |poison: Option<&Path>| {
        let d = project(true);
        std::fs::write(d.path().join("src/util.wj"), "pub fn label(s: string) -> int {\n    s.len()\n}\n").unwrap();
        std::fs::write(d.path().join("src/main.wj"), "use crate::util\n\nfn main() {\n    let n = util::label(\"abc\")\n    println(\"${n}\")\n}\n").unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut planted = None;
        if let Some(rel) = poison {
            let meta = r#"{"module_path":"util","functions":{"label":{"params":["String"],"return_type":"Int","is_associated":false,"parent_type":null,"param_ownership":["Owned"],"has_self_receiver":false}},"structs":{},"trait_impls":{},"copy_structs":[],"version":"0.0.0"}"#;
            let dest = d.path().join(rel);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::write(&dest, meta).unwrap();
            planted = Some(dest);
        }
        let o = wj(d.path(), cache.path(), &["build", "src/main.wj", "--no-cargo", "--no-lint", "--quiet", "-o", "out"]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let rs = std::fs::read_to_string(d.path().join("out/main.rs")).unwrap();
        let untouched = planted.map(|p| std::fs::read_to_string(p).unwrap().starts_with("{\"module_path\":\"util\"")).unwrap_or(true);
        (rs, untouched)
    };
    let (clean, _) = build(None);
    // Control: the same poison in a cache location IS read, so a differing result is detectable at all.
    let (cached, _) = build(Some(Path::new(".wj-cache/util.wj.meta")));
    assert_ne!(clean, cached, "poison is inert even in the cache; the gate would be vacuous");
    // The legacy location must behave exactly like no poison.
    let (legacy, untouched) = build(Some(Path::new("src/util.wj.meta")));
    assert_eq!(clean, legacy, "a sidecar next to a source influenced the build");
    assert!(untouched, "a legacy sidecar must be left alone by build, not rewritten");
}

#[test]
fn clean_sidecars_removes_legacy_files_and_only_those() {
    let d = project(true);
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("src/util.wj.meta"), "{}").unwrap();
    std::fs::create_dir_all(d.path().join("src/deep")).unwrap();
    std::fs::write(d.path().join("src/deep/a.wj.meta"), "{}").unwrap();
    std::fs::write(d.path().join("src/deep/a.wj"), "fn f() {}\n").unwrap();
    std::fs::write(d.path().join("src/notes.meta"), "keep").unwrap();
    let o = wj(d.path(), cache.path(), &["clean", "--sidecars"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(!d.path().join("src/util.wj.meta").exists());
    assert!(!d.path().join("src/deep/a.wj.meta").exists());
    assert!(d.path().join("src/notes.meta").exists(), "only *.wj.meta may be removed");
    assert!(d.path().join("src/util.wj").exists());
}

#[test]
fn doctor_mentions_legacy_sidecars() {
    let d = project(true);
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("src/util.wj.meta"), "{}").unwrap();
    let o = wj(d.path(), cache.path(), &["doctor"]);
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(text.contains("clean --sidecars"), "doctor should point at `wj clean --sidecars`:\n{text}");
}
