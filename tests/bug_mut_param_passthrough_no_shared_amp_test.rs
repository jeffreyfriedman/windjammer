//! When a formal is emitted as `&mut T` (field take/restore), passing it to another
//! `&mut T` callee must use the bare binding — not `&param` (`&&mut T`, E0308).
//!
//! Dogfood: `graph_dense_csr_take_in_edges(&csr)` inside
//! `graph_pagerank_run_dense_pull_parallel(csr: &mut DenseCsr)`.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn mut_param_passthrough_must_not_prefix_shared_amp() {
    let source = r#"
pub struct DenseCsr {
    pub offsets: Vec<int>,
    pub neighbors: Vec<int>,
}

pub struct Edges {
    pub offsets: Vec<int>,
    pub neighbors: Vec<int>,
}

pub fn take_in_edges(csr: DenseCsr) -> Edges {
    let empty_o: Vec<int> = Vec::new()
    let empty_n: Vec<int> = Vec::new()
    let out = Edges { offsets: csr.offsets, neighbors: csr.neighbors }
    csr.offsets = empty_o
    csr.neighbors = empty_n
    out
}

pub fn restore_in_edges(csr: DenseCsr, offsets: Vec<int>, neighbors: Vec<int>) {
    csr.offsets = offsets
    csr.neighbors = neighbors
}

pub fn run_parallel(csr: DenseCsr) -> int {
    let edges = take_in_edges(csr)
    restore_in_edges(csr, edges.offsets, edges.neighbors)
    0
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("fn run_parallel(csr: &mut DenseCsr)")
            || generated.contains("fn run_parallel(mut csr: &mut DenseCsr)"),
        "run_parallel must emit &mut DenseCsr formal, got:\n{generated}"
    );
    assert!(
        generated.contains("take_in_edges(csr)")
            && !generated.contains("take_in_edges(&csr)")
            && !generated.contains("take_in_edges(&mut csr)")
            && !generated.contains("take_in_edges(csr.clone())"),
        "must reborrow bare csr into &mut take_in_edges (no & / &mut / clone). Got:\n{generated}"
    );
    assert!(
        generated.contains("restore_in_edges(csr,")
            && !generated.contains("restore_in_edges(&csr,")
            && !generated.contains("restore_in_edges(&mut csr,")
            && !generated.contains("restore_in_edges(csr.clone(),"),
        "must reborrow bare csr into &mut restore_in_edges. Got:\n{generated}"
    );
}

#[test]
fn mut_param_method_passthrough_must_not_prefix_shared_amp() {
    let source = r#"
pub struct DenseCsr {
    pub offsets: Vec<int>,
    pub neighbors: Vec<int>,
}

pub struct Host {}

impl Host {
    pub fn take_edges(self, csr: DenseCsr) -> int {
        let empty: Vec<int> = Vec::new()
        let n = csr.offsets.len() as int
        csr.offsets = empty
        n
    }

    pub fn run(self, csr: DenseCsr) -> int {
        self.take_edges(csr)
    }
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("csr: &mut DenseCsr"),
        "take_edges/run must emit &mut DenseCsr formals, got:\n{generated}"
    );
    assert!(
        generated.contains("self.take_edges(csr)")
            || generated.contains("Host::take_edges(self, csr)"),
        "method passthrough must pass bare csr, got:\n{generated}"
    );
    assert!(
        !generated.contains("self.take_edges(&csr)")
            && !generated.contains("self.take_edges(&mut csr)")
            && !generated.contains("self.take_edges(csr.clone())"),
        "must not emit &csr / &mut csr / clone into &mut method formal. Got:\n{generated}"
    );
}

#[test]
fn mut_formal_shared_borrow_callee_must_not_prefix_amp() {
    let source = r#"
pub struct DenseCsr {
    pub offsets: Vec<int>,
    pub neighbors: Vec<int>,
}

pub struct LccEngine {
    pub total_triangles: int,
}

pub fn run_dense(csr: DenseCsr) -> LccEngine {
    LccEngine { total_triangles: csr.offsets.len() as int }
}

pub fn execute(csr: DenseCsr) -> int {
    let eng = run_dense(csr)
    let empty: Vec<int> = Vec::new()
    csr.offsets = empty
    eng.total_triangles
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("fn execute(csr: &mut DenseCsr)")
            || generated.contains("fn execute(mut csr: &mut DenseCsr)"),
        "execute must emit &mut DenseCsr formal, got:\n{generated}"
    );
    assert!(
        generated.contains("run_dense(csr)")
            && !generated.contains("run_dense(&csr)")
            && !generated.contains("run_dense(&mut csr)"),
        "&mut formal into &T callee must reborrow bare csr (no stacked &). Got:\n{generated}"
    );
}
