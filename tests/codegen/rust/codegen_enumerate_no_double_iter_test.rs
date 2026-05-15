/// TDD Test: .enumerate() should not produce double .iter().iter().enumerate()
///
/// Bug: When Windjammer source has `items.iter().enumerate()`, the codegen
/// processes `.iter()` first (producing `items.iter()`), then sees `.enumerate()`
/// and blindly wraps it with `.iter().enumerate()`, resulting in
/// `items.iter().iter().enumerate()` which is incorrect.
///
/// The fix: if the object already ends with `.iter()`, `.iter_mut()`, or
/// `.into_iter()`, skip adding the extra `.iter()` prefix.
///
/// Discovered via: codegen_loops_comprehensive_tests::test_enumerate_basic
#[path = "../../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_iter_enumerate_no_double_iter() {
    // items.iter().enumerate() should NOT produce items.iter().iter().enumerate()
    let code = test_utils::compile_single(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    for (i, item) in items.iter().enumerate() {
        println("{}: {}", i, item)
    }
}
"#,
    );

    // Should NOT contain .iter().iter()
    assert!(
        !code.contains(".iter().iter()"),
        "Double .iter().iter() detected! Generated:\n{}",
        code
    );

    // Should contain .iter().enumerate() (single .iter())
    assert!(
        code.contains(".iter().enumerate()"),
        "Expected .iter().enumerate() in output. Generated:\n{}",
        code
    );
}

#[test]
fn test_vec_enumerate_adds_single_iter() {
    // items.enumerate() (no explicit .iter()) should produce items.iter().enumerate()
    let code = test_utils::compile_single(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    for (i, item) in items.enumerate() {
        println("{}: {}", i, item)
    }
}
"#,
    );

    // Should contain exactly one .iter().enumerate()
    assert!(
        code.contains(".iter().enumerate()"),
        "Expected .iter().enumerate() for bare .enumerate(). Generated:\n{}",
        code
    );

    // Should NOT contain .iter().iter()
    assert!(
        !code.contains(".iter().iter()"),
        "Double .iter().iter() detected! Generated:\n{}",
        code
    );
}
