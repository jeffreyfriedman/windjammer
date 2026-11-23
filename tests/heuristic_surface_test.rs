//! Heuristic-surface ratchet: is convergence shrinking the name-based guessing, or growing it?
//!
//! Counts, per file in the deciding layers (`src/codegen`, `src/analyzer`, `src/ir`, `src/typing`,
//! `src/type_inference`), three kinds of heuristic surface (see `tests/common/heuristic_scan.rs`):
//! name literals compared or searched (`name_matches`), name literals as match-arm patterns
//! (`match_arm_names`) and `*fallback*` / `*legacy*` identifiers (`fallback_legacy`).
//! Each per-file number is compared with `docs/heuristic-allowlist.toml`: it may only fall, a new
//! file may not appear, and the allow-list may never be higher than reality. The totals are
//! published in `docs/spec/convergence-burndown.md` and must match the allow-list.
//!
//! Lower the baseline after a deletion with
//! `cargo test --test all heuristic_surface_test::regenerate_heuristic_baseline -- --ignored`.

#[path = "common/arch_scan.rs"]
mod arch_scan;
#[path = "common/heuristic_scan.rs"]
mod heuristic_scan;

use arch_scan::{compare, format_diff, Counts, SrcFile};
use heuristic_scan::{count_file, is_test_file, spells_name, string_literals, uncounted_forms, Exempt, FileCounts, UncountedForms};
use std::collections::BTreeMap;
use std::path::PathBuf;

const ALLOWLIST_PATH: &str = "docs/heuristic-allowlist.toml";
const BURNDOWN_PATH: &str = "docs/spec/convergence-burndown.md";
const COUNTED_DIRS: &[&str] = &["src/codegen/", "src/analyzer/", "src/ir/", "src/typing/", "src/type_inference/"];
const SECTIONS: [&str; 3] = ["name_matches", "match_arm_names", "fallback_legacy"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------------------------
// Allow-list
// ---------------------------------------------------------------------------------------------

#[derive(serde::Deserialize, Default)]
struct Allowlist {
    /// Documented conventions a literal may spell: `name = "doc path that documents it"`.
    #[serde(default)]
    conventions: BTreeMap<String, String>,
    #[serde(default)]
    name_matches: Counts,
    #[serde(default)]
    match_arm_names: Counts,
    #[serde(default)]
    fallback_legacy: Counts,
}

impl Allowlist {
    fn section(&self, s: &str) -> &Counts {
        match s {
            "name_matches" => &self.name_matches,
            "match_arm_names" => &self.match_arm_names,
            _ => &self.fallback_legacy,
        }
    }
}

fn load_allowlist() -> Allowlist {
    let Ok(text) = std::fs::read_to_string(root().join(ALLOWLIST_PATH)) else {
        return Allowlist::default();
    };
    toml::from_str(&text).expect("docs/heuristic-allowlist.toml parses")
}

// ---------------------------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------------------------

/// File stems and directory names under `std/`: the Windjammer std module names.
fn wj_std_modules() -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(root().join("std")) else { return Vec::new() };
    rd.flatten()
        .filter_map(|e| {
            let p = e.path();
            let stem = p.file_stem()?.to_string_lossy().to_string();
            (p.is_dir() || p.extension().is_some_and(|x| x == "wj" || x == "wji")).then_some(stem)
        })
        .collect()
}

fn exempt(a: &Allowlist) -> Exempt {
    Exempt::new(wj_std_modules(), a.conventions.keys().cloned())
}

fn counted(f: &SrcFile) -> bool {
    COUNTED_DIRS.iter().any(|d| f.rel.starts_with(d)) && !is_test_file(&f.rel)
}

fn measure_files(files: &[SrcFile], ex: &Exempt) -> BTreeMap<&'static str, Counts> {
    let mut out: BTreeMap<&'static str, Counts> = SECTIONS.iter().map(|s| (*s, Counts::new())).collect();
    for f in files.iter().filter(|f| counted(f)) {
        let c = count_file(&f.raw, &f.code, ex);
        for (s, n) in [("name_matches", c.name_matches), ("match_arm_names", c.match_arm_names), ("fallback_legacy", c.fallback_legacy)] {
            if n > 0 {
                out.get_mut(s).unwrap().insert(f.rel.clone(), n);
            }
        }
    }
    out
}

fn measure() -> BTreeMap<&'static str, Counts> {
    measure_files(&arch_scan::load_src_files(&root()), &exempt(&load_allowlist()))
}

fn total(c: &Counts) -> u64 {
    c.values().sum()
}

// ---------------------------------------------------------------------------------------------
// Scanner unit tests (hand-counted snippets are the oracle)
// ---------------------------------------------------------------------------------------------

fn counts_of(src: &str) -> FileCounts {
    count_file(src, &arch_scan::strip_code(src), &Exempt::new(Vec::new(), Vec::new()))
}

#[test]
fn literal_lexer_skips_comments_char_literals_and_lifetimes() {
    let chars: Vec<char> = "// \"a\"\nfn f<'a>(c: char) -> &'a str { let q = '\"'; /* \"b\" */ r#\"c\"# ; \"d\\\"e\" }".chars().collect();
    let texts: Vec<String> = string_literals(&chars).into_iter().map(|l| l.text).collect();
    assert_eq!(texts, vec!["c".to_string(), "d\\\"e".to_string()]);
}

#[test]
fn spells_name_accepts_identifiers_paths_and_prefixes_only() {
    for yes in ["Widget", "app_", "Vec<", "::new", ".clone", "std::fs", "a::B::c", "to_string("] {
        assert!(spells_name(yes), "{yes} spells a name");
    }
    for no in ["", "_ ", "a b", "1x", "{}", "fn main() {", "&mut ", "->", "0", "Vec<int>"] {
        assert!(!spells_name(no), "{no:?} does not spell a name");
    }
}

#[test]
fn name_comparisons_are_counted_in_every_form() {
    let src = r#"
fn f(n: &str) -> bool {
    n == "Widget" || "Gadget" != n || n.starts_with("app_") || n.ends_with("Map")
        || n.contains(&"Shared") || n.strip_prefix("desktop_").is_some() || n.strip_suffix("SI").is_some()
}"#;
    assert_eq!(counts_of(src), FileCounts { name_matches: 7, match_arm_names: 0, fallback_legacy: 0 });
}

#[test]
fn match_arm_name_patterns_are_counted_including_alternatives_options_and_guards() {
    let src = r#"
fn f(n: Option<&str>) -> u8 {
    match n {
        Some("Alpha") => 1,
        Some("Beta" | "Gamma") => 2,
        Some("Delta") if true => 3,
        _ => 0,
    }
}
fn g(n: &str) -> u8 { match n { "Epsilon" | "Zeta" => 1, _ => 0 } }"#;
    assert_eq!(counts_of(src).match_arm_names, 6);
}

#[test]
fn non_name_literals_and_other_uses_are_not_counted() {
    let src = r#"
fn f(n: &str, out: &mut String) -> bool {
    out.push_str("Widget");
    let s = format!("{}::new", n);
    let v = vec!["Gadget"];
    let c = n.split(',').count() == 2 || n.contains(", ") || n.starts_with('x') || n == "a b";
    c || v.is_empty() || s.is_empty() || n.ends_with(" {")
}"#;
    assert_eq!(counts_of(src), FileCounts::default());
}

#[test]
fn closures_and_boolean_or_are_not_match_arms() {
    let src = r#"fn f(v: Vec<u8>, b: bool, n: &str) -> bool { let _x: Vec<&str> = v.iter().map(|_| "Name").collect(); b || n == "Other" }"#;
    let c = counts_of(src);
    assert_eq!((c.match_arm_names, c.name_matches), (0, 1));
}

#[test]
fn exempt_vocabulary_covers_keywords_primitives_std_modules_and_conventions() {
    let ex = Exempt::new(vec!["http".to_string()], vec!["main".to_string()]);
    for word in ["self", "Self", "int", "string", "usize", "f32", "std", "std::fs", "http", "main", "Vec<"] {
        let src = format!("fn f(n: &str) -> bool {{ n == \"{word}\" }}");
        let c = count_file(&src, &arch_scan::strip_code(&src), &ex);
        let expected = u64::from(word == "Vec<");
        assert_eq!(c.name_matches, expected, "{word}");
    }
}

#[test]
fn fallback_and_legacy_identifiers_are_counted() {
    let src = "fn legacy_width() {}\nfn decide() { if x { width_fallback() } else { legacy_width() } }\nlet FallbackMode = 1; // legacy in a comment\n";
    assert_eq!(counts_of(src).fallback_legacy, 4);
}

#[test]
fn test_modules_and_test_files_are_not_counted() {
    let src = "fn f(n: &str) -> bool { n == \"Widget\" }\n#[cfg(test)]\nmod tests {\n fn t(n: &str) -> bool { n == \"Gadget\" || legacy() }\n}\n";
    assert_eq!(counts_of(src), FileCounts { name_matches: 1, match_arm_names: 0, fallback_legacy: 0 });
    assert!(is_test_file("src/codegen/rust/foo_tests.rs") && is_test_file("src/ir/tests/a.rs"));
    assert!(!is_test_file("src/codegen/rust/test_detection.rs"));
}

// ---------------------------------------------------------------------------------------------
// Forms the counts do not see yet (report-only; stage 9 of docs/design/table-driven-std-surface.md enforces them)
// ---------------------------------------------------------------------------------------------

fn uncounted_of(src: &str) -> UncountedForms {
    uncounted_forms(src, &arch_scan::strip_code(src), &Exempt::new(Vec::new(), Vec::new()))
}

#[test]
fn tuple_pattern_and_matches_list_names_are_found_without_double_counting() {
    // hand-counted: two tuple-pattern literals ("insert", "get"), one in a guarded arm; one matches! list of three
    let src = r#"
fn a(name: &str, i: usize) -> u8 {
    match (name, i) {
        ("insert", 0) => 1,
        ("get", 0) | ("peek", 1) => 2,
        (n, _) if n.len() > 3 => 3,
        _ => 0,
    }
}
fn b(name: &str) -> bool { matches!(name, "len" | "count" | "capacity") }
fn c(name: &str) -> bool { matches!(name.len(), 0) }
"#;
    assert_eq!(uncounted_of(src), UncountedForms { tuple_pattern_names: 3, matches_list_names: 3 });
    // already counted forms stay in their own counts; a call with a literal argument is not a pattern
    let counted = r#"fn f(n: &str) -> u8 { match n { "Alpha" => 1, _ => 0 } }
fn g(n: &str) -> bool { n == "Beta" || foo("Gamma", 1) }
"#;
    assert_eq!(uncounted_of(counted), UncountedForms::default());
}

/// Print the uncounted forms per file and in total. Never fails: stage 9 enforces them with the baseline this
/// prints (`cargo test --test all heuristic_surface_test::report_uncounted_name_forms -- --nocapture`).
#[test]
fn report_uncounted_name_forms() {
    let ex = exempt(&load_allowlist());
    let mut rows: Vec<(String, UncountedForms)> = Vec::new();
    for f in arch_scan::load_src_files(&root()).iter().filter(|f| counted(f)) {
        let u = uncounted_forms(&f.raw, &f.code, &ex);
        if u != UncountedForms::default() {
            rows.push((f.rel.clone(), u));
        }
    }
    let (t, m): (u64, u64) = rows.iter().fold((0, 0), |(t, m), (_, u)| (t + u.tuple_pattern_names, m + u.matches_list_names));
    for (rel, u) in &rows {
        println!("uncounted\t{rel}\ttuple_pattern_names={}\tmatches_list_names={}", u.tuple_pattern_names, u.matches_list_names);
    }
    println!("uncounted total: tuple_pattern_names {t}, matches_list_names {m} (sum {})", t + m);
}

// ---------------------------------------------------------------------------------------------
// Ratchet
// ---------------------------------------------------------------------------------------------

fn assert_no_regression(section: &str) {
    let allow = load_allowlist();
    let measured = measure();
    let diff = compare(&measured[section], allow.section(section));
    assert!(
        diff.regressions.is_empty(),
        "[{section}] heuristic surface grew. Decide from a declaration, a parsed type or a solver fact \
         instead of matching a name (see {BURNDOWN_PATH}, 'Heuristic surface').\n{}",
        format_diff("regression", &diff.regressions)
    );
}

fn assert_tight(section: &str) {
    let allow = load_allowlist();
    let measured = measure();
    let diff = compare(&measured[section], allow.section(section));
    assert!(
        diff.stale.is_empty(),
        "[{section}] heuristic surface shrank: lower the baseline so it cannot come back \
         (cargo test --test all heuristic_surface_test::regenerate_heuristic_baseline -- --ignored, then update the \
         totals in {BURNDOWN_PATH}).\n{}",
        format_diff("stale", &diff.stale)
    );
}

#[test]
fn name_matches_do_not_grow() {
    assert_no_regression("name_matches");
}

#[test]
fn name_matches_allowlist_is_tight() {
    assert_tight("name_matches");
}

#[test]
fn match_arm_names_do_not_grow() {
    assert_no_regression("match_arm_names");
}

#[test]
fn match_arm_names_allowlist_is_tight() {
    assert_tight("match_arm_names");
}

#[test]
fn fallback_legacy_paths_do_not_grow() {
    assert_no_regression("fallback_legacy");
}

#[test]
fn fallback_legacy_allowlist_is_tight() {
    assert_tight("fallback_legacy");
}

/// Mutation check: adding one name match, one match-arm name and one fallback path to a real
/// counted file must make every section of the gate fail for that file.
#[test]
fn gate_fails_when_a_name_guess_is_added() {
    let allow = load_allowlist();
    let ex = exempt(&allow);
    let mut files = arch_scan::load_src_files(&root());
    let target = files.iter_mut().find(|f| counted(f)).expect("a counted file");
    let rel = target.rel.clone();
    target.raw.push_str(
        "\nfn probe_guess(n: &str) -> u8 { if n.starts_with(\"WidgetPanel\") { return probe_fallback(); } \
         match n { \"GadgetPanel\" => 1, _ => 0 } }\n",
    );
    target.code = arch_scan::strip_code(&target.raw);
    let mutated = measure_files(&files, &ex);
    for section in SECTIONS {
        let diff = compare(&mutated[section], allow.section(section));
        assert!(
            diff.regressions.iter().any(|(k, _, _)| k == &rel),
            "[{section}] the gate did not notice a name guess added to {rel}"
        );
    }
}

/// Every documented convention names a doc that mentions it.
#[test]
fn conventions_are_documented() {
    for (name, doc) in &load_allowlist().conventions {
        let text = std::fs::read_to_string(root().join(doc)).unwrap_or_default();
        assert!(text.contains(&format!("`{name}`")), "convention `{name}` is not documented in {doc}");
    }
}

/// The burndown doc publishes the same totals as the allow-list.
#[test]
fn burndown_publishes_current_totals() {
    let allow = load_allowlist();
    let doc = std::fs::read_to_string(root().join(BURNDOWN_PATH)).expect("burndown doc");
    let line = published_totals_line(&allow);
    assert!(doc.contains(&line), "{BURNDOWN_PATH} must contain the current totals:\n{line}");
}

fn published_totals_line(a: &Allowlist) -> String {
    let (n, m, f) = (total(&a.name_matches), total(&a.match_arm_names), total(&a.fallback_legacy));
    format!("`H-NAME-MATCHES` {n}, `H-MATCH-ARM-NAMES` {m}, `H-FALLBACK-LEGACY` {f} (total {})", n + m + f)
}

// ---------------------------------------------------------------------------------------------
// Baseline regeneration (ignored; never raises a number unless the file does not exist yet)
// ---------------------------------------------------------------------------------------------

#[test]
#[ignore = "writes docs/heuristic-allowlist.toml"]
fn regenerate_heuristic_baseline() {
    let path = root().join(ALLOWLIST_PATH);
    let bootstrap = !path.exists();
    let old = load_allowlist();
    let now = measure();
    for section in SECTIONS {
        let diff = compare(&now[section], old.section(section));
        assert!(
            bootstrap || diff.regressions.is_empty(),
            "[{section}] refusing to raise the baseline:\n{}",
            format_diff("regression", &diff.regressions)
        );
    }
    let mut s = String::from(
        "# Heuristic-surface ratchet baseline. Read by tests/heuristic_surface_test.rs.\n\
         # Per-file counts may only go DOWN. Lower them after a deletion with\n\
         #   cargo test --test all heuristic_surface_test::regenerate_heuristic_baseline -- --ignored\n\
         # and update the published totals in docs/spec/convergence-burndown.md.\n\
         # [conventions]: names a literal may spell without counting; each must be documented\n\
         # (in backticks) in the doc it points to.\n\n[conventions]\n",
    );
    for (k, v) in &old.conventions {
        s.push_str(&format!("\"{k}\" = \"{v}\"\n"));
    }
    for section in SECTIONS {
        s.push_str(&format!("\n[{section}]\n"));
        for (k, v) in &now[section] {
            s.push_str(&format!("\"{k}\" = {v}\n"));
        }
    }
    std::fs::write(&path, s).unwrap();
    let new = load_allowlist();
    println!("{}", published_totals_line(&new));
}
