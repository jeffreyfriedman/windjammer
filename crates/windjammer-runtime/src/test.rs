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
// COMPARISONS
// ============================================================================

/// Assert that a value is greater than another
pub fn assert_gt<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if !(left > right) {
        panic!("assertion failed: left > right\n  left: {:?}\n right: {:?}", left, right);
    }
}

/// Assert that a value is less than another
pub fn assert_lt<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if !(left < right) {
        panic!("assertion failed: left < right\n  left: {:?}\n right: {:?}", left, right);
    }
}

/// Assert that a value is greater than or equal to another
pub fn assert_ge<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if !(left >= right) {
        panic!("assertion failed: left >= right\n  left: {:?}\n right: {:?}", left, right);
    }
}

/// Assert that a value is less than or equal to another
pub fn assert_le<T: PartialOrd + std::fmt::Debug>(left: T, right: T) {
    if !(left <= right) {
        panic!("assertion failed: left <= right\n  left: {:?}\n right: {:?}", left, right);
    }
}

/// A float width `assert_approx_eq` accepts: `f32` and `f64`.
pub trait ApproxFloat: Copy + PartialOrd + std::fmt::Display + std::ops::Sub<Output = Self> {
    fn abs_value(self) -> Self;
}

impl ApproxFloat for f32 {
    fn abs_value(self) -> Self {
        self.abs()
    }
}

impl ApproxFloat for f64 {
    fn abs_value(self) -> Self {
        self.abs()
    }
}

/// An operand brought to the width of the left operand: the same width is the identity, the other width converts.
/// (A float literal the compiler could not tie to the other operand's width still compares.)
pub trait ApproxFrom<S> {
    fn approx_from(value: S) -> Self;
}

impl ApproxFrom<f32> for f32 {
    fn approx_from(value: f32) -> f32 {
        value
    }
}
impl ApproxFrom<f64> for f64 {
    fn approx_from(value: f64) -> f64 {
        value
    }
}
impl ApproxFrom<f64> for f32 {
    fn approx_from(value: f64) -> f32 {
        value as f32
    }
}
impl ApproxFrom<f32> for f64 {
    fn approx_from(value: f32) -> f64 {
        value as f64
    }
}

/// Assert that two floats differ by at most `epsilon`, compared at the width of `left` (`f32` or `f64`; the
/// operands normally share it). A NaN never passes.
pub fn assert_approx_eq<T, R, E>(left: T, right: R, epsilon: E)
where
    T: ApproxFloat + ApproxFrom<R> + ApproxFrom<E>,
{
    let right = T::approx_from(right);
    let epsilon = T::approx_from(epsilon);
    let diff = (left - right).abs_value();
    if !(diff <= epsilon) {
        panic!(
            "assertion failed: values not approximately equal\n  left: {}\n  right: {}\n  epsilon: {}\n  diff: {}",
            left, right, epsilon, diff
        );
    }
}

/// Assert that a value is in an inclusive range
pub fn assert_in_range<T: PartialOrd + std::fmt::Debug>(value: T, min: T, max: T) {
    if value < min || value > max {
        panic!(
            "assertion failed: value not in range\n  value: {:?}\n  min: {:?}\n  max: {:?}",
            value, min, max
        );
    }
}

// ============================================================================
// OPTION / RESULT: the payload is returned
// ============================================================================

/// Assert that an Option is Some and return the value
pub fn assert_some<T>(value: Option<T>) -> T {
    match value {
        Some(v) => v,
        None => panic!("assertion failed: expected Some, got None"),
    }
}

/// Assert that an Option is None
pub fn assert_none<T: std::fmt::Debug>(value: Option<T>) {
    if let Some(v) = value {
        panic!("assertion failed: expected None, got Some({:?})", v);
    }
}

/// Assert that a Result is Ok and return the value
pub fn assert_ok<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(v) => v,
        Err(e) => panic!("assertion failed: expected Ok, got Err({:?})", e),
    }
}

/// Assert that a Result is Err and return the error
pub fn assert_err<T: std::fmt::Debug, E>(value: Result<T, E>) -> E {
    match value {
        Ok(v) => panic!("assertion failed: expected Err, got Ok({:?})", v),
        Err(e) => e,
    }
}

// ============================================================================
// STRINGS AND COLLECTIONS
// ============================================================================

/// What `assert_contains` can search: a string for a substring, a list for an element.
pub trait ContainsCheck<N> {
    /// `Err(panic message)` when the needle is absent.
    fn check_contains(&self, needle: &N) -> Result<(), String>;
}

fn string_contains(hay: &str, needle: &str) -> Result<(), String> {
    if hay.contains(needle) {
        Ok(())
    } else {
        Err(format!(
            "assertion failed: string does not contain substring\n  string: {:?}\n  substring: {:?}",
            hay, needle
        ))
    }
}

impl<N: AsRef<str>> ContainsCheck<N> for str {
    fn check_contains(&self, needle: &N) -> Result<(), String> {
        string_contains(self, needle.as_ref())
    }
}

impl<N: AsRef<str>> ContainsCheck<N> for String {
    fn check_contains(&self, needle: &N) -> Result<(), String> {
        string_contains(self, needle.as_ref())
    }
}

fn list_contains<T: PartialEq + std::fmt::Debug>(list: &[T], item: &T) -> Result<(), String> {
    if list.contains(item) {
        Ok(())
    } else {
        Err(format!(
            "assertion failed: collection does not contain item\n  collection: {:?}\n  item: {:?}",
            list, item
        ))
    }
}

macro_rules! list_haystack {
    ($($ty:ty),*) => {$(
        impl<T: PartialEq + std::fmt::Debug> ContainsCheck<T> for $ty {
            fn check_contains(&self, needle: &T) -> Result<(), String> {
                list_contains(self, needle)
            }
        }
        impl<T: PartialEq + std::fmt::Debug> ContainsCheck<&T> for $ty {
            fn check_contains(&self, needle: &&T) -> Result<(), String> {
                list_contains(self, needle)
            }
        }
    )*};
}

list_haystack!(Vec<T>, [T]);

impl<T: PartialEq + std::fmt::Debug, const L: usize> ContainsCheck<T> for [T; L] {
    fn check_contains(&self, needle: &T) -> Result<(), String> {
        list_contains(self, needle)
    }
}

impl<T: PartialEq + std::fmt::Debug, const L: usize> ContainsCheck<&T> for [T; L] {
    fn check_contains(&self, needle: &&T) -> Result<(), String> {
        list_contains(self, needle)
    }
}

impl<N, H: ContainsCheck<N> + ?Sized> ContainsCheck<N> for &H {
    fn check_contains(&self, needle: &N) -> Result<(), String> {
        (**self).check_contains(needle)
    }
}

/// Assert that a string contains a substring, or a list contains an element
pub fn assert_contains<H: ContainsCheck<N>, N>(haystack: H, needle: N) {
    if let Err(message) = haystack.check_contains(&needle) {
        panic!("{}", message);
    }
}

/// Assert that a string starts with a prefix
pub fn assert_starts_with(string: impl AsRef<str>, prefix: impl AsRef<str>) {
    let (string, prefix) = (string.as_ref(), prefix.as_ref());
    if !string.starts_with(prefix) {
        panic!(
            "assertion failed: string does not start with prefix\n  string: {:?}\n  prefix: {:?}",
            string, prefix
        );
    }
}

/// Assert that a string ends with a suffix
pub fn assert_ends_with(string: impl AsRef<str>, suffix: impl AsRef<str>) {
    let (string, suffix) = (string.as_ref(), suffix.as_ref());
    if !string.ends_with(suffix) {
        panic!(
            "assertion failed: string does not end with suffix\n  string: {:?}\n  suffix: {:?}",
            string, suffix
        );
    }
}

/// Anything with a length: lists, strings and maps (owned or borrowed).
pub trait HasLen {
    fn item_len(&self) -> usize;
}

impl<T> HasLen for Vec<T> {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<T> HasLen for [T] {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<T, const N: usize> HasLen for [T; N] {
    fn item_len(&self) -> usize {
        N
    }
}
impl HasLen for str {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl HasLen for String {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<K, V> HasLen for std::collections::HashMap<K, V> {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<K, V> HasLen for std::collections::BTreeMap<K, V> {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<T> HasLen for std::collections::HashSet<T> {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<T> HasLen for std::collections::BTreeSet<T> {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<T> HasLen for std::collections::VecDeque<T> {
    fn item_len(&self) -> usize {
        self.len()
    }
}
impl<T: HasLen + ?Sized> HasLen for &T {
    fn item_len(&self) -> usize {
        (**self).item_len()
    }
}

/// Assert that a collection (or string) is empty
pub fn assert_empty<C: HasLen>(collection: C) {
    let len = collection.item_len();
    if len != 0 {
        panic!("assertion failed: expected empty, got length {}", len);
    }
}

/// Assert that a collection (or string) is not empty
pub fn assert_not_empty<C: HasLen>(collection: C) {
    if collection.item_len() == 0 {
        panic!("assertion failed: expected non-empty, got length 0");
    }
}

/// Assert that a collection (or string) has a specific length
pub fn assert_length<C: HasLen>(collection: C, expected: usize) {
    let actual = collection.item_len();
    if actual != expected {
        panic!(
            "assertion failed: length mismatch\n  expected: {}\n  actual: {}",
            expected, actual
        );
    }
}

// ============================================================================
// CLOSURES THAT PANIC
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
/// assert_panics_with("error", || {
///     panic!("error");
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

// ============================================================================
// DESIGN BY CONTRACT - @requires, @ensures, @invariant
// ============================================================================

/// Check a precondition (used by @requires decorator)
/// Asserts that a condition is true before a function executes
pub fn requires(condition: bool, description: &str) {
    if !condition {
        panic!("Precondition violation (@requires): {}", description);
    }
}

/// Check a postcondition (used by @ensures decorator)
/// Asserts that a condition is true after a function executes
pub fn ensures(condition: bool, description: &str) {
    if !condition {
        panic!("Postcondition violation (@ensures): {}", description);
    }
}

/// Check an invariant (used by @invariant decorator)
/// Asserts that a condition remains true throughout execution
pub fn invariant(condition: bool, description: &str) {
    if !condition {
        panic!("Invariant violation (@invariant): {}", description);
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

    fn panic_message<F: FnOnce() + std::panic::UnwindSafe>(f: F) -> String {
        let err = std::panic::catch_unwind(f).expect_err("expected a panic");
        err.downcast_ref::<String>().cloned().or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
    }

    #[test]
    fn test_assert_eq_ne() {
        assert(true, "should pass");
        assert_eq(1, 1);
        assert_ne(1, 2);
    }

    #[test]
    #[should_panic]
    fn test_assert_fail() {
        assert(false, "should fail");
    }

    #[test]
    fn comparisons_pass_and_fail() {
        assert_gt(5, 3);
        assert_lt(3, 5);
        assert_ge(5, 5);
        assert_le(5, 5);
        assert_gt(10.5, 10.4);
        assert_eq!(panic_message(|| assert_gt(1, 2)), "assertion failed: left > right\n  left: 1\n right: 2");
        assert_eq!(panic_message(|| assert_lt(2, 1)), "assertion failed: left < right\n  left: 2\n right: 1");
        assert_eq!(panic_message(|| assert_ge(1, 2)), "assertion failed: left >= right\n  left: 1\n right: 2");
        assert_eq!(panic_message(|| assert_le(2, 1)), "assertion failed: left <= right\n  left: 2\n right: 1");
        // equal operands fail the strict forms
        assert!(std::panic::catch_unwind(|| assert_gt(2, 2)).is_err());
        assert!(std::panic::catch_unwind(|| assert_lt(2, 2)).is_err());
    }

    #[test]
    fn approx_eq_accepts_both_widths() {
        assert_approx_eq(3.1_f64, 3.0, 0.2);
        assert_approx_eq(1.0_f32, 1.0001, 0.001);
        assert_eq!(
            panic_message(|| assert_approx_eq(3.5_f64, 2.0, 0.5)),
            "assertion failed: values not approximately equal\n  left: 3.5\n  right: 2\n  epsilon: 0.5\n  diff: 1.5"
        );
        assert_eq!(
            panic_message(|| assert_approx_eq(3.5_f32, 2.0, 0.5)),
            "assertion failed: values not approximately equal\n  left: 3.5\n  right: 2\n  epsilon: 0.5\n  diff: 1.5"
        );
        // a NaN never passes
        assert!(std::panic::catch_unwind(|| assert_approx_eq(f64::NAN, 1.0, 1.0)).is_err());
        // the width of the left operand decides when a literal arrives as `f64`
        let x: f32 = 3.14;
        assert_approx_eq(x, 3.14_f64, 0.001_f64);
        assert_approx_eq(2.5_f64, 2.5_f32, 0.001_f32);
    }

    #[test]
    fn in_range() {
        assert_in_range(5, 0, 10);
        assert_eq!(
            panic_message(|| assert_in_range(15, 0, 10)),
            "assertion failed: value not in range\n  value: 15\n  min: 0\n  max: 10"
        );
    }

    #[test]
    fn option_and_result_return_the_payload() {
        assert_eq!(assert_some(Some(7)), 7);
        assert_none::<i32>(None);
        assert_eq!(assert_ok::<i32, String>(Ok(3)), 3);
        assert_eq!(assert_err::<i32, &str>(Err("bad")), "bad");
        assert_eq!(panic_message(|| { assert_some::<i32>(None); }), "assertion failed: expected Some, got None");
        assert_eq!(panic_message(|| assert_none(Some(1))), "assertion failed: expected None, got Some(1)");
        assert_eq!(panic_message(|| { assert_ok::<i32, &str>(Err("bad")); }), "assertion failed: expected Ok, got Err(\"bad\")");
        assert_eq!(panic_message(|| { assert_err::<i32, &str>(Ok(1)); }), "assertion failed: expected Err, got Ok(1)");
    }

    #[test]
    fn contains_strings_and_lists() {
        assert_contains("hello world", "world");
        assert_contains(String::from("hello world"), String::from("lo w"));
        let nums = vec![1, 2, 3];
        assert_contains(&nums, &3);
        assert_contains(&nums, 3);
        assert_contains(nums.clone(), 2);
        assert_eq!(
            panic_message(|| assert_contains("hello world", "foo")),
            "assertion failed: string does not contain substring\n  string: \"hello world\"\n  substring: \"foo\""
        );
        assert_eq!(
            panic_message(move || assert_contains(&nums, &9)),
            "assertion failed: collection does not contain item\n  collection: [1, 2, 3]\n  item: 9"
        );
    }

    #[test]
    fn starts_and_ends_with() {
        assert_starts_with("hello", "he");
        assert_ends_with(String::from("hello"), "llo");
        assert_eq!(
            panic_message(|| assert_starts_with("hello", "lo")),
            "assertion failed: string does not start with prefix\n  string: \"hello\"\n  prefix: \"lo\""
        );
        assert_eq!(
            panic_message(|| assert_ends_with("hello", "he")),
            "assertion failed: string does not end with suffix\n  string: \"hello\"\n  suffix: \"he\""
        );
    }

    #[test]
    fn empty_not_empty_length() {
        let empty: Vec<i32> = vec![];
        let full = vec![1, 2, 3];
        assert_empty(&empty);
        assert_empty("");
        assert_not_empty(&full);
        assert_not_empty(String::from("x"));
        assert_length(&full, 3);
        assert_length("abc", 3);
        assert_eq!(panic_message(|| assert_empty(vec![1])), "assertion failed: expected empty, got length 1");
        assert_eq!(panic_message(|| assert_not_empty(Vec::<i32>::new())), "assertion failed: expected non-empty, got length 0");
        assert_eq!(panic_message(move || assert_length(&full, 5)), "assertion failed: length mismatch\n  expected: 5\n  actual: 3");
    }

    #[test]
    fn panics_helpers() {
        assert_panics(|| {
            panic!("This should panic");
        });
        assert_panics_with("division by zero", || {
            panic!("Error: division by zero occurred");
        });
        assert!(panic_message(|| assert_panics(|| {})).contains("expected panic, but function completed successfully"));
        assert!(panic_message(|| assert_panics_with("expected message", || panic!("different message"))).contains("panic message mismatch"));
    }
}
