//! The parsed files shared between the tests of one `wj test --interpret` run
//! (`docs/design/interp-soundness/parseshare.md`): a shared parse equals a fresh one, a changed file is parsed afresh,
//! the tests of one module parse each file once, and results do not depend on the order of the tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use windjammer::interpreter::loader::parse_source_renaming;
use windjammer::interpreter::parse_share::ParseShare;
use windjammer::interpreter::runner::{run_specs_shared_with, TestResult};
use windjammer::source_map::elided_debug;
use windjammer::test_runner::interpreted::discover_specs;

const LIB: &str = "pub fn apply(f: fn(i64) -> i64, x: i64) -> i64 {\n    f(x)\n}\n\npub fn adder(n: i64) -> fn(i64) -> i64 {\n    |x| x + n\n}\n\npub fn sum_with(v: Vec<i64>, k: i64) -> i64 {\n    let mut t = 0\n    for x in v {\n        t += apply(|y| y * k, x)\n    }\n    t\n}\n";

fn test_file(first: i64, count: usize) -> String {
    let mut s = String::from("use crate::{apply, adder, sum_with}\n\n");
    for n in 0..count {
        let k = first + n as i64;
        s.push_str(&format!(
            "fn test_t_{n}() {{\n    let add = adder({k})\n    assert_eq(apply(add, 1), {})\n    let bump = |y| y + {n}\n    assert_eq(apply(bump, 2), {})\n    assert_eq(sum_with(vec![1, 2, 3], {k}), {})\n}}\n\n",
            1 + k,
            2 + n as i64,
            6 * k
        ));
    }
    s
}

fn project(name: &str, tests: &[(&str, String)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wj-parseshare-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(dir.join("wj.toml"), "[package]\nname = \"ps\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n").unwrap();
    std::fs::write(dir.join("src/lib.wj"), LIB).unwrap();
    for (n, text) in tests {
        std::fs::write(dir.join("tests").join(n), text).unwrap();
    }
    dir
}

fn run_in_order(dir: &Path, share: Option<Arc<ParseShare>>, reverse: bool) -> Vec<(String, bool, String)> {
    let mut specs = discover_specs(dir).unwrap();
    assert!(!specs.is_empty());
    if reverse {
        specs.reverse();
    }
    let r: Vec<TestResult> = run_specs_shared_with(specs, None, None, None, share);
    let mut v: Vec<_> = r.into_iter().map(|r| (r.name, r.passed, scrub_scratch(&r.message))).collect();
    v.sort();
    v
}

fn run(dir: &Path, share: Option<Arc<ParseShare>>) -> Vec<(String, bool, String)> {
    run_in_order(dir, share, false)
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
fn tests_of_one_module_parse_each_file_once() {
    let dir = project("once", &[("a_test.wj", test_file(5, 4))]);
    let share = Arc::new(ParseShare::verifying());
    let shared = run(&dir, Some(share.clone()));
    assert!(shared.iter().all(|r| r.1), "{shared:?}");
    let (hits, misses) = share.counts();
    assert_eq!(misses, 2, "one parse per distinct file text (the library and the test file)");
    assert_eq!(hits, 2 * (4 - 1), "every later test hits");
    assert_eq!(share.mismatches(), 0);
    assert_eq!(shared, run(&dir, None), "results equal an unshared run");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_changed_test_file_reparses_only_its_own_text() {
    let dir = project("change", &[("a_test.wj", test_file(5, 2)), ("b_test.wj", test_file(7, 2))]);
    let share = Arc::new(ParseShare::verifying());
    let first = run(&dir, Some(share.clone()));
    assert_eq!(share.counts().1, 3, "lib, a_test, b_test");
    assert_eq!(run(&dir, Some(share.clone())), first);
    assert_eq!(share.counts().1, 3, "an unchanged second run parses nothing");
    std::fs::write(dir.join("tests/a_test.wj"), test_file(9, 3)).unwrap();
    let after = run(&dir, Some(share.clone()));
    assert_eq!(share.counts().1, 4, "only the changed file's text is a miss");
    assert_eq!(after, run(&dir, None), "the result with the share equals a fresh run");
    assert!(after.iter().any(|r| r.0.contains("test_t_2")), "the edit added a test: {after:?}");
    assert_eq!(share.mismatches(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_same_name_with_new_text_is_parsed_afresh() {
    let share = ParseShare::verifying();
    let none = HashMap::new();
    let a = "fn f() -> i64 {\n    1\n}\n";
    let b = "fn f() -> i64 {\n    2\n}\n";
    let pa = elided_debug(share.parse(a, "same.wj", &none).unwrap());
    let pb = elided_debug(share.parse(b, "same.wj", &none).unwrap());
    assert_ne!(pa, pb);
    assert_eq!(pa, elided_debug(&parse_source_renaming(a, "same.wj", &none).unwrap()));
    assert_eq!(pb, elided_debug(&parse_source_renaming(b, "same.wj", &none).unwrap()));
    assert_eq!(elided_debug(share.parse(a, "same.wj", &none).unwrap()), pa, "back to the first text: a hit");
    assert_eq!(share.counts(), (1, 2));
    assert_eq!(share.mismatches(), 0);
}

#[test]
fn renames_are_part_of_the_key() {
    let share = ParseShare::verifying();
    let src = "fn f(v: Vec<i64>) -> i64 {\n    v.size()\n}\n";
    let plain = HashMap::new();
    let renamed: HashMap<String, String> = [("size".to_string(), "len".to_string())].into();
    let a = elided_debug(share.parse(src, "r.wj", &plain).unwrap());
    let b = elided_debug(share.parse(src, "r.wj", &renamed).unwrap());
    assert_ne!(a, b, "the rename turns `size` into `len`");
    assert_eq!(b, elided_debug(&parse_source_renaming(src, "r.wj", &renamed).unwrap()));
    assert_eq!(share.counts(), (0, 2));
}

#[test]
fn a_failing_parse_is_not_kept() {
    let share = ParseShare::new();
    let none = HashMap::new();
    assert!(share.parse("fn (((", "bad.wj", &none).is_err());
    assert!(share.parse("fn (((", "bad.wj", &none).is_err());
    assert_eq!(share.counts(), (0, 2));
}

#[test]
fn a_shared_parse_equals_a_fresh_one_over_the_repo_programs() {
    let share = ParseShare::verifying();
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
            let Ok(fresh) = parse_source_renaming(&src, &file, &none) else { continue };
            let fresh = elided_debug(&fresh);
            assert_eq!(elided_debug(share.parse(&src, &file, &none).unwrap()), fresh, "{file}: miss");
            assert_eq!(elided_debug(share.parse(&src, &file, &none).unwrap()), fresh, "{file}: hit");
            n += 1;
        }
    }
    assert!(n > 20, "the corpus is the repo's programs ({n})");
    assert_eq!(share.mismatches(), 0);
    assert_eq!(share.counts(), (n as u64, n as u64));
}

#[test]
fn results_do_not_depend_on_the_order_of_the_tests() {
    // closures are defined, compiled and called by one test and the same functions run in the next: forward, reversed
    // and unshared runs all agree
    let dir = project("order", &[("a_test.wj", test_file(3, 5)), ("b_test.wj", test_file(11, 5))]);
    let forward = run_in_order(&dir, Some(Arc::new(ParseShare::verifying())), false);
    assert!(forward.iter().all(|r| r.1), "{forward:?}");
    let share = Arc::new(ParseShare::verifying());
    let reversed = run_in_order(&dir, Some(share.clone()), true);
    assert_eq!(forward, reversed);
    assert_eq!(forward, run(&dir, None));
    assert_eq!(share.mismatches(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failing_test_does_not_affect_its_neighbours() {
    let bad = "use crate::{apply, adder}\n\nfn test_a_wrong() {\n    assert_eq(apply(adder(1), 1), 99)\n}\n\nfn test_b_right() {\n    assert_eq(apply(adder(1), 1), 2)\n}\n\nfn test_c_wrong() {\n    assert_eq(apply(adder(2), 1), 99)\n}\n\nfn test_d_right() {\n    assert_eq(apply(adder(2), 1), 3)\n}\n";
    let dir = project("neighbours", &[("a_test.wj", bad.to_string())]);
    let shared = run(&dir, Some(Arc::new(ParseShare::verifying())));
    let passed: Vec<_> = shared.iter().filter(|r| r.1).map(|r| r.0.clone()).collect();
    assert_eq!(passed.len(), 2, "{shared:?}");
    assert_eq!(shared, run(&dir, None));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ecosystem_packages_give_identical_results_with_and_without_the_share() {
    let root = std::env::var_os("WJ_ECOSYSTEM").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("src/windjammer-ecosystem")));
    let Some(root) = root.map(|r| r.join("packages")).filter(|r| r.is_dir()) else { return };
    for pkg in ["wj-semver", "wj-decimal", "wj-fuzzy"] {
        let src = root.join(pkg);
        if !src.is_dir() {
            continue;
        }
        let dir = std::env::temp_dir().join(format!("wj-parseshare-{}-{pkg}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir(&src, &dir);
        let share = Arc::new(ParseShare::verifying());
        let with = run(&dir, Some(share.clone()));
        let without = run(&dir, None);
        assert_eq!(with, without, "{pkg}: the share changes a result");
        let (hits, misses) = share.counts();
        assert_eq!(share.mismatches(), 0, "{pkg}: a hit differs from a fresh parse");
        assert!(hits > misses, "{pkg}: tests share ({hits} hits, {misses} misses)");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
