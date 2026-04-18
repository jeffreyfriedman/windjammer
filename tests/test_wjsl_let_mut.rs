/// TDD: WJSL `let mut` support (consistency with Windjammer)
///
/// WJSL should use `let mut` for mutable variables, matching Windjammer syntax.
/// `var` is a WGSL-ism that leaked into WJSL. Both should work during migration,
/// but `let mut` should be the preferred idiom.
///
/// In WGSL output, `let mut` transpiles to `var` (WGSL's mutable variable syntax).

use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_let_mut_basic() {
    let source = r#"
fn test_mut() -> f32 {
    let mut x = 1.0;
    x = x + 1.0;
    return x;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let mut should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("var x"),
        "WGSL output should use 'var' for mutable. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_let_mut_with_type_annotation() {
    let source = r#"
fn test_mut_typed() -> u32 {
    let mut count: u32 = 0u;
    count = count + 1u;
    return count;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let mut with type annotation should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("var count"),
        "WGSL should use 'var' for let mut. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_let_mut_in_loop() {
    let source = r#"
fn accumulate() -> f32 {
    let mut sum = 0.0;
    for (var i = 0u; i < 10u; i = i + 1u) {
        sum = sum + 1.0;
    }
    return sum;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let mut in loop should work. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_for_let_mut_initializer() {
    let source = r#"
fn count_up() -> u32 {
    let mut total = 0u;
    for (let mut i = 0u; i < 10u; i = i + 1u) {
        total = total + 1u;
    }
    return total;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "for (let mut ...) should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("for (var i"),
        "for (let mut i) should transpile to for (var i). Got:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("var total"),
        "let mut total should transpile to var total. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_var_still_works() {
    let source = r#"
fn test_var() -> f32 {
    var x = 1.0;
    x = x + 1.0;
    return x;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "var should still work (backward compat). Error: {:?}",
        result.err()
    );
}

#[test]
fn test_let_immutable_still_works() {
    let source = r#"
fn test_let() -> f32 {
    let x = 1.0;
    return x;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "immutable let should still work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("let x"),
        "WGSL should keep 'let' for immutable. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_let_mut_hash_function() {
    let source = r#"
fn hash(input: u32) -> u32 {
    let mut h = input;
    h = h ^ (h >> 16u);
    h = h * 0x45d9f3bu;
    h = h ^ (h >> 16u);
    return h;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "let mut in hash function should work. Error: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("var h"),
        "WGSL should use 'var' for let mut. Got:\n{}",
        wgsl
    );
}
