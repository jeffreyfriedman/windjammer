use windjammer_runtime::test::*;
#[inline]
fn square(x: i64) -> i64 {
    x * x
}

#[test]

fn test_square_impl(input: i64, expected: i64) {
    assert_eq!(square(input), expected);
}

#[test]
fn test_square_case_0() {
    test_square_impl(0, 0);
}

#[test]
fn test_square_case_1() {
    test_square_impl(3, 9);
}

#[test]
fn test_square_case_2() {
    test_square_impl(-4, 16);
}

#[test]
fn test_square_case_3() {
    test_square_impl(10, 100);
}



