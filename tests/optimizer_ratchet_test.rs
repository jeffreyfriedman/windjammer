//! Ratchet: the generated Rust of a fixed corpus must not get more wasteful.
//! Counts `.clone()`, `.to_string()`, ` as ` casts, `format!(` and `#[inline]` in the
//! output of `wj build` (which includes the optimizer) and compares with the
//! checked-in baseline. Lower the baseline when you improve; never raise it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/optimizer/corpus")
}

fn generate(wj: &Path, out: &Path) -> String {
    std::fs::create_dir_all(out).unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", wj.to_str().unwrap(), "--no-cargo", "--no-lint", "-q", "-o"])
        .arg(out)
        .output()
        .unwrap();
    assert!(r.status.success(), "wj build failed for {wj:?}: {}", String::from_utf8_lossy(&r.stderr));
    let stem = wj.file_stem().unwrap().to_string_lossy().to_string();
    std::fs::read_to_string(out.join(format!("{stem}.rs"))).unwrap()
}

fn count(code: &str) -> BTreeMap<&'static str, usize> {
    let mut m = BTreeMap::new();
    let code: String = code.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    m.insert("clone", code.matches(".clone()").count());
    m.insert("to_string", code.matches(".to_string()").count());
    m.insert("format", code.matches("format!(").count());
    m.insert("inline_attr", code.matches("#[inline]").count());
    let casts = code.split(" as ").count() - 1;
    m.insert("as_cast", casts);
    m
}

fn corpus_counts() -> BTreeMap<&'static str, usize> {
    let tmp = std::env::temp_dir().join(format!("wj-opt-ratchet-{}", std::process::id()));
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wj"))
        .collect();
    files.sort();
    let mut total = BTreeMap::new();
    for f in files {
        let code = generate(&f, &tmp.join(f.file_stem().unwrap()));
        for (k, v) in count(&code) {
            *total.entry(k).or_insert(0) += v;
        }
    }
    total
}

fn baseline() -> BTreeMap<String, usize> {
    let text = std::fs::read_to_string(corpus_dir().join("../ratchet_baseline.txt")).unwrap();
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().parse().unwrap()))
        .collect()
}

#[test]
fn optimizer_ratchet_waste_counts_do_not_increase() {
    let now = corpus_counts();
    let base = baseline();
    for (k, v) in &now {
        let limit = base.get(*k).unwrap_or_else(|| panic!("baseline missing key {k}"));
        assert!(v <= limit, "{k}: {v} > baseline {limit}; generated Rust got more wasteful (now: {now:?})");
    }
}
