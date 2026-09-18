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

//! WDB-285 class: reused Vec into owned Vec formal must `.clone()`, never `&samples`.

#[path = "common/test_utils.rs"]
mod test_utils;

const SRC: &str = r#"
fn median_pair(samples: Vec<u64>) -> u64 {
    if samples.len() == 0 {
        return 0
    }
    samples[0]
}

fn workload_verdict(workload_id: u32, samples: Vec<u64>) -> u64 {
    let _ = workload_id
    median_pair(samples)
}

fn claim_cap(samples: Vec<u64>) -> u64 {
    let quiet = workload_verdict(2, samples)
    let _ = samples.len()
    quiet
}

fn main() {}
"#;

#[test]
fn reused_vec_second_arg_into_owned_callee_must_clone_not_reborrow() {
    let (rs, ok) = test_utils::compile_single_check(SRC);
    assert!(
        !rs.contains(", &samples)"),
        "must not reborrow into owned Vec formal. Generated:\n{rs}"
    );
    assert!(
        rs.contains("samples.clone()"),
        "expected samples.clone() into owned workload_verdict when reused after. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
