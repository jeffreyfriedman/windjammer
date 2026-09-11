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

//! WDB-128: multipass demotes method first-arg struct formal to `&T` but call sites still
//! pass `view.clone()` owned → E0308 (`expected &GraphAdjacencyView, found GraphAdjacencyView`).
//!
//! WindjammerDB CQ-C5 (~56× Graph* residual before dogfood):
//!   `port.out_degree(view.clone(), vertex)` while formal is `&GraphAdjacencyView`
//!
//! Distinct from WDB-125 (free-fn). This gate is **method** call sites.
//! Expected: `port.out_degree(&view.clone(), …)` / `port.out_degree(&view, …)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod view
pub mod query
"#;

const VIEW: &str = r#"
pub struct AdjView {
    pub vertex_count: u64,
}

pub struct AdjPort {
    pub tag: u64,
}

/// Free read helper — paired with methods to pressure multipass demotion of `AdjView`.
pub fn touch(view: AdjView) -> u64 {
    view.vertex_count
}

impl AdjPort {
    pub fn out_degree(self, view: AdjView, vertex: i64) -> u32 {
        let _ = vertex
        view.vertex_count as u32
    }

    pub fn neighbor_at(self, view: AdjView, vertex: i64, index: u32) -> i64 {
        let _ = view
        let _ = index
        vertex
    }
}
"#;

const QUERY: &str = r#"
use crate::view::AdjView
use crate::view::AdjPort
use crate::view::touch

/// Reuse `view` across free-fn + methods — demotes `AdjView` formals to `&AdjView`.
pub fn walk(port: AdjPort, view: AdjView, vertex: i64) -> u32 {
    let _ = touch(view.clone())
    let d = port.out_degree(view.clone(), vertex)
    let _ = port.neighbor_at(view.clone(), vertex, 0)
    let _ = touch(view.clone())
    d
}

pub fn cap() -> u32 {
    walk(AdjPort { tag: 1 }, AdjView { vertex_count: 3 }, 7)
}
"#;

fn wdb128_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("view.wj", VIEW);
    test.add_file("query.wj", QUERY);
    test
}

#[test]
fn wdb128_module_file_demoted_method_struct_formal_must_borrow_clone_call_sites() {
    let test = wdb128_fixture();
    let map = test
        .compile()
        .expect("WDB-128 multipass compile should succeed (codegen may still be wrong)");
    let view_rs = map.get("view.rs").expect("view.rs");
    let query_rs = map.get("query.rs").expect("query.rs");

    let demoted = view_rs.contains("view: &AdjView") || view_rs.contains("view: & AdjView");
    let method_borrows = query_rs.contains("out_degree(&view")
        || query_rs.contains("neighbor_at(&view")
        || query_rs.contains("out_degree(&view.clone()")
        || query_rs.contains("neighbor_at(&view.clone()");
    let method_bad = (query_rs.contains("out_degree(view.clone()")
        || query_rs.contains("neighbor_at(view.clone()"))
        && !method_borrows;

    eprintln!("WDB-128 view.rs:\n{view_rs}\nquery.rs:\n{query_rs}");
    eprintln!("demoted={demoted} method_borrows={method_borrows} method_bad={method_bad}");

    if demoted && method_bad {
        panic!(
            "WDB-128 RED: demoted method &AdjView + owned .clone() call sites must borrow. \
             Product: GraphAdjacencyPort::out_degree/neighbor_at (CQ-C5 Graph* residual)."
        );
    }

    // Vacuous GREEN if tip never demotes this fixture — still cargo-check for smoke.
    if !demoted {
        eprintln!("WDB-128 tip did not demote AdjView in this fixture; cargo-check smoke only.");
    }

    test.cargo_check().expect(
        "WDB-128: emit must cargo-check. Product dogfood borrows method &T formals until tip demotes+borrows.",
    );
}
