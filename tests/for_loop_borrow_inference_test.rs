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
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Compile .wj source and return the generated Rust code
fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler stderr:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    })
}

// ==========================================
// Collection used after loop → auto-borrow
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_auto_borrows_when_collection_used_after() {
    let generated = compile_and_get_rust(
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

    // The compiler should auto-insert `&` before `items` in the for loop
    // because `items` is used after the loop (items.len())
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
    let generated = compile_and_get_rust(
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
    let generated = compile_and_get_rust(
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
    let generated = compile_and_get_rust(
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
