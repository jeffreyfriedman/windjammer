#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! The browser playground (ADR-0034) runs this library compiled to wasm32-unknown-unknown, where
//! `Instant::now()` and `SystemTime::now()` panic ("time not implemented on this platform"). A clock read that
//! the interpreter or the checker reaches on that target breaks every playground run, and nothing but a wasm
//! build shows it. This gate makes each clock read in `src/` a decision: `tests/wasm_clock_sites.txt` lists, per
//! file, how many ungated reads it has (`FILE COUNT`), and the test fails when a file gains one, gains a first
//! one, or loses one without the list being lowered. A read is not counted when `cfg(not(target_arch =
//! "wasm32"))` (or `cfg!(...)`) appears on the same line or one of the two lines before it.
//!
//! A new read belongs behind a clock port (`interpreter::engine::Epoch`, the simulated world's clock), behind
//! that cfg, or in a module only the native CLI reaches; only then may the count in the list be raised. The
//! end-to-end check is `scripts/book/playground_check.wj` (the `Playground parity` step of book.yml).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn ungated_reads(text: &str) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let gated = |l: &str| l.contains("target_arch") && l.contains("wasm32");
    (0..lines.len())
        .filter(|&i| {
            let l = lines[i].trim_start();
            let code = !l.starts_with("//");
            code && (l.contains("Instant::now(") || l.contains("SystemTime::now("))
                && !(i.saturating_sub(2)..=i).any(|j| gated(lines[j]))
        })
        .count()
}

fn measured() -> BTreeMap<String, usize> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files
        .iter()
        .filter_map(|p| {
            let n = ungated_reads(&fs::read_to_string(p).ok()?);
            (n > 0).then(|| (p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), n))
        })
        .collect()
}

fn recorded() -> BTreeMap<String, usize> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/wasm_clock_sites.txt")).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (f, n) = l.split_once(' ').unwrap();
            (f.to_string(), n.trim().parse().unwrap())
        })
        .collect()
}

#[test]
fn clock_reads_in_src_match_the_recorded_list() {
    let (now, then) = (measured(), recorded());
    let mut problems = Vec::new();
    for (f, n) in &now {
        match then.get(f) {
            None => problems.push(format!("{f}: {n} new clock read(s); wasm32 panics on them (see the module docs)")),
            Some(m) if n > m => problems.push(format!("{f}: {n} clock reads, {m} recorded; gate the new one or use a clock port")),
            Some(m) if n < m => problems.push(format!("{f}: {n} clock reads, {m} recorded; lower the count in tests/wasm_clock_sites.txt")),
            _ => {}
        }
    }
    for f in then.keys().filter(|f| !now.contains_key(*f)) {
        problems.push(format!("{f}: no clock reads any more; delete its line from tests/wasm_clock_sites.txt"));
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn a_read_behind_the_wasm_cfg_is_not_counted_and_a_bare_one_is() {
    assert_eq!(ungated_reads("fn f() { let t = std::time::Instant::now(); }"), 1);
    assert_eq!(ungated_reads("#[cfg(not(target_arch = \"wasm32\"))]\nlet t = SystemTime::now();"), 0);
    assert_eq!(ungated_reads("// Instant::now() in a comment"), 0);
}
