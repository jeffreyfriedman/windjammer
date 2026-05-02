// TDD Test: Remove unnecessary type casts in comparisons
//
// Bug: while i < (vec.len() as i64) generates mismatched types
// Root cause: User added `as i64` manually, but `i` is i32
// Rust: `i32 < i64` fails
//
// Fix: Either remove `as i64` or infer `i` as i64

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_len_comparison_no_explicit_cast() {
    let source = r#"
fn test(items: Vec<i32>) {
    let mut i = 0
    while i < items.len() {
        println!("{}", i)
        i = i + 1
    }
}
"#;

    let rust_code = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", rust_code);
    println!("Len comparison test PASSED");
}

#[test]
fn test_len_comparison_with_explicit_i64_cast() {
    let source = r#"
fn test(items: Vec<i32>) {
    let mut i = 0
    while i < (items.len() as i64) {
        println!("{}", i)
        i = i + 1
    }
}
"#;

    let rust_code = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", rust_code);
    println!("Len comparison with i64 cast test PASSED");
}
