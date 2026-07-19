#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Non-Copy params used in multiple calls within one function must auto-clone on reuse.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn txn_param_reused_across_trait_calls_auto_clones() {
    let source = r#"
pub struct Txn {
    pub label: string,
}

pub trait StorageEngine {
    fn get(self, txn: Txn, key: string) -> Option<string>
}

pub struct MemoryEngine {
    pub data: Vec<string>,
}

impl StorageEngine for MemoryEngine {
    fn get(self, txn: Txn, key: string) -> Option<string> {
        None
    }
}

pub fn read_twice(engine: MemoryEngine, txn: Txn) -> (Option<string>, Option<string>) {
    let a = engine.get(txn, "a")
    let b = engine.get(txn, "b")
    (a, b)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("txn.clone()"),
        "second get() must auto-clone txn after first call moves it.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
