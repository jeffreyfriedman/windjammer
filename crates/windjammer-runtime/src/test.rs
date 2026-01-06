//! Test framework for Windjammer
//!
//! Provides test primitives similar to Rust's test framework

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

static TESTS_PASSED: AtomicUsize = AtomicUsize::new(0);
static TESTS_FAILED: AtomicUsize = AtomicUsize::new(0);
static CURRENT_TEST: Mutex<String> = Mutex::new(String::new());

/// Assert that a condition is true
pub fn assert(condition: bool, message: &str) {
    if !condition {
        panic!("Assertion failed: {}", message);
    }
}

/// Assert that two values are equal
pub fn assert_eq<T: PartialEq + std::fmt::Debug>(left: T, right: T) {
    if left != right {
        panic!("Assertion failed: {:?} != {:?}", left, right);
    }
}

/// Assert that two values are not equal
pub fn assert_ne<T: PartialEq + std::fmt::Debug>(left: T, right: T) {
    if left == right {
        panic!("Assertion failed: {:?} == {:?}", left, right);
    }
}

// ============================================================================
// ENHANCED ASSERTIONS
// ============================================================================

/// Assert that a value is greater than another
pub fn assert_gt<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if left <= right {
        panic!(
            "assertion failed: left > right\n  left: {:?}\n right: {:?}",
            left, right
        );
    }
}

/// Assert that a value is less than another
pub fn assert_lt<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if left >= right {
        panic!(
            "assertion failed: left < right\n  left: {:?}\n right: {:?}",
            left, right
        );
    }
}

/// Assert that a value is greater than or equal to another
pub fn assert_gte<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if left < right {
        panic!(
            "assertion failed: left >= right\n  left: {:?}\n right: {:?}",
            left, right
        );
    }
}

/// Assert that a value is less than or equal to another
pub fn assert_lte<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if left > right {
        panic!(
            "assertion failed: left <= right\n  left: {:?}\n right: {:?}",
            left, right
        );
    }
}

/// Assert that two floating point values are approximately equal
pub fn assert_approx(actual: f64, expected: f64, epsilon: f64) {
    let diff = (actual - expected).abs();
    if diff > epsilon {
        panic!(
            "assertion failed: values not approximately equal\n  actual: {}\n  expected: {}\n  epsilon: {}\n  diff: {}",
            actual, expected, epsilon, diff
        );
    }
}

/// Assert that two f32 values are approximately equal
pub fn assert_approx_f32(actual: f32, expected: f32, epsilon: f32) {
    let diff = (actual - expected).abs();
    if diff > epsilon {
        panic!(
            "assertion failed: values not approximately equal\n  actual: {}\n  expected: {}\n  epsilon: {}\n  diff: {}",
            actual, expected, epsilon, diff
        );
    }
}

/// Assert that a collection contains an item
pub fn assert_contains<T: PartialEq + std::fmt::Debug>(collection: &[T], item: &T) {
    if !collection.contains(item) {
        panic!(
            "assertion failed: collection doesn't contain item\n  collection: {:?}\n  item: {:?}",
            collection, item
        );
    }
}

/// Assert that a collection has a specific length
pub fn assert_length<T>(collection: &[T], expected: usize) {
    let actual = collection.len();
    if actual != expected {
        panic!(
            "assertion failed: collection length mismatch\n  expected: {}\n  actual: {}",
            expected, actual
        );
    }
}

/// Assert that a collection is empty
pub fn assert_empty<T>(collection: &[T]) {
    if !collection.is_empty() {
        panic!(
            "assertion failed: collection is not empty\n  length: {}",
            collection.len()
        );
    }
}

/// Assert that a collection is not empty
pub fn assert_not_empty<T>(collection: &[T]) {
    if collection.is_empty() {
        panic!("assertion failed: collection is empty");
    }
}

/// Assert that a string contains a substring
pub fn assert_str_contains(string: &str, substring: &str) {
    if !string.contains(substring) {
        panic!(
            "assertion failed: string doesn't contain substring\n  string: \"{}\"\n  substring: \"{}\"",
            string, substring
        );
    }
}

/// Assert that a string starts with a prefix
pub fn assert_starts_with(string: &str, prefix: &str) {
    if !string.starts_with(prefix) {
        panic!(
            "assertion failed: string doesn't start with prefix\n  string: \"{}\"\n  prefix: \"{}\"",
            string, prefix
        );
    }
}

/// Assert that a string ends with a suffix
pub fn assert_ends_with(string: &str, suffix: &str) {
    if !string.ends_with(suffix) {
        panic!(
            "assertion failed: string doesn't end with suffix\n  string: \"{}\"\n  suffix: \"{}\"",
            string, suffix
        );
    }
}

/// Assert that an Option is Some
pub fn assert_is_some<T: std::fmt::Debug>(option: &Option<T>) {
    if option.is_none() {
        panic!("assertion failed: Option is None, expected Some");
    }
}

/// Assert that an Option is None
pub fn assert_is_none<T: std::fmt::Debug>(option: &Option<T>) {
    if let Some(ref value) = option {
        panic!(
            "assertion failed: Option is Some, expected None\n  value: {:?}",
            value
        );
    }
}

/// Assert that a Result is Ok
pub fn assert_is_ok<T: std::fmt::Debug, E: std::fmt::Debug>(result: &Result<T, E>) {
    if let Err(ref e) = result {
        panic!(
            "assertion failed: Result is Err, expected Ok\n  error: {:?}",
            e
        );
    }
}

/// Assert that a Result is Err
pub fn assert_is_err<T: std::fmt::Debug, E: std::fmt::Debug>(result: &Result<T, E>) {
    if let Ok(ref value) = result {
        panic!(
            "assertion failed: Result is Ok, expected Err\n  value: {:?}",
            value
        );
    }
}

/// Assert that a value is in a range
pub fn assert_in_range<T: PartialOrd + std::fmt::Debug>(value: T, min: T, max: T) {
    if value < min || value > max {
        panic!(
            "assertion failed: value not in range\n  value: {:?}\n  min: {:?}\n  max: {:?}",
            value, min, max
        );
    }
}

// ============================================================================
// ADVANCED ASSERTIONS
// ============================================================================

/// Assert that a closure panics
///
/// # Example
/// ```
/// use windjammer_runtime::test::assert_panics;
///
/// assert_panics(|| {
///     panic!("This should panic");
/// });
/// ```
pub fn assert_panics<F: FnOnce() + std::panic::UnwindSafe>(f: F) {
    let result = std::panic::catch_unwind(f);
    if result.is_ok() {
        panic!("assertion failed: expected panic, but function completed successfully");
    }
}

/// Assert that a closure panics with a specific message
///
/// # Example
/// ```
/// use windjammer_runtime::test::assert_panics_with;
///
/// assert_panics_with("division by zero", || {
///     let _ = 1 / 0;
/// });
/// ```
pub fn assert_panics_with<F: FnOnce() + std::panic::UnwindSafe>(expected_msg: &str, f: F) {
    let result = std::panic::catch_unwind(f);
    match result {
        Ok(_) => {
            panic!(
                "assertion failed: expected panic with message '{}', but function completed successfully",
                expected_msg
            );
        }
        Err(err) => {
            // Try to extract the panic message
            let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = err.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic message".to_string()
            };

            if !panic_msg.contains(expected_msg) {
                panic!(
                    "assertion failed: panic message mismatch\n  expected (substring): \"{}\"\n  actual: \"{}\"",
                    expected_msg, panic_msg
                );
            }
        }
    }
}

/// Assert that a Result matches a pattern (Ok or Err)
/// This is a simplified version - full pattern matching requires compiler support
pub fn assert_result_ok<T, E: std::fmt::Debug>(result: &Result<T, E>) {
    if let Err(e) = result {
        panic!("assertion failed: expected Ok(_), got Err({:?})", e);
    }
}

/// Assert that a Result matches Err pattern
pub fn assert_result_err<T: std::fmt::Debug, E>(result: &Result<T, E>) {
    if let Ok(val) = result {
        panic!("assertion failed: expected Err(_), got Ok({:?})", val);
    }
}

/// Assert that two values are deeply equal (same as assert_eq, but explicit name)
pub fn assert_deep_eq<T: PartialEq + std::fmt::Debug>(left: &T, right: &T) {
    if left != right {
        panic!(
            "assertion failed: deep equality check failed\n  left: {:?}\n  right: {:?}",
            left, right
        );
    }
}

/// Mark a test as passed
pub fn pass() {
    TESTS_PASSED.fetch_add(1, Ordering::SeqCst);
}

/// Mark a test as failed
pub fn fail(message: &str) {
    TESTS_FAILED.fetch_add(1, Ordering::SeqCst);
    panic!("Test failed: {}", message);
}

/// Get the number of tests passed
pub fn passed_count() -> usize {
    TESTS_PASSED.load(Ordering::SeqCst)
}

/// Get the number of tests failed
pub fn failed_count() -> usize {
    TESTS_FAILED.load(Ordering::SeqCst)
}

/// Reset test counters (for test isolation)
pub fn reset() {
    TESTS_PASSED.store(0, Ordering::SeqCst);
    TESTS_FAILED.store(0, Ordering::SeqCst);
}

/// Set the current test name
pub fn set_current_test(name: String) {
    *CURRENT_TEST.lock().unwrap() = name;
}

/// Get the current test name
pub fn current_test() -> String {
    CURRENT_TEST.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert() {
        assert(true, "should pass");
    }

    #[test]
    #[should_panic]
    fn test_assert_fail() {
        assert(false, "should fail");
    }

    #[test]
    fn test_assert_eq() {
        assert_eq(1, 1);
        assert_eq("hello", "hello");
    }

    #[test]
    #[should_panic]
    fn test_assert_eq_fail() {
        assert_eq(1, 2);
    }

    #[test]
    fn test_assert_ne() {
        assert_ne(1, 2);
        assert_ne("hello", "world");
    }

    #[test]
    #[should_panic]
    fn test_assert_ne_fail() {
        assert_ne(1, 1);
    }

    // Enhanced assertions tests
    #[test]
    fn test_assert_gt_passes() {
        assert_gt(5, 3);
        assert_gt(10.5, 10.4);
    }

    #[test]
    #[should_panic(expected = "assertion failed: left > right")]
    fn test_assert_gt_fails() {
        assert_gt(3, 5);
    }

    #[test]
    fn test_assert_lt_passes() {
        assert_lt(3, 5);
        assert_lt(10.4, 10.5);
    }

    #[test]
    #[should_panic(expected = "assertion failed: left < right")]
    fn test_assert_lt_fails() {
        assert_lt(5, 3);
    }

    #[test]
    fn test_assert_approx_passes() {
        assert_approx(3.1, 3.0, 0.2);
        assert_approx(1.0, 1.0001, 0.001);
    }

    #[test]
    #[should_panic(expected = "assertion failed: values not approximately equal")]
    fn test_assert_approx_fails() {
        assert_approx(3.1, 2.0, 0.01);
    }

    #[test]
    fn test_assert_contains_passes() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_contains(&vec, &3);
    }

    #[test]
    #[should_panic(expected = "assertion failed: collection doesn't contain item")]
    fn test_assert_contains_fails() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_contains(&vec, &10);
    }

    #[test]
    fn test_assert_length_passes() {
        let vec = vec![1, 2, 3];
        assert_length(&vec, 3);
    }

    #[test]
    #[should_panic(expected = "assertion failed: collection length mismatch")]
    fn test_assert_length_fails() {
        let vec = vec![1, 2, 3];
        assert_length(&vec, 5);
    }

    #[test]
    fn test_assert_str_contains_passes() {
        assert_str_contains("hello world", "world");
    }

    #[test]
    #[should_panic(expected = "assertion failed: string doesn't contain substring")]
    fn test_assert_str_contains_fails() {
        assert_str_contains("hello world", "foo");
    }

    #[test]
    fn test_assert_is_some_passes() {
        let opt = Some(42);
        assert_is_some(&opt);
    }

    #[test]
    #[should_panic(expected = "assertion failed: Option is None")]
    fn test_assert_is_some_fails() {
        let opt: Option<i32> = None;
        assert_is_some(&opt);
    }

    #[test]
    fn test_assert_in_range_passes() {
        assert_in_range(5, 0, 10);
        assert_in_range(3.1, 3.0, 4.0);
    }

    #[test]
    #[should_panic(expected = "assertion failed: value not in range")]
    fn test_assert_in_range_fails() {
        assert_in_range(15, 0, 10);
    }

    // Advanced assertions tests
    #[test]
    fn test_assert_panics_passes() {
        assert_panics(|| {
            panic!("This should panic");
        });
    }

    #[test]
    #[should_panic(expected = "expected panic, but function completed successfully")]
    fn test_assert_panics_fails() {
        assert_panics(|| {
            // This doesn't panic
        });
    }

    #[test]
    fn test_assert_panics_with_passes() {
        assert_panics_with("division by zero", || {
            panic!("Error: division by zero occurred");
        });
    }

    #[test]
    #[should_panic(expected = "panic message mismatch")]
    fn test_assert_panics_with_fails() {
        assert_panics_with("expected message", || {
            panic!("different message");
        });
    }

    #[test]
    fn test_assert_result_ok_passes() {
        let result: Result<i32, &str> = Ok(42);
        assert_result_ok(&result);
    }

    #[test]
    #[should_panic(expected = "expected Ok(_), got Err")]
    fn test_assert_result_ok_fails() {
        let result: Result<i32, &str> = Err("error");
        assert_result_ok(&result);
    }

    #[test]
    fn test_assert_result_err_passes() {
        let result: Result<i32, &str> = Err("error");
        assert_result_err(&result);
    }

    #[test]
    #[should_panic(expected = "expected Err(_), got Ok")]
    fn test_assert_result_err_fails() {
        let result: Result<i32, &str> = Ok(42);
        assert_result_err(&result);
    }

    #[test]
    fn test_assert_deep_eq_passes() {
        let vec1 = vec![1, 2, 3];
        let vec2 = vec![1, 2, 3];
        assert_deep_eq(&vec1, &vec2);
    }

    #[test]
    #[should_panic(expected = "deep equality check failed")]
    fn test_assert_deep_eq_fails() {
        let vec1 = vec![1, 2, 3];
        let vec2 = vec![1, 2, 4];
        assert_deep_eq(&vec1, &vec2);
    }
}
