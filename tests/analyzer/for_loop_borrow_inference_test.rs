/// TDD Tests: Automatic for-loop borrow inference (v0.41.0)
///
/// THE WINDJAMMER PHILOSOPHY:
/// Users write `for item in collection` — the compiler figures out
/// whether to borrow or consume the collection.
///
/// Rules:
/// - If the collection is used after the loop → auto-insert `&` (borrow)
/// - If the loop body mutates items → auto-insert `&mut`
/// - If the collection is NOT used after → allow move (consume)
/// - Copy types and ranges are unaffected (no borrow needed)
#[path = "../common/test_utils.rs"]
mod test_utils;

/// Compile .wj source and return the generated Rust code
// ==========================================
// Collection used after loop → auto-borrow
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_auto_borrows_when_collection_used_after() {
    let generated = test_utils::compile_single(
        r#"
fn main() {
    let items: Vec<int> = Vec::new()
    for item in items {
        println("{}", item)
    }
    let n = items.len()
}
"#,
    );

    assert!(
        generated.contains("for item in &items")
            || generated.contains("for item in & items")
            || generated.contains("for item in items.iter()"),
        "Expected auto-borrow `&items` or `.iter()` when collection is used after loop.\nGenerated:\n{}",
        generated
    );
}

// ==========================================
// Collection NOT used after loop → consume
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_consumes_when_collection_not_used_after() {
    let generated = test_utils::compile_single(
        r#"
fn main() {
    let items: Vec<int> = Vec::new()
    for item in items {
        println("{}", item)
    }
}
"#,
    );

    // The collection is NOT used after the loop, so no borrow needed
    // Should generate: `for item in items` (consume/move)
    assert!(
        !generated.contains("for item in &items")
            && !generated.contains("for item in items.iter()"),
        "Should NOT add borrow when collection is not used after the loop.\nGenerated:\n{}",
        generated
    );
}

// ==========================================
// Ranges should NOT be affected
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_range_not_affected() {
    let generated = test_utils::compile_single(
        r#"
fn main() {
    for i in 0..10 {
        println("{}", i)
    }
}
"#,
    );

    // Ranges are not collections — no borrow needed
    assert!(
        generated.contains("for i in 0..10")
            || generated.contains("for i in 0i64..10i64")
            || generated.contains("for i in 0 ..10")
            || generated.contains("0..10"),
        "Range for-loops should not be modified.\nGenerated:\n{}",
        generated
    );
}

// ==========================================
// Field access iteration (already handled)
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_field_access_still_borrows() {
    let generated = test_utils::compile_single(
        r#"
struct Game {
    items: Vec<int>
}

impl Game {
    fn print_items(self) {
        for item in self.items {
            println("{}", item)
        }
    }
}
"#,
    );

    // Field access should already be borrowed (existing behavior)
    assert!(
        generated.contains("&self.items") || generated.contains("self.items.iter()"),
        "Field access iteration should be borrowed.\nGenerated:\n{}",
        generated
    );
}
