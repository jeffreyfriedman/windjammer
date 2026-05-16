#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Tests: Ownership inference for self field access patterns (v0.41.0+)
///
/// DESIGN DECISION (Windjammer Way): When a method returns `self.field` where
/// the field is a non-Copy type (e.g. String), the compiler infers `&self` and
/// auto-clones the returned field. This prevents cascading E0382 "use of moved
/// value" errors at callsites — the getter can be called multiple times safely.
///
/// For methods that truly consume self (builder pattern, match on self, etc.),
/// owned self is inferred through other mechanisms.
#[path = "../common/test_utils.rs"]
mod test_utils;

// ==========================================
// Return self.field (non-Copy) → &self + clone
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_string_field_infers_borrowed_self_with_clone() {
    let generated = test_utils::compile_single(
        r#"
struct Text {
    content: string
}

impl Text {
    fn get_content(self) -> string {
        self.content
    }
}
"#,
    );

    // Getter returning non-Copy field uses &self + auto-clone.
    // This prevents E0382 at callsites (can call get_content() multiple times).
    assert!(
        generated.contains("fn get_content(&self) -> String"),
        "Expected `&self` with auto-clone for getter returning non-Copy field. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("self.content.clone()"),
        "Expected auto-clone on non-Copy field return. Got:\n{}",
        generated
    );
}

// ==========================================
// Return self.field (Copy type) → &self OK
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_copy_field_allows_borrowed_self() {
    let generated = test_utils::compile_single(
        r#"
struct Counter {
    value: int
}

impl Counter {
    fn get_value(self) -> int {
        self.value
    }
}
"#,
    );

    // int is Copy, so reading self.value works fine with &self
    // The compiler can (and should) use &self here for efficiency
    assert!(
        generated.contains("fn get_value(&self) -> i64"),
        "Expected &self when returning Copy field. Got:\n{}",
        generated
    );
}

// ==========================================
// Return self.field (Vec, non-Copy) → &self + clone
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_vec_field_infers_borrowed_self_with_clone() {
    let generated = test_utils::compile_single(
        r#"
struct Container {
    items: Vec<int>
}

impl Container {
    fn take_items(self) -> Vec<int> {
        self.items
    }
}
"#,
    );

    // Same as String: getter returning non-Copy Vec uses &self + clone.
    assert!(
        generated.contains("fn take_items(&self) -> Vec<i64>"),
        "Expected `&self` with auto-clone for getter returning non-Copy Vec field. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("self.items.clone()"),
        "Expected auto-clone on non-Copy Vec field return. Got:\n{}",
        generated
    );
}
