#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Dogfooding: object_pool.wj `Vec::with_capacity(capacity)` with `capacity: usize`
/// must not emit invalid `capacity as usize.clone()`.
#[test]
fn test_usize_with_capacity_no_cast_clone() {
    let source = r#"
pub struct ObjectPool<T> {
    available: Vec<T>,
    capacity: usize,
}

impl<T> ObjectPool<T> {
    pub fn new(capacity: usize) -> ObjectPool<T> {
        ObjectPool {
            available: Vec::with_capacity(capacity),
            capacity: capacity,
        }
    }
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        !generated.contains("as usize.clone()"),
        "must not append .clone() to cast expression. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("Vec::with_capacity(capacity)"),
        "usize arg should pass through without cast. Generated:\n{}",
        generated
    );
}
