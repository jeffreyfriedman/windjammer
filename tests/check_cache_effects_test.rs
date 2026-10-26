//! The check caches never change effect diagnostics (WJ0040-WJ0049, ADR-0021): every `check-error` spec example
//! of `tests/spec/effects` with a WJ004x expectation (not an expected failure), as a project, gives the same diagnostics cached and uncached
//! (and the expected code); and edits that change those diagnostics without touching the checked file (the
//! manifest's grant, an effect moving into or out of a dependency module) are seen by the cached run.
//! Run: cargo test --release --test check_cache_effects

#[path = "check_cache/support.rs"]
mod support;

use std::path::Path;
use support::{compare, diags, in_dir, write};

fn effect_codes(r: &windjammer::diagnostics::Report) -> Vec<String> {
    let mut v: Vec<String> = r.diagnostics.iter().map(|d| d.code.to_string()).filter(|c| c.starts_with("WJ004")).collect();
    v.sort();
    v
}

/// A spec example as a project: its `// manifest:` lines under `[package]`, the program as `src/main.wj`.
fn spec_project(src: &str) -> tempfile::TempDir {
    let manifest: Vec<&str> = src.lines().filter_map(|l| l.strip_prefix("// manifest: ")).collect();
    let toml = format!("[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n{}\n", manifest.join("\n"));
    let d = tempfile::tempdir().unwrap();
    write(d.path(), &[("wj.toml".into(), toml), ("src/main.wj".into(), src.into())]);
    d
}

#[test]
fn effect_spec_examples_agree_cached_and_uncached() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec/effects");
    let mut checked = 0;
    let mut entries: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let src = std::fs::read_to_string(&p).unwrap();
        let Some(code) = src.lines().find_map(|l| l.strip_prefix("// expect-error: WJ004")).map(|c| format!("WJ004{}", &c[..1])) else { continue };
        if !src.contains("// mode: check-error") || src.contains("// xfail:") {
            continue;
        }
        let d = spec_project(&src);
        let main = d.path().join("src/main.wj");
        let got = in_dir(d.path(), || compare(&main, &p.display().to_string()).uncached);
        assert!(effect_codes(&got).contains(&code), "{}: expected {code}, got {:?}", p.display(), effect_codes(&got));
        checked += 1;
    }
    assert!(checked >= 6, "only {checked} effect examples checked");
}

const MAIN: &str = "use crate::dep\n\nfn main() {\n    println(\"${dep::size()}\")\n}\n";
const DEP_PURE: &str = "pub fn size() -> int {\n    3\n}\n";
const DEP_READS: &str = "use std::fs\n\npub fn size() -> int {\n    fs::read_to_string(\"data.txt\").unwrap_or(\"\").len()\n}\n";
const MAIN_PURE: &str = "use crate::dep\n\n@pure\nfn shown() -> int {\n    dep::size()\n}\n\nfn main() {\n    println(\"${shown()}\")\n}\n";

/// Grant and dependency edits change WJ0040 / WJ0042 for a file that never changes; each state agrees cached and
/// uncached, and the effect codes follow the edit.
#[test]
fn grant_and_dependency_edits_reach_cached_effect_diagnostics() {
    let d = tempfile::tempdir().unwrap();
    let toml = |caps: &str| format!("[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n[capabilities]\n{caps}");
    write(d.path(), &[("wj.toml".into(), toml("")), ("src/main.wj".into(), MAIN.into()), ("src/dep.wj".into(), DEP_PURE.into())]);
    let main = d.path().join("src/main.wj");
    in_dir(d.path(), || {
        let pure = compare(&main, "pure dependency").uncached;
        assert!(effect_codes(&pure).is_empty(), "{}", diags(&pure));
        write(d.path(), &[("src/dep.wj".into(), DEP_READS.into())]);
        let reads = compare(&main, "dependency reads a file, nothing granted").uncached;
        assert!(effect_codes(&reads).contains(&"WJ0040".to_string()), "{}", diags(&reads));
        write(d.path(), &[("wj.toml".into(), toml("fs.read = true\n"))]);
        let granted = compare(&main, "fs.read granted").uncached;
        assert!(!effect_codes(&granted).contains(&"WJ0040".to_string()), "{}", diags(&granted));
        write(d.path(), &[("src/main.wj".into(), MAIN_PURE.into())]);
        let undeclared = compare(&main, "@pure caller of a reading dependency").uncached;
        assert!(effect_codes(&undeclared).contains(&"WJ0042".to_string()), "{}", diags(&undeclared));
        write(d.path(), &[("src/dep.wj".into(), DEP_PURE.into())]);
        let fixed = compare(&main, "dependency pure again").uncached;
        assert!(effect_codes(&fixed).is_empty(), "{}", diags(&fixed));
    });
}
