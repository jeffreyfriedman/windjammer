//! Guard against silently switching off the "full suite" test files.
//!
//! Most bug-regression files carry `#![cfg(not(any(feature = "<suite>_tests", ...)))]`, meaning
//! "run only when no opt-in suite feature is selected". `cargo test --all-features` enables every
//! suite feature at once and therefore compiles all of those files OUT, reporting a green run that
//! executed a fraction of the tests. This file makes that mistake loud.

use std::fs;
use std::path::Path;

/// Fails when every suite feature is enabled at once (i.e. `--all-features`).
/// The cfg list is kept in sync with `[features]` by `suite_features_match_gate_headers`.
#[cfg(all(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests"
))]
#[test]
fn all_features_disables_gated_test_files() {
    panic!(
        "all six *_tests suite features are enabled (`--all-features`). That compiles out every \
         file gated with #![cfg(not(any(...)))] and silently skips them. Run `cargo test` \
         without --all-features (use --all-features only for clippy/check)."
    );
}

fn suite_features_in_cargo_toml(manifest: &str) -> Vec<String> {
    let mut in_features = false;
    let mut out = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_features = t == "[features]";
            continue;
        }
        if in_features {
            if let Some((name, _)) = t.split_once('=') {
                let name = name.trim();
                if name.ends_with("_tests") {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

fn features_named_in_gate_headers(tests_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(tests_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap_or_default();
        for line in src.lines().take(40) {
            let t = line.trim();
            if t.starts_with("feature = \"") {
                if let Some(name) = t.trim_start_matches("feature = \"").split('"').next() {
                    if name.ends_with("_tests") {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The `all_features_disables_gated_test_files` cfg must list exactly the suite features that
/// exist; a new `*_tests` feature added to Cargo.toml must also be added there.
#[test]
fn suite_features_match_gate_headers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = suite_features_in_cargo_toml(&manifest);
    let guard_src = fs::read_to_string(root.join("tests/feature_gate_guard_test.rs")).unwrap();
    for f in &declared {
        let needle = format!("feature = \"{f}\"");
        assert!(
            guard_src.matches(&needle).count() >= 1,
            "suite feature `{f}` is not listed in the all-features guard cfg of \
             tests/feature_gate_guard_test.rs"
        );
    }
    let used = features_named_in_gate_headers(&root.join("tests"));
    for f in &used {
        assert!(
            declared.contains(f),
            "test headers reference feature `{f}` that is not a *_tests feature in Cargo.toml"
        );
    }
}
