//! Library multipass: owned Custom formals must not get stale `&arg` / `&self.field`
//! at call sites once defining-module codegen refresh records owned emission.
//!
//! Dogfood (wdb-layers):
//! - `graph_bfs_run_dense(&self.csr, source)` while formal is `csr: DenseCsr`
//! - `graph_sql_physical_exec_with_batch(&host2, …)` while formal is owned host

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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn library_multipass_owned_custom_self_field_must_clone_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "graph/types.wj",
        r#"
pub struct DenseCsr {
    pub offsets: Vec<int>,
    pub neighbors: Vec<int>,
}

pub struct BfsEngine {
    pub visited: int,
}
"#,
    );
    test.add_file(
        "graph/bfs.wj",
        r#"
use crate::graph::types::DenseCsr
use crate::graph::types::BfsEngine

pub fn take_offsets(csr: DenseCsr) -> Vec<int> {
    csr.offsets
}

pub fn run_dense(csr: DenseCsr, source: int) -> BfsEngine {
    let offs = take_offsets(csr)
    BfsEngine { visited: offs.len() as int + source }
}
"#,
    );
    test.add_file(
        "graph/session.wj",
        r#"
use crate::graph::types::DenseCsr
use crate::graph::types::BfsEngine
use crate::graph::bfs::run_dense

pub struct AnalyticsSession {
    pub csr: DenseCsr,
}

impl AnalyticsSession {
    pub fn bfs(self, source: int) -> BfsEngine {
        run_dense(self.csr, source)
    }

    pub fn bfs_twice(self, source: int) -> int {
        let a = run_dense(self.csr, source)
        let b = run_dense(self.csr, source + 1)
        a.visited + b.visited
    }
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("graph/session.rs")
        .or_else(|| map.get("session.rs"))
        .expect("session.rs output");
    assert!(
        !rs.contains("run_dense(&self.csr"),
        "owned DenseCsr formal must not receive &self.csr. Got:\n{rs}"
    );
    assert!(
        rs.contains("run_dense(self.csr.clone()"),
        "owned DenseCsr from &self must clone (E0507 if moved). Got:\n{rs}"
    );
}

#[test]
fn library_multipass_owned_custom_local_must_not_prefix_amp() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sql/types.wj",
        r#"
pub struct QueryHost {
    pub visits: int,
}
"#,
    );
    test.add_file(
        "sql/exec.wj",
        r#"
use crate::sql::types::QueryHost

pub fn exec_with_batch(host: QueryHost) -> int {
    host.visits
}
"#,
    );
    test.add_file(
        "sql/port.wj",
        r#"
use crate::sql::types::QueryHost
use crate::sql::exec::exec_with_batch

pub fn run_query() -> int {
    let host2 = QueryHost { visits: 3 }
    exec_with_batch(host2)
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("sql/port.rs")
        .or_else(|| map.get("port.rs"))
        .expect("port.rs output");
    assert!(
        !rs.contains("exec_with_batch(&host2)"),
        "owned QueryHost formal must not receive &host2. Got:\n{rs}"
    );
    assert!(
        rs.contains("exec_with_batch(host2)"),
        "expected move of host2 into owned formal. Got:\n{rs}"
    );
}
