#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Cross-type method call: callee takes &Key; caller must pass `key`, not `key.clone()`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn engine_get_on_field_receiver_passes_borrowed_key() {
    let source = r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn get(self, key: Key) -> i32 {
        0
    }
}

pub struct LsmEngine {
    pub engine: MemoryEngine,
}

impl LsmEngine {
    pub fn get(self, key: Key) -> i32 {
        self.engine.get(key)
    }
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("self.engine.get(key)") && !generated.contains("self.engine.get(key.clone())"),
        "MemoryEngine::get expects &Key — pass borrowed key, not owned clone.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}

#[test]
fn nested_self_field_move_auto_clones_all_non_copy_fields() {
    let source = r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub struct KeyRange {
    pub start: Key,
    pub end: Key,
}

impl KeyRange {
    pub fn contains(self, key: Key) -> bool {
        let a = key.bytes
        let lo = self.start.bytes
        let hi = self.end.bytes
        vec_gte(a, lo) && vec_lt(a, hi)
    }
}

fn vec_lt(a: Vec<u8>, b: Vec<u8>) -> bool {
    a.len() < b.len()
}

fn vec_gte(a: Vec<u8>, b: Vec<u8>) -> bool {
    !vec_lt(a, b)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("self.end.bytes.clone()"),
        "moving non-Copy field off &self must auto-clone self.end.bytes.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
