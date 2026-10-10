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

//! WDB-176: loop-bound struct field `.clone()` into demoted `&str` must borrow.
//!
//! Product residual after tip-out sync (~12× `&str`←`String`), tip-out still RED:
//!   `cross_signal_find_bm25(hits, v.node_id.clone())` while `doc_id: &str`
//!   `cross_signal_find_graph(hits, h.id.clone())` while id formal is `&str`
//!
//! Distinct from WDB-166 (Emit struct field into Cap sql). Signature-driven.
//! Prefer tip greens over downstream.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod find
pub mod fuse
"#;

const FIND: &str = r#"
pub struct Hit {
    pub id: string,
    pub score: f64,
}

/// Read-only id probe — tip demotes to `&str` (product cross_signal_find_bm25).
pub fn find_by_id(hits: Vec<Hit>, doc_id: string) -> f64 {
    let mut out = 0.0
    for h in hits {
        if h.id == doc_id {
            out = h.score
        }
    }
    out
}
"#;

const FUSE: &str = r#"
use crate::find::Hit
use crate::find::find_by_id

pub struct Node {
    pub node_id: string,
    pub vec_score: f64,
}

pub fn fuse_nodes(hits: Vec<Hit>, nodes: Vec<Node>) -> f64 {
    let mut total = 0.0
    for v in nodes {
        // Product: find_bm25(…, v.node_id.clone()) into demoted &str
        let kw = find_by_id(hits.clone(), v.node_id.clone())
        total = total + kw + v.vec_score
    }
    total
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("find.wj", FIND);
    test.add_file("fuse.wj", FUSE);
    test
}

#[test]
fn module_file_loop_field_clone_into_demoted_str_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let find_rs = map.get("find.rs").expect("find.rs");
    let fuse_rs = map.get("fuse.rs").expect("fuse.rs");

    eprintln!("find.rs:\n{find_rs}\nfuse.rs:\n{fuse_rs}");

    let demoted = {
        let i = find_rs.find("fn find_by_id").unwrap_or(0);
        let sl = &find_rs[i..find_rs.len().min(i + 160)];
        sl.contains("doc_id: &str") || sl.contains("doc_id:&str")
    };
    let bad_clone = fuse_rs.contains("v.node_id.clone()")
        && (fuse_rs.contains("find_by_id(hits.clone(), v.node_id.clone()")
            || fuse_rs.contains("find_by_id("));
    let good_borrow = fuse_rs.contains("&v.node_id")
        || fuse_rs.contains("find_by_id(hits.clone(), &v.node_id")
        || fuse_rs.contains("find_by_id(&hits, &v.node_id");

    if demoted && bad_clone && !good_borrow {
        panic!(
            "demoted &str received loop field .clone(). \
             Product: cross_signal_find_bm25(…, v.node_id.clone()). Got:\n{fuse_rs}\n{find_rs}"
        );
    }

    if demoted {
        assert!(
            good_borrow || !bad_clone,
            "demoted &str must borrow loop field. Got:\n{fuse_rs}"
        );
    }
}
