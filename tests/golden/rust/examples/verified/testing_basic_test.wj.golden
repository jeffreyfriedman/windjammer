use windjammer_runtime::test::*;
#[inline]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[inline]
fn is_even(n: i64) -> bool {
    n % 2_i64 == 0_i64
}

#[test]
fn test_add() {
    assert_eq!(add(2_i64, 3_i64), 5_i64);
}

#[test]
fn test_add_negative() {
    assert_eq!(add(-2_i64, -3_i64), -5_i64);
}

#[test]
fn test_is_even() {
    assert!(is_even(4_i64));
    assert!(!is_even(7_i64), "7 is odd");
}

