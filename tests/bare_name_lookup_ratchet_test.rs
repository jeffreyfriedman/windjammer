//! Bare-name signature lookup ratchet: is callee identity moving from the bare name to the declaration?
//!
//! A signature lookup keyed by a bare function or method name (`registry.get_signature(method)`,
//! `find_unique_signature_ending_with(simple)`) reads whichever same-named function was registered, whatever the
//! call names: another trait's `append`, another module's private `draft`, another type's `active`. Each one
//! counted here is a site still keyed by the bare name instead of the resolved callee (the receiver's
//! `Type::method`, the defining module's path, the file's own declaration).
//!
//! The count is per file under `src/`: a call of a lookup function whose first argument is one of the bare-leaf
//! bindings the code base uses for a callee's last segment (`method`, `simple`, `method_simple`, ...). Each
//! per-file number is compared with `docs/bare-name-lookup-baseline.toml`: it may only fall, a new file may not
//! appear, and the baseline may never be higher than reality. Lower it after removing sites with
//! `cargo test --test all bare_name_lookup_ratchet_test::regenerate_bare_name_lookup_baseline -- --ignored`.

#[path = "common/arch_scan.rs"]
mod arch_scan;

use arch_scan::{compare, format_diff, per_file, Counts};
use std::path::PathBuf;

const BASELINE_PATH: &str = "docs/bare-name-lookup-baseline.toml";

/// Registry lookups that take a callee name.
const LOOKUPS: &[&str] = &[
    "get_signature",
    "lookup_method",
    "find_unique_signature_ending_with",
    "find_signature_ending_with",
    "find_signature_by_name_and_arg_count",
    "find_own_signature_by_name_and_arg_count",
    "resolve_callee_signature",
    "method_keys_for",
    "trait_definition_sigs_for_method",
    "global_trait_owned_plain_string_arg",
];

/// Bindings that hold a callee's bare last segment.
const BARE_LEAVES: &[&str] = &["method", "simple", "method_simple", "method_leaf", "call_method", "simple_name", "base_name"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Lookups in `code` (comments and literals already blanked) whose first argument is a bare leaf binding,
/// possibly behind `&` or a `registry,` / `global,` first argument (free functions take the registry first).
fn count_bare_lookups(code: &str) -> u64 {
    let mut n = 0;
    for lookup in LOOKUPS {
        let pat = format!("{lookup}(");
        let mut from = 0;
        while let Some(pos) = code[from..].find(&pat) {
            let at = from + pos;
            from = at + pat.len();
            if code[..at].chars().next_back().is_some_and(is_ident) {
                continue; // a longer name ending in `lookup`
            }
            let args: String = code[from..].chars().take_while(|c| *c != ')').collect();
            let mut parts = args.split(',').map(str::trim);
            let first = parts.next().unwrap_or("");
            let named = |a: &str| BARE_LEAVES.contains(&a.trim_start_matches('&').trim());
            // `fn lookup(&self, ...)` / `fn lookup(registry: ..)` declarations are not calls.
            let declaration = code[..at].trim_end().ends_with("fn");
            if !declaration && (named(first) || (!first.contains(':') && parts.next().is_some_and(named))) {
                n += 1;
            }
        }
    }
    n
}

fn measure() -> Counts {
    let files: Vec<_> = arch_scan::load_src_files(&root())
        .into_iter()
        .filter(|f| !f.rel.ends_with("_test.rs") && !f.rel.ends_with("/tests.rs") && !f.rel.contains("/tests/"))
        .collect();
    per_file(&files, |f| count_bare_lookups(&f.code))
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
        "# Bare-name signature lookups per file (tests/bare_name_lookup_ratchet_test.rs). May only shrink.\n\
         # Total: {total}\n\n[bare_name_lookups]\n"
    );
    for (k, v) in counts {
        s.push_str(&format!("\"{k}\" = {v}\n"));
    }
    s
}

#[test]
fn scanner_counts_bare_leaf_lookups_only() {
    let code = "fn f() { reg.get_signature(method); g.get_signature(&simple); reg.get_signature(&qualified); \
                find_unique_signature_ending_with(simple); global_trait_owned_plain_string_arg(g, method, 0); \
                trait_definition_sigs_for_method(registry, &key); my_get_signature(method); }\n\
                fn lookup_method(&self, name: &str) {}";
    assert_eq!(count_bare_lookups(code), 4);
}

#[test]
fn bare_name_lookups_do_not_grow() {
    let d = compare(&measure(), &load_baseline());
    assert!(d.regressions.is_empty(), "bare-name signature lookups grew (key the lookup by the resolved callee):\n{}", format_diff("grew", &d.regressions));
}

#[test]
fn bare_name_lookup_baseline_is_tight() {
    let d = compare(&measure(), &load_baseline());
    assert!(d.stale.is_empty(), "bare-name lookups fell: lower {BASELINE_PATH} (regenerate_bare_name_lookup_baseline):\n{}", format_diff("fell", &d.stale));
}

#[test]
#[ignore]
fn regenerate_bare_name_lookup_baseline() {
    std::fs::write(root().join(BASELINE_PATH), render(&measure())).unwrap();
}
