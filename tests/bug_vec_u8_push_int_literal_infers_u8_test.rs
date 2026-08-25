//! Integer literals pushed into `Vec<u8>` must infer `u8`, not `i64`.
//!
//! Ecosystem `wj-base64`:
//! ```
//! let mut bytes = Vec::new()
//! bytes.push(0)
//! bytes.push(255)
//! encode_bytes(bytes)
//! ```
//! Codegen keeps the vec as `Vec<i64>` → E0308 vs `Vec<u8>` formal.

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
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vec_u8_push_int_literal_infers_u8() {
    let source = r#"
pub fn sample() -> Vec<u8> {
    let mut bytes = Vec::new()
    bytes.push(0)
    bytes.push(255)
    bytes.push(10)
    bytes
}

pub fn take_bytes(data: Vec<u8>) -> int {
    data.len()
}

pub fn roundtrip_len() -> int {
    take_bytes(sample())
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "Vec<u8> push of 0/255/10 must infer u8, got:\n{generated}"
    );
}
