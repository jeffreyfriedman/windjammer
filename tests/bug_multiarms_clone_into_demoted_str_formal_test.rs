#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! P3.734: LedgerKit `render_read_model` keeps `json: String` because most
//! match arms pass it to owned `string` parsers. A few parsers demote to
//! `&str` (`parse_balance_sheet_fields`, `parse_general_ledger_fields`,
//! `parse_close_checklist_days_to_close`, …) but the call site still emits
//! `json.clone()` (E0308). A two-arm isolate borrows (`&json`); the product
//! match does not.
//!
//! This gate reads the tip-regenerated finance-screens emit. It stays RED
//! until that emit borrows into the demoted formals.

use std::path::PathBuf;

fn search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let mut walked = manifest;
    for _ in 0..8 {
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
            roots.push(walked.clone());
        } else {
            break;
        }
    }
    roots
}

fn product_read_models() -> PathBuf {
    let rels = [
        "financial-management/financial-management-platform/packages/finance-screens/build/read_models.rs",
        "financial-management-platform/packages/finance-screens/build/read_models.rs",
        "packages/finance-screens/build/read_models.rs",
    ];
    for dir in search_roots() {
        for rel in rels {
            let p = dir.join(rel);
            if p.is_file() {
                return p;
            }
        }
    }
    panic!("P3.734: finance-screens build/read_models.rs not found — tip-regen first");
}

#[test]
fn multiarm_owned_json_clone_must_not_feed_demoted_str_parser() {
    let path = product_read_models();
    let rs = std::fs::read_to_string(&path).expect("read read_models.rs");
    let parsers = [
        "parse_balance_sheet_fields",
        "parse_income_statement_fields",
        "parse_general_ledger_fields",
        "parse_close_checklist_days_to_close",
    ];
    let json_rs = path
        .parent()
        .map(|d| d.join("json.rs"))
        .filter(|p| p.is_file())
        .map(|p| std::fs::read_to_string(p).unwrap_or_default())
        .unwrap_or_default();
    let mut hits = Vec::new();
    for name in parsers {
        let demoted = json_rs.contains(&format!("fn {name}(json: &str)"));
        let cloned = rs.contains(&format!("{name}(json.clone())"));
        if demoted && cloned {
            hits.push(name);
        }
    }
    assert!(
        hits.is_empty(),
        "P3.734 RED: owned json.clone() into demoted &str parser in {}:\n{}",
        path.display(),
        hits.join("\n")
    );
}
