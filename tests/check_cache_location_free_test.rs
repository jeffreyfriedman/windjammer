//! The check caches under `.wj-cache` hold no absolute path: stored text names the project root only as a
//! placeholder, so a copied or moved checkout carries no foreign location and keeps its cache hits.
//! Run: cargo test --test check_cache_location_free

#[path = "common/check_cache/support.rs"]
mod support;

use std::path::Path;
use support::{in_dir, report, write};

const TOML: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\n";
// `dep::f` takes one argument: the call below yields a diagnostic whose span names the file
const MAIN: &str = "use crate::dep\n\nfn main() {\n    let x = dep::f(1, 2)\n    println(\"${x}\")\n}\n";
const DEP: &str = "pub fn f(x: int) -> int {\n    x + 1\n}\n";

fn project() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        &[("wj.toml", TOML), ("src/main.wj", MAIN), ("src/dep.wj", DEP)].map(|(a, b)| (a.to_string(), b.to_string())),
    );
    d
}

fn cache_texts(root: &Path) -> Vec<(String, String)> {
    std::fs::read_dir(root.join(".wj-cache"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read_to_string(e.path()).unwrap()))
        .collect()
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

fn spellings(root: &Path) -> Vec<String> {
    let mut v = vec![root.display().to_string()];
    if let Ok(c) = std::fs::canonicalize(root) {
        v.push(c.display().to_string());
    }
    v
}

#[test]
fn stored_cache_files_hold_no_absolute_path() {
    let d = project();
    let main = d.path().join("src/main.wj");
    in_dir(d.path(), || {
        let first = report(&main, true);
        assert!(!first.diagnostics.is_empty(), "the fixture must produce a diagnostic with a span");
        let files = cache_texts(d.path());
        assert!(files.iter().any(|(n, _)| n.starts_with("check-files-")), "memo written: {files:?}");
        assert!(files.iter().any(|(n, _)| n.starts_with("check-run-")), "run cache written: {files:?}");
        for (name, text) in &files {
            for sp in spellings(d.path()) {
                assert!(!text.contains(&sp), "{name} embeds the checkout location {sp}");
            }
            assert!(text.contains("{PROJECT_ROOT}"), "{name} names files relative to the project root");
        }
    });
}

#[test]
fn a_copied_checkout_keeps_its_cache_hits_and_reports_its_own_paths() {
    let a = project();
    in_dir(a.path(), || report(&a.path().join("src/main.wj"), true));
    let b = tempfile::tempdir().unwrap();
    copy_dir(a.path(), b.path());
    let main_b = b.path().join("src/main.wj");
    in_dir(b.path(), || {
        let r = report(&main_b, true);
        assert!(r.summary.files_cached > 0, "the copy answers from the copied cache");
        let uncached = report(&main_b, false);
        assert_eq!(serde_json::to_string(&r.diagnostics).unwrap(), serde_json::to_string(&uncached.diagnostics).unwrap());
        let span_files: Vec<String> = r.diagnostics.iter().filter_map(|d| d.primary_span.as_ref().map(|s| s.file.clone())).collect();
        assert!(!span_files.is_empty());
        for f in span_files {
            let copy = spellings(b.path());
            assert!(copy.iter().any(|c| f.starts_with(c.as_str())), "{f} points into the copy, not the original");
        }
    });
}
