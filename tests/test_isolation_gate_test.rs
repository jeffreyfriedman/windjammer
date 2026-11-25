//! Process-global state in tests: no `std::env::set_current_dir`, `set_var` or `remove_var` in `tests/`, in the test
//! helpers (`src/test_utils.rs`) or in a source file's unit-test section (after its first `#[cfg(test)]`), except the
//! shrink-only allowlist `docs/test-isolation-allowlist.toml`. The working directory and the environment belong to the
//! whole test process, so a change by one test is seen by every concurrent one (a chdir into a temp dir that is deleted
//! later: "Current directory is invalid" for unrelated cases). Use `Command::current_dir` / `Command::env`, or
//! `windjammer::test_utils::in_dir` (a per-thread override, no chdir).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Allow {
    file: String,
    max: usize,
    reason: String,
}

#[derive(Deserialize, Default)]
struct AllowFile {
    #[serde(default)]
    allow: Vec<Allow>,
}

fn needles() -> Vec<String> {
    // assembled so this file does not match itself
    ["set_current_dir", "set_var", "remove_var"].iter().map(|f| format!("env::{f}(")).collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        let name = e.file_name();
        if name == "target" || name.to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Number of process-global calls in the part of `text` the gate covers.
fn violations(rel: &str, text: &str) -> usize {
    let skip = if rel.starts_with("tests/") || rel == "src/test_utils.rs" {
        0
    } else {
        // a source file: only its unit-test section
        match text.find("#[cfg(test)]") {
            Some(i) => text[..i].lines().count(),
            None => return 0,
        }
    };
    let ns = needles();
    text.lines()
        .skip(skip)
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(|l| ns.iter().filter(|n| l.contains(n.as_str())).count())
        .sum()
}

/// Per-file violation counts under `root`.
fn scan(root: &Path, gate_file: &str) -> BTreeMap<String, usize> {
    let mut files = Vec::new();
    walk(&root.join("tests"), &mut files);
    walk(&root.join("src"), &mut files);
    let mut found = BTreeMap::new();
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if rel == gate_file {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let n = violations(&rel, &text);
        if n > 0 {
            found.insert(rel, n);
        }
    }
    found
}

/// Problems: a violation outside the allowlist, a file over its `max`, a stale or loose entry (the list only shrinks).
fn audit(found: &BTreeMap<String, usize>, allow: &AllowFile) -> Vec<String> {
    let mut problems = Vec::new();
    let allowed: BTreeMap<&str, &Allow> = allow.allow.iter().map(|a| (a.file.as_str(), a)).collect();
    for (file, n) in found {
        match allowed.get(file.as_str()) {
            None => problems.push(format!("{file}: {n} process-global call(s) (set_current_dir/set_var/remove_var): use Command::current_dir / Command::env, or test_utils::in_dir")),
            Some(a) if *n > a.max => problems.push(format!("{file}: {n} calls > allowlisted {} (the list only shrinks)", a.max)),
            Some(a) if *n < a.max => problems.push(format!("{file}: only {n} calls left: lower `max` in docs/test-isolation-allowlist.toml to {n}")),
            Some(_) => {}
        }
    }
    for a in &allow.allow {
        if !found.contains_key(&a.file) {
            problems.push(format!("{}: no calls left: remove it from docs/test-isolation-allowlist.toml", a.file));
        }
        if a.reason.trim().is_empty() {
            problems.push(format!("{}: an allowlist entry needs a reason", a.file));
        }
    }
    problems
}

fn load_allow(root: &Path) -> AllowFile {
    let text = std::fs::read_to_string(root.join("docs/test-isolation-allowlist.toml")).expect("docs/test-isolation-allowlist.toml");
    toml::from_str(&text).expect("allowlist parses")
}

#[test]
fn no_test_changes_process_global_state_outside_the_shrinking_allowlist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let problems = audit(&scan(root, "tests/test_isolation_gate_test.rs"), &load_allow(root));
    assert!(problems.is_empty(), "test isolation:\n  {}", problems.join("\n  "));
}

#[test]
fn selftest_planted_violations_are_caught() {
    let [cwd, set, rm] = [0, 1, 2].map(|i| needles()[i].clone());
    for (name, line) in [("chdir", cwd), ("set_var", set), ("remove_var", rm)] {
        let src = format!("fn t() {{\n    std::{line}x);\n}}\n");
        let found: BTreeMap<String, usize> = [("tests/planted_test.rs".to_string(), violations("tests/planted_test.rs", &src))].into_iter().collect();
        assert_eq!(found["tests/planted_test.rs"], 1, "{name} must be counted");
        assert!(!audit(&found, &AllowFile::default()).is_empty(), "{name}: a planted violation must fail the gate");
    }
    // comments, wj programs in strings (`fs::set_current_dir`) and the non-test part of a source file are not violations
    assert_eq!(violations("tests/a.rs", "// std::env::set_var(\"A\", \"b\")\nlet p = \"fs::set_current_dir(x)\";\n"), 0);
    assert_eq!(violations("src/x.rs", "fn prod() { std::env::set_var(\"A\", \"b\") }\n#[cfg(test)]\nmod t {}\n"), 0);
    assert_eq!(violations("src/x.rs", "#[cfg(test)]\nmod t { fn f() { std::env::remove_var(\"A\") } }\n"), 1);
    // the allowlist only shrinks: too many calls, too few calls and a stale entry all fail
    let allow = |max| AllowFile { allow: vec![Allow { file: "tests/a.rs".into(), max, reason: "r".into() }] };
    let at = |n: usize| -> BTreeMap<String, usize> { if n == 0 { BTreeMap::new() } else { [("tests/a.rs".to_string(), n)].into_iter().collect() } };
    assert!(audit(&at(2), &allow(2)).is_empty());
    assert!(!audit(&at(3), &allow(2)).is_empty(), "more calls than allowlisted");
    assert!(!audit(&at(1), &allow(2)).is_empty(), "fewer calls: lower max");
    assert!(!audit(&at(0), &allow(2)).is_empty(), "no calls left: remove the entry");
}
