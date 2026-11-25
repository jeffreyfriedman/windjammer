//! The literal widths shared between the tests of one `wj test --interpret` run
//! (`docs/design/interp-soundness/widthshare.md`): a shared answer equals a fresh analysis, a changed file is
//! analysed afresh, and tests of one module analyse it once.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use windjammer::interpreter::compiled_facts::{widths_text_of_source, WidthShare};
use windjammer::interpreter::runner::{run_specs_shared, TestResult};
use windjammer::test_runner::interpreted::discover_specs;

const LIB: &str = "pub fn total(v: Vec<i64>) -> i64 {\n    let mut i = 0\n    let mut t = 0\n    while i < v.len() {\n        t += v[i]\n        i += 1\n    }\n    t\n}\n";

fn test_file(first: i64, count: usize) -> String {
    let mut s = String::from("use crate::total\n\n");
    for n in 0..count {
        s.push_str(&format!("fn test_total_{n}() {{\n    let mut k = 0\n    k += {first}\n    assert_eq(total(vec![k, {n}]), {})\n}}\n\n", first + n as i64));
    }
    s
}

fn project(name: &str, tests: &[(&str, String)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wj-widthshare-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(dir.join("wj.toml"), "[package]\nname = \"ws\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n").unwrap();
    std::fs::write(dir.join("src/lib.wj"), LIB).unwrap();
    for (n, text) in tests {
        std::fs::write(dir.join("tests").join(n), text).unwrap();
    }
    dir
}

fn run(dir: &Path, share: Option<Arc<WidthShare>>) -> Vec<(String, bool, String)> {
    let specs = discover_specs(dir).unwrap();
    assert!(!specs.is_empty());
    let r: Vec<TestResult> = run_specs_shared(specs, None, None, share);
    let mut v: Vec<_> = r.into_iter().map(|r| (r.name, r.passed, scrub_scratch(&r.message))).collect();
    v.sort();
    v
}

/// A message names the test's scratch directory (`wj-scratch-<pid>-<random>`): the random part differs per run.
fn scrub_scratch(m: &str) -> String {
    let mut out = String::new();
    let mut rest = m;
    while let Some(i) = rest.find("wj-scratch-") {
        out.push_str(&rest[..i + "wj-scratch-".len()]);
        rest = &rest[i + "wj-scratch-".len()..];
        let n = rest.find(|c: char| !(c.is_ascii_hexdigit() || c == '-')).unwrap_or(rest.len());
        rest = &rest[n..];
    }
    out + rest
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (p, q) = (e.path(), to.join(e.file_name()));
        if e.file_name() == ".wj-cache" {
            continue;
        }
        if p.is_dir() {
            copy_dir(&p, &q);
        } else {
            std::fs::copy(&p, &q).unwrap();
        }
    }
}

#[test]
fn two_tests_of_one_module_analyse_it_once() {
    let dir = project("once", &[("a_test.wj", test_file(5, 4))]);
    let share = Arc::new(WidthShare::new());
    let shared = run(&dir, Some(share.clone()));
    assert!(shared.iter().all(|r| r.1), "{shared:?}");
    let (hits, misses) = share.counts();
    // two files evaluate literals (the library and the test file): one analysis each, however many tests
    assert_eq!(misses, 2, "one analysis per distinct program text");
    assert_eq!(hits, 2 * (4 - 1), "every later test hits");
    assert_eq!(share.mismatches(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_changed_test_file_recomputes_only_its_own_text() {
    let dir = project("change", &[("a_test.wj", test_file(5, 2)), ("b_test.wj", test_file(7, 2))]);
    let share = Arc::new(WidthShare::verifying());
    let first = run(&dir, Some(share.clone()));
    let (_, m1) = share.counts();
    assert_eq!(m1, 3, "lib, a_test, b_test");
    // an unchanged second run recomputes nothing
    assert_eq!(run(&dir, Some(share.clone())), first);
    assert_eq!(share.counts().1, 3);
    // one test file changes: its text is new, its dependents (itself) recompute; the library and b_test hit
    std::fs::write(dir.join("tests/a_test.wj"), test_file(9, 3)).unwrap();
    let after = run(&dir, Some(share.clone()));
    assert_eq!(share.counts().1, 4, "only the changed file's text is a miss");
    assert_eq!(after, run(&dir, None), "the result with the share equals a fresh run");
    assert_eq!(share.mismatches(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_shared_answer_equals_a_fresh_one_after_an_edit() {
    let share = WidthShare::verifying();
    let narrow = "pub fn f(v: Vec<i64>) -> i64 {\n    let mut i = 0\n    while i < v.len() {\n        i += 1\n    }\n    7\n}\n";
    let other = "pub fn f(v: Vec<i64>) -> i64 {\n    let mut i = 0\n    let mut j = 3\n    while i < v.len() {\n        i += 1\n        j += 1\n    }\n    v[j] + 7\n}\n";
    let none = HashMap::new();
    let a = widths_text_of_source(Some(&share), narrow, "same.wj", &none).unwrap();
    let b = widths_text_of_source(Some(&share), other, "same.wj", &none).unwrap();
    assert_ne!(a, b, "the corpus needs two different answers under one file name");
    assert_eq!(a, widths_text_of_source(None, narrow, "same.wj", &none).unwrap());
    assert_eq!(b, widths_text_of_source(None, other, "same.wj", &none).unwrap());
    // back to the first text: a hit, equal to the first answer
    assert_eq!(widths_text_of_source(Some(&share), narrow, "same.wj", &none).unwrap(), a);
    assert_eq!(share.counts(), (1, 2));
    assert_eq!(share.mismatches(), 0);
}

#[test]
fn renames_are_part_of_the_key() {
    let share = WidthShare::verifying();
    let src = "pub fn f(v: Vec<i64>) -> i64 {\n    let mut i = 0\n    while i < v.size() {\n        i += 1\n    }\n    7\n}\n";
    let plain = HashMap::new();
    let renamed: HashMap<String, String> = [("size".to_string(), "len".to_string())].into();
    let a = widths_text_of_source(Some(&share), src, "r.wj", &plain).unwrap();
    let b = widths_text_of_source(Some(&share), src, "r.wj", &renamed).unwrap();
    assert_ne!(a, b, "the rename turns `size()` into `len()` and `i` into a usize");
    assert_eq!(b, widths_text_of_source(None, src, "r.wj", &renamed).unwrap());
    assert_eq!(share.counts(), (0, 2));
}

#[test]
fn the_repo_programs_agree_with_a_fresh_analysis() {
    let share = WidthShare::verifying();
    let mut n = 0;
    for dir in ["tests/engine_parity", "tests/interpreter_programs"] {
        let Ok(rd) = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(dir)) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "wj") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&p) else { continue };
            let file = p.to_string_lossy().to_string();
            let none = HashMap::new();
            let (Some(first), Some(second)) = (widths_text_of_source(Some(&share), &src, &file, &none), widths_text_of_source(Some(&share), &src, &file, &none)) else { continue };
            assert_eq!(first, second);
            assert_eq!(first, widths_text_of_source(None, &src, &file, &none).unwrap(), "{file}");
            n += 1;
        }
    }
    assert!(n > 20, "the corpus is the repo's programs ({n})");
    assert_eq!(share.mismatches(), 0);
    assert_eq!(share.counts().0, n as u64);
}

#[test]
fn ecosystem_packages_give_identical_results_with_and_without_the_share() {
    let root = std::env::var_os("WJ_ECOSYSTEM").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("src/windjammer-ecosystem")));
    let Some(root) = root.map(|r| r.join("packages")).filter(|r| r.is_dir()) else { return };
    let mut ran = 0;
    for pkg in ["wj-semver", "wj-decimal", "wj-fuzzy"] {
        let src = root.join(pkg);
        if !src.is_dir() {
            continue;
        }
        let dir = std::env::temp_dir().join(format!("wj-widthshare-{}-{pkg}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir(&src, &dir);
        let share = Arc::new(WidthShare::verifying());
        let with = run(&dir, Some(share.clone()));
        let without = run(&dir, None);
        assert_eq!(with, without, "{pkg}: the share changes a result");
        let (hits, misses) = share.counts();
        assert_eq!(share.mismatches(), 0, "{pkg}: a hit differs from a fresh analysis");
        assert!(hits > misses, "{pkg}: tests share ({hits} hits, {misses} misses)");
        let _ = std::fs::remove_dir_all(&dir);
        ran += 1;
    }
    let _ = ran;
}
