//! `docs/reference/errors.md`: WJ codes, catalog (E) codes and lint (W) codes, all from code registries.

use super::util::table;
use windjammer::error_catalog::ErrorCatalog;
use windjammer::error_codes::get_registry;
use windjammer::linter::codes::LINT_CODES;

/// Every code the compiler can emit or document (used by the link checker too).
pub fn known_codes() -> Vec<String> {
    let mut v: Vec<String> = get_registry().all_codes().iter().map(|c| c.code.clone()).collect();
    v.extend(ErrorCatalog::new().errors.keys().cloned());
    v.extend(LINT_CODES.iter().map(|l| l.code.to_string()));
    v.sort();
    v.dedup();
    v
}

fn first_sentence(s: &str) -> String {
    let t = s.trim().lines().next().unwrap_or("");
    t.split(". ").next().unwrap_or(t).trim_end_matches('.').to_string()
}

pub fn errors() -> String {
    let mut wj: Vec<_> = get_registry().all_codes();
    wj.sort_by(|a, b| a.code.cmp(&b.code));
    let wj_rows: Vec<Vec<String>> = wj
        .iter()
        .map(|c| vec![format!("`{}`", c.code), c.title.clone(), format!("{:?}", c.maturity), c.rust_codes.join(", "), first_sentence(&c.explanation)])
        .collect();
    let cat = ErrorCatalog::new();
    let mut entries: Vec<_> = cat.errors.values().collect();
    entries.sort_by(|a, b| a.code.cmp(&b.code));
    let cat_rows: Vec<Vec<String>> = entries
        .iter()
        .map(|e| vec![format!("`{}`", e.code), e.title.clone(), e.category.clone(), e.severity.clone()])
        .collect();
    let lint_rows: Vec<Vec<String>> = LINT_CODES
        .iter()
        .map(|l| vec![format!("`{}`", l.code), l.name.to_string(), l.default_level.to_string(), l.summary.to_string()])
        .collect();
    format!(
        "# Error and lint codes\n\nGenerated from `src/error_codes.rs`, `src/error_catalog.rs` and `src/linter/codes.rs`. Long-form text: `wj explain <CODE>`.\n\n## WJ codes\n\n{}\n## Error catalog (translated rustc and intrinsic codes)\n\n{}\n## Lint codes (W)\n\n{}",
        table(&["Code", "Title", "Maturity", "Rust codes", "Summary"], &wj_rows),
        table(&["Code", "Title", "Category", "Severity"], &cat_rows),
        table(&["Code", "Name", "Default level", "Summary"], &lint_rows)
    )
}

pub fn lint_range() -> String {
    format!("{}-{}", LINT_CODES.first().map(|l| l.code).unwrap_or("W0000"), LINT_CODES.last().map(|l| l.code).unwrap_or("W0000"))
}
