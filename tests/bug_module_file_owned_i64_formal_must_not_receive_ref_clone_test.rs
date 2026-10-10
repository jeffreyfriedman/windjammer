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
))]

//! (`expected i64, found &i64`).
//!
//!   `graph_adjacency_collect_unique_neighbors(…, &v.clone())` while `vertex: i64`
//!   `graph_cdlp_collect_neighbor_labels(…, &v.clone())` while `vertex: i64`
//!
//! The compiler must pass Copy locals by value (or `*v` / `v.clone()` without `&`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod adj
pub mod run
"#;

const ADJ: &str = r#"
pub fn collect_at(vertices: Vec<i64>, vertex: i64) -> i64 {
    let mut n = 0
    for v in vertices {
        if v == vertex {
            n = n + 1
        }
    }
    n
}
"#;

const RUN: &str = r#"
use crate::adj::collect_at

/// Loop reuses `vertices` + Copy `v` into owned i64 formal.
pub fn sum_hits(vertices: Vec<i64>) -> i64 {
    let mut total: i64 = 0
    for v in vertices {
        total = total + collect_at(vertices, v)
    }
    total
}

pub fn cap() -> i64 {
    sum_hits(vec![1, 2, 1])
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("adj.wj", ADJ);
    test.add_file("run.wj", RUN);
    test
}

#[test]
fn module_file_owned_i64_formal_must_not_receive_ref_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let run_rs = map.get("run.rs").expect("run.rs");
    let adj_rs = map.get("adj.rs").expect("adj.rs");

    let formal_owned_i64 = {
        let i = adj_rs.find("fn collect_at").unwrap_or(0);
        let sl = &adj_rs[i..adj_rs.len().min(i + 160)];
        sl.contains("vertex: i64") && !sl.contains("vertex: &i64")
    };
    let bad_ref_clone = run_rs.contains("collect_at(")
        && (run_rs.contains(", &v.clone())")
            || run_rs.contains(", &v)")
            || run_rs.contains(",&(v.clone())"));
    let good = run_rs.contains(", v)")
        || run_rs.contains(", v.clone())")
        || run_rs.contains(", *v)");

    eprintln!("adj.rs:\n{adj_rs}\nrun.rs:\n{run_rs}");
    eprintln!("formal_owned_i64={formal_owned_i64} bad_ref_clone={bad_ref_clone} good={good}");

    if formal_owned_i64 && bad_ref_clone && !good {
        panic!(
            "RED: owned i64 formal must not receive &v / &v.clone(). \
             "
        );
    }

    test.cargo_check().expect(
        "Copy i64 into owned i64 formal must cargo-check (pass by value).",
    );
}
