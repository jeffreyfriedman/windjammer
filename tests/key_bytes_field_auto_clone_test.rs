#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Moving `Vec<u8>` field off a struct twice must auto-clone (E0507 / E0382).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn key_bytes_field_reuse_auto_clones() {
    let source = r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub fn dup_bytes(key: Key) -> (Vec<u8>, Vec<u8>) {
    let a = key.bytes
    let b = key.bytes
    (a, b)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains(".bytes.clone()") || generated.contains("bytes.clone()"),
        "second bytes access must auto-clone.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}

#[test]
fn key_bytes_behind_borrowed_param_auto_clones_at_extern_call() {
    let source = r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

extern fn ffi_write(key_bytes: Vec<u8>)

pub struct Writer {}

impl Writer {
    pub fn write_key(self, key: Key) {
        ffi_write(key.bytes)
    }
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("key.bytes.clone()"),
        "field moved from &Key must auto-clone for owned Vec<u8> extern param.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
