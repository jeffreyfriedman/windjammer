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
    feature = "integration_tests",
))]

//! P3.321: `for x in parent.field` then use `parent` must not partial-move the Vec field.
//!
//! LedgerKit tip api-check: `for a in payment.allocations` then
//! `allocations_sum_valid(payment, …)` → WJ0007 borrow of partially moved `payment`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub struct Alloc {
    amount: int,
}

pub struct Payment {
    allocations: Vec<Alloc>,
    total: int,
}

fn sum_check(payment: Payment, drafts: Vec<int>) -> Result<int, string> {
    let mut s = payment.total
    for d in drafts {
        s = s + d
    }
    Ok(s)
}

pub fn rebuild(payment: Payment) -> Result<int, string> {
    let mut drafts: Vec<int> = vec![]
    for a in payment.allocations {
        drafts.push(a.amount)
    }
    sum_check(payment, drafts)
}
"#;

#[test]
fn module_file_for_in_struct_vec_field_must_not_partial_move_parent() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "wj build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    if !check.status.success() {
        eprintln!("P3.321 RED (partial move after for-in field):\n{err}");
    }
    assert!(
        check.status.success(),
        "RED P3.321: for-in struct Vec field must not partial-move parent for later use:\n{err}"
    );
}
