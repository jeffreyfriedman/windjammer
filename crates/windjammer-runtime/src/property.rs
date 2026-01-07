//! Property-based testing utilities for Windjammer
//!
//! Provides simple property-based testing without external dependencies.
//! For more advanced property testing, integrate proptest or quickcheck.

use std::fmt::Debug;

/// Run a property test with random generated inputs
///
/// # Example
/// ```
/// use windjammer_runtime::property::property_test;
///
/// property_test(100, || {
///     let a = rand::random::<i32>() % 1000;
///     let b = rand::random::<i32>() % 1000;
///     // Test property: addition is commutative
///     assert_eq!(a + b, b + a);
/// });
/// ```
pub fn property_test<F: Fn()>(iterations: usize, f: F) {
    for i in 0..iterations {
        // Run the property test
        // If it panics, the test fails
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(&f)) {
            Ok(_) => {}
            Err(e) => {
                panic!(
                    "Property test failed on iteration {}/{}: {:?}",
                    i + 1,
                    iterations,
                    e
                );
            }
        }
    }
}

/// Test a property with a specific generator function
///
/// # Example
/// ```
/// use windjammer_runtime::property::property_test_with_gen;
///
/// property_test_with_gen(100, || rand::random::<u32>() % 100, |value| {
///     // Property: all generated values should be < 100
///     assert!(value < 100);
/// });
/// ```
pub fn property_test_with_gen<T, G, F>(iterations: usize, gen: G, property: F)
where
    G: Fn() -> T,
    F: Fn(T),
{
    for i in 0..iterations {
        let value = gen();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| property(value))) {
            Ok(_) => {}
            Err(e) => {
                panic!(
                    "Property test failed on iteration {}/{}: {:?}",
                    i + 1,
                    iterations,
                    e
                );
            }
        }
    }
}

/// Test a property with two generated inputs
///
/// # Example
/// ```
/// use windjammer_runtime::property::property_test_with_gen2;
///
/// property_test_with_gen2(
///     100,
///     || rand::random::<i32>() % 1000,
///     || rand::random::<i32>() % 1000,
///     |a, b| {
///         // Property: addition is commutative
///         assert_eq!(a + b, b + a);
///     }
/// );
/// ```
pub fn property_test_with_gen2<T1, T2, G1, G2, F>(
    iterations: usize,
    gen1: G1,
    gen2: G2,
    property: F,
) where
    G1: Fn() -> T1,
    G2: Fn() -> T2,
    F: Fn(T1, T2),
{
    for i in 0..iterations {
        let val1 = gen1();
        let val2 = gen2();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| property(val1, val2))) {
            Ok(_) => {}
            Err(e) => {
                panic!(
                    "Property test failed on iteration {}/{}: {:?}",
                    i + 1,
                    iterations,
                    e
                );
            }
        }
    }
}

/// Test a property with three generated inputs
pub fn property_test_with_gen3<T1, T2, T3, G1, G2, G3, F>(
    iterations: usize,
    gen1: G1,
    gen2: G2,
    gen3: G3,
    property: F,
) where
    G1: Fn() -> T1,
    G2: Fn() -> T2,
    G3: Fn() -> T3,
    F: Fn(T1, T2, T3),
{
    for i in 0..iterations {
        let val1 = gen1();
        let val2 = gen2();
        let val3 = gen3();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| property(val1, val2, val3)))
        {
            Ok(_) => {}
            Err(e) => {
                panic!(
                    "Property test failed on iteration {}/{}: {:?}",
                    i + 1,
                    iterations,
                    e
                );
            }
        }
    }
}

/// Shrink a value to find minimal failing case
/// This is a simplified shrinking strategy
pub fn shrink_int(value: i64) -> Vec<i64> {
    let mut shrinks = vec![0];
    let mut current = value.abs() / 2;
    while current > 0 {
        shrinks.push(current);
        shrinks.push(-current);
        current /= 2;
    }
    shrinks
}

/// Find the minimal failing input using shrinking
///
/// # Example
/// ```
/// use windjammer_runtime::property::{find_minimal_failing, shrink_int};
///
/// let minimal = find_minimal_failing(
///     1000,
///     shrink_int,
///     |x| x < 100  // Property fails for x >= 100
/// );
///
/// if let Some(min) = minimal {
///     println!("Minimal failing value: {}", min);
/// }
/// ```
pub fn find_minimal_failing<T: Copy + Debug, F, S>(initial: T, shrink: S, property: F) -> Option<T>
where
    F: Fn(T) -> bool,
    S: Fn(T) -> Vec<T>,
{
    // Check if initial value fails
    if property(initial) {
        return None;
    }

    let mut current_failure = initial;

    // Try to shrink to a simpler failing case
    for shrunk in shrink(current_failure) {
        if !property(shrunk) {
            current_failure = shrunk;
        }
    }

    Some(current_failure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_test_passes() {
        property_test(10, || {
            // This property should always hold
            let a = 5;
            let b = 3;
            assert_eq!(a + b, b + a);
        });
    }

    #[test]
    #[should_panic(expected = "Property test failed")]
    fn test_property_test_fails() {
        property_test(10, || {
            panic!("Intentional failure");
        });
    }

    #[test]
    fn test_property_test_with_gen() {
        property_test_with_gen(
            10,
            || 42,
            |value| {
                assert_eq!(value, 42);
            },
        );
    }

    #[test]
    fn test_property_test_with_gen2() {
        property_test_with_gen2(
            10,
            || 1,
            || 2,
            |a, b| {
                assert_eq!(a + b, 3);
            },
        );
    }

    #[test]
    fn test_property_test_with_gen3() {
        property_test_with_gen3(
            10,
            || 1,
            || 2,
            || 3,
            |a, b, c| {
                assert_eq!(a + b + c, 6);
            },
        );
    }

    #[test]
    fn test_shrink_int() {
        let shrinks = shrink_int(100);
        assert!(shrinks.contains(&0));
        assert!(shrinks.contains(&50));
        assert!(shrinks.contains(&-50));
        assert!(shrinks.contains(&25));
    }

    #[test]
    fn test_find_minimal_failing() {
        let minimal = find_minimal_failing(
            1000,
            shrink_int,
            |x| x < 100, // Property fails for x >= 100
        );

        assert!(minimal.is_some());
        let min = minimal.unwrap();
        assert!(min >= 100); // Should find a value >= 100
    }
}
