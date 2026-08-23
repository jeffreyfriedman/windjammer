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

//! WDB-104: methods that mutate `self` fields must codegen `mut self`, not owned
//! `self` (Phase 148 `GraphWritePort::append_edge` dogfood).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn wdb104_field_mutating_method_must_emit_mut_self() {
    let source = r#"
pub struct EdgeBuffer {
    pub edges: Vec<i64>,
    pub pending: u64,
}

pub struct WritePort {
    pub buffer: EdgeBuffer,
}

impl WritePort {
    pub fn append_edge(self, edge: i64) -> WritePort {
        self.buffer.edges.push(edge)
        self.buffer.pending = self.buffer.pending + 1
        self
    }
}
"#;

    let rs = test_utils::compile_single(source);
    assert!(
        rs.contains("append_edge(mut self") || rs.contains("append_edge(&mut self"),
        "WDB-104: mutating method must take mut self. Got:\n{rs}"
    );
    assert!(
        !rs.contains("pub fn append_edge(self, edge: i64)") || rs.contains("mut self"),
        "WDB-104: must not use immutable self when mutating fields. Got:\n{rs}"
    );
}
