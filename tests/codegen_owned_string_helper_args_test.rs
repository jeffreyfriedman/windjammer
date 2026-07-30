#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! FAILING REPRO (dogfood): multi-string helper args must not be auto-borrowed.
//! Documents why seed match uses inline BankLineView construction instead of helpers.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn multi_string_helper_should_take_owned_args() {
    let source = r##"
pub struct BankLineView {
    id: string,
    status: string,
    matched_journal_entry_id: string,
}

pub fn apply_match(line: BankLineView, journal_entry_id: string) -> BankLineView {
    BankLineView {
        id: line.id + "",
        status: "matched".to_string(),
        matched_journal_entry_id: journal_entry_id + "",
    }
}

fn main() {
    let line = BankLineView {
        id: "bank~1000~demo-fit-01".to_string(),
        status: "unmatched".to_string(),
        matched_journal_entry_id: "".to_string(),
    }
    let je = "seed-je-ops".to_string()
    let matched = apply_match(line, je)
    println!("{} {}", matched.status, matched.matched_journal_entry_id)
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("matched")
        && result.contains("seed-je-ops")
        && !result.contains("error[E0308]")
        && !result.contains("expected `String`, found `&String`");
    assert!(
        ok,
        "owned string helper args should codegen without & borrow. Got:\n{}",
        result
    );
}
