//! Location-dependence ratchet: code generation decides from declarations (the manifest, the source module tree, the
//! output root the driver names), never from what a directory is called or what happens to exist around the build
//! output. Every remaining filesystem probe or file-name read in `src/codegen/` is counted per file here: an
//! `.exists()`, `.is_dir()`, `.is_file()`, `.file_name()` or `.file_stem()` call. The counts in
//! `docs/location-dependence-baseline.toml` may only fall, a new file may not appear, and the baseline may never be
//! higher than reality. Lower it after removing probes with
//! `cargo test --release --test location_dependence_ratchet -- --ignored regenerate`.
//!
//! Probes of the source module tree through `import_generation/module_position.rs` are the declared module
//! structure and are counted like any other, so moving a decision onto them shows up as a shift between files.

#[path = "common/arch_scan.rs"]
mod arch_scan;

use arch_scan::{compare, format_diff, per_file, Counts};
use std::path::PathBuf;

const BASELINE_PATH: &str = "docs/location-dependence-baseline.toml";
const PROBES: &[&str] = &[".exists()", ".is_dir()", ".is_file()", ".file_name()", ".file_stem()"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Probe calls in `code` (comments and literals already blanked).
fn count_probes(code: &str) -> u64 {
    PROBES.iter().map(|p| code.matches(p).count() as u64).sum()
}

fn is_test_code(rel: &str) -> bool {
    rel.ends_with("_test.rs") || rel.ends_with("/tests.rs") || rel.contains("/tests/")
}

fn measure() -> Counts {
    let files: Vec<_> = arch_scan::load_src_files(&root())
        .into_iter()
        .filter(|f| f.rel.starts_with("src/codegen/") && !is_test_code(&f.rel))
        .collect();
    per_file(&files, |f| {
        // `#[cfg(test)] mod tests { .. }` at the end of a file is test code.
        let code = f.code.split("#[cfg(test)]").next().unwrap_or(&f.code);
        count_probes(code)
    })
}

fn load_baseline() -> Counts {
    let text = std::fs::read_to_string(root().join(BASELINE_PATH)).unwrap_or_default();
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with('"'))
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            Some((k.trim().trim_matches('"').to_string(), v.trim().parse().ok()?))
        })
        .collect()
}

fn render(counts: &Counts) -> String {
    let total: u64 = counts.values().sum();
    let mut s = format!(
        "# Filesystem probes and file-name reads per file in src/codegen (tests/location_dependence_ratchet_test.rs).\n\
         # May only shrink. Total: {total}\n\n[location_probes]\n"
    );
    for (k, v) in counts {
        s.push_str(&format!("\"{k}\" = {v}\n"));
    }
    s
}

#[test]
fn scanner_counts_probe_calls() {
    assert_eq!(count_probes("a.exists() || b.is_dir() && c.file_name().is_some() || d.is_file() || e.file_stem()"), 5);
    assert_eq!(count_probes("exists(x) || is_dir"), 0);
}

#[test]
fn location_probes_do_not_grow() {
    let d = compare(&measure(), &load_baseline());
    assert!(d.regressions.is_empty(), "filesystem probes grew in src/codegen (decide from declarations):\n{}", format_diff("grew", &d.regressions));
}

#[test]
fn location_probe_baseline_is_tight() {
    let d = compare(&measure(), &load_baseline());
    assert!(d.stale.is_empty(), "filesystem probes fell: lower {BASELINE_PATH} (regenerate):\n{}", format_diff("fell", &d.stale));
}

#[test]
#[ignore]
fn regenerate() {
    std::fs::write(root().join(BASELINE_PATH), render(&measure())).unwrap();
}
