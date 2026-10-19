use windjammer_runtime::test::*;
/// Test parameterized tests with @test_cases decorator
#[inline]
fn add_numbers_impl(a: i64, b: i64, expected: i64) {
    let result = a + b;
    assert_eq!(result, expected);
}

#[test]
fn add_numbers_case_0() {
    add_numbers_impl(5, 3, 8);
}

#[test]
fn add_numbers_case_1() {
    add_numbers_impl(10, -5, 5);
}

#[test]
fn add_numbers_case_2() {
    add_numbers_impl(0, 0, 0);
}

#[test]
fn add_numbers_case_3() {
    add_numbers_impl(-3, -7, -10);
}



#[inline]
fn string_length_impl(input: &str, expected: i64) {
    let len = input.len() as i64;
    assert_eq!(len, expected);
}

#[test]
fn string_length_case_0() {
    string_length_impl("hello", 5);
}

#[test]
fn string_length_case_1() {
    string_length_impl("world", 5);
}

#[test]
fn string_length_case_2() {
    string_length_impl("", 0);
}



#[inline]
fn bool_negation_impl(input: bool, expected: bool) {
    assert_eq!(!input, expected);
}

#[test]
fn bool_negation_case_0() {
    bool_negation_impl(true, false);
}

#[test]
fn bool_negation_case_1() {
    bool_negation_impl(false, true);
}



