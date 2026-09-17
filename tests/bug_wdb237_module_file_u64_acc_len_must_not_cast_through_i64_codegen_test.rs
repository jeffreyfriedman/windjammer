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

//! WDB-237 multipass codegen: u64 acc += len() as u64 must not cast through i64.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = r#"
pub struct Graph {
    countries: Vec<int>,
    cities: Vec<int>,
}

impl Graph {
    pub fn vertex_count(self) -> u64 {
        let mut total: u64 = 0
        total = total + (self.countries.len() as u64)
        total = total + (self.cities.len() as u64)
        total
    }
}
"#;

#[test]
fn wdb237_codegen_u64_acc_len_must_not_cast_through_i64() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let bad = rs.contains("as u64 as i64") || rs.contains("len() as u64 as i64");
    eprintln!("WDB-237 codegen bad={bad}\n{rs}");
    assert!(!bad, "WDB-237: u64 += len() as u64 must not cast through i64:\n{rs}");
    assert!(ok, "WDB-237 fixture must cargo-check:\n{rs}");
}
