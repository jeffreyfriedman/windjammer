//! Testing utilities
//!
//! Windjammer's `std::testing` module maps to these functions.

/// Assert that a condition is true
pub fn assert(condition: bool, message: &str) {
    if !condition {
        panic!("Assertion failed: {}", message);
    }
}

/// Assert that two values are equal
pub fn assert_eq<T: PartialEq + std::fmt::Debug>(left: T, right: T, message: &str) {
    if left != right {
        panic!(
            "Assertion failed: {}\n  left: {:?}\n right: {:?}",
            message, left, right
        );
    }
}

/// Assert that two values are not equal
pub fn assert_ne<T: PartialEq + std::fmt::Debug>(left: T, right: T, message: &str) {
    if left == right {
        panic!(
            "Assertion failed: {}\n  values should not be equal: {:?}",
            message, left
        );
    }
}

/// Fail a test with a message
pub fn fail(message: &str) -> ! {
    panic!("Test failed: {}", message);
}

/// Check if code panics
pub fn should_panic<F: FnOnce() + std::panic::UnwindSafe>(f: F) -> bool {
    std::panic::catch_unwind(f).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_true() {
        assert(true, "should not panic");
    }

    #[test]
    #[should_panic]
    fn test_assert_false() {
        assert(false, "should panic");
    }

    #[test]
    fn test_assert_eq_pass() {
        assert_eq(5, 5, "should be equal");
    }

    #[test]
    #[should_panic]
    fn test_assert_eq_fail() {
        assert_eq(5, 10, "should panic");
    }

    #[test]
    fn test_should_panic() {
        let panics = should_panic(|| {
            panic!("test panic");
        });
        assert!(panics);

        let no_panic = should_panic(|| {
            // do nothing
        });
        assert!(!no_panic);
    }
}
