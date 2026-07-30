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

//! FAILING REPRO (dogfood): owned Vec helper args must not force callers to pre-reduce.
//! Documents why bank recon domain takes register aggregates instead of Vec<BankLineView>.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn owned_vec_helper_should_take_owned_vec() {
    let source = r##"
pub struct BankLineView {
    amount_cents: int,
    cleared: bool,
}

pub fn sum_uncleared(lines: Vec<BankLineView>) -> int {
    let mut total = 0
    let mut i = 0
    while i < lines.len() {
        if !lines[i].cleared {
            total = total + lines[i].amount_cents
        }
        i = i + 1
    }
    total
}

fn main() {
    let lines = vec![
        BankLineView { amount_cents: -4500, cleared: false },
        BankLineView { amount_cents: 150000, cleared: true },
    ]
    let n = sum_uncleared(lines)
    println!("{}", n)
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("-4500")
        || (result.contains("fn main")
            && !result.contains("error[E0308]")
            && !result.contains("expected `Vec")
            && !result.contains("found `&Vec"));
    assert!(
        ok,
        "owned Vec helper args should codegen without &Vec borrow. Got:\n{}",
        result
    );
}
