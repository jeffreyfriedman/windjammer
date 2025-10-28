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
}

