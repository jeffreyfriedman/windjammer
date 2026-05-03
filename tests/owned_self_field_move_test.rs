/// TDD Tests: Ownership inference for non-Copy field moves (v0.41.0)
///
/// When a method returns `self.field` where the field is a non-Copy type (e.g. String),
/// the compiler must infer owned `self` (not `&self`), because you can't move a field
/// out of a borrowed reference.
///
/// This was a known issue from v0.40.0.
#[path = "test_utils.rs"]
mod test_utils;

// ==========================================
// Return self.field (non-Copy) → owned self
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_string_field_infers_owned_self() {
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

    // The method returns self.content (a String, non-Copy).
    // Moving a field out of self requires owned self, not &self.
    assert!(
        generated.contains("fn get_content(self) -> String"),
        "Expected owned `self` when returning non-Copy field. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn get_content(&self)"),
        "Should NOT use &self when returning non-Copy field. Got:\n{}",
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
// Return self.field (Vec, non-Copy) → owned self
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_self_vec_field_infers_owned_self() {
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

    assert!(
        generated.contains("fn take_items(self) -> Vec<i64>"),
        "Expected owned `self` when returning non-Copy Vec field. Got:\n{}",
        generated
    );
}
