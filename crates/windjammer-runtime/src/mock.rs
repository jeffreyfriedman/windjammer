//! Simple mocking utilities for Windjammer tests
//!
//! Provides basic mocking without requiring complex external dependencies.
//! For advanced mocking, consider using mockall or similar crates.

use std::sync::{Arc, Mutex};

/// A simple mock call tracker
///
/// # Example
/// ```
/// use windjammer_runtime::mock::MockTracker;
///
/// let tracker = MockTracker::new();
/// tracker.record_call("my_function", vec!["arg1".to_string()]);
/// assert_eq!(tracker.call_count("my_function"), 1);
/// assert!(tracker.was_called("my_function"));
/// ```
pub struct MockTracker {
    calls: Arc<Mutex<Vec<(String, Vec<String>)>>>,
}

impl MockTracker {
    /// Create a new mock tracker
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record a function call with arguments
    pub fn record_call(&self, function: &str, args: Vec<String>) {
        let mut calls = self.calls.lock().unwrap();
        calls.push((function.to_string(), args));
    }

    /// Check if a function was called
    pub fn was_called(&self, function: &str) -> bool {
        let calls = self.calls.lock().unwrap();
        calls.iter().any(|(name, _)| name == function)
    }

    /// Get the number of times a function was called
    pub fn call_count(&self, function: &str) -> usize {
        let calls = self.calls.lock().unwrap();
        calls.iter().filter(|(name, _)| name == function).count()
    }

    /// Get all calls to a specific function
    pub fn get_calls(&self, function: &str) -> Vec<Vec<String>> {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter(|(name, _)| name == function)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// Clear all recorded calls
    pub fn reset(&self) {
        let mut calls = self.calls.lock().unwrap();
        calls.clear();
    }

    /// Verify that a function was called a specific number of times
    pub fn verify_call_count(&self, function: &str, expected: usize) {
        let actual = self.call_count(function);
        if actual != expected {
            panic!(
                "Mock verification failed: expected {} to be called {} time(s), but was called {} time(s)",
                function, expected, actual
            );
        }
    }

    /// Verify that a function was called at least once
    pub fn verify_called(&self, function: &str) {
        if !self.was_called(function) {
            panic!(
                "Mock verification failed: expected {} to be called at least once",
                function
            );
        }
    }

    /// Verify that a function was never called
    pub fn verify_not_called(&self, function: &str) {
        if self.was_called(function) {
            panic!(
                "Mock verification failed: expected {} to never be called, but was called {} time(s)",
                function,
                self.call_count(function)
            );
        }
    }
}

impl Default for MockTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple mock return value
pub struct MockReturn<T> {
    values: Arc<Mutex<Vec<T>>>,
    index: Arc<Mutex<usize>>,
}

impl<T: Clone> MockReturn<T> {
    /// Create a new mock return value
    pub fn new(values: Vec<T>) -> Self {
        Self {
            values: Arc::new(Mutex::new(values)),
            index: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the next return value
    pub fn next(&self) -> Option<T> {
        let mut index = self.index.lock().unwrap();
        let values = self.values.lock().unwrap();

        if *index < values.len() {
            let value = values[*index].clone();
            *index += 1;
            Some(value)
        } else {
            None
        }
    }

    /// Reset to the first return value
    pub fn reset(&self) {
        let mut index = self.index.lock().unwrap();
        *index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_tracker_basic() {
        let tracker = MockTracker::new();

        tracker.record_call("foo", vec!["arg1".to_string()]);

        assert!(tracker.was_called("foo"));
        assert!(!tracker.was_called("bar"));
        assert_eq!(tracker.call_count("foo"), 1);
        assert_eq!(tracker.call_count("bar"), 0);
    }

    #[test]
    fn test_mock_tracker_multiple_calls() {
        let tracker = MockTracker::new();

        tracker.record_call("foo", vec!["1".to_string()]);
        tracker.record_call("foo", vec!["2".to_string()]);
        tracker.record_call("bar", vec!["3".to_string()]);

        assert_eq!(tracker.call_count("foo"), 2);
        assert_eq!(tracker.call_count("bar"), 1);

        let foo_calls = tracker.get_calls("foo");
        assert_eq!(foo_calls.len(), 2);
        assert_eq!(foo_calls[0], vec!["1".to_string()]);
        assert_eq!(foo_calls[1], vec!["2".to_string()]);
    }

    #[test]
    fn test_mock_tracker_reset() {
        let tracker = MockTracker::new();

        tracker.record_call("foo", vec![]);
        assert_eq!(tracker.call_count("foo"), 1);

        tracker.reset();
        assert_eq!(tracker.call_count("foo"), 0);
    }

    #[test]
    fn test_mock_tracker_verify_called() {
        let tracker = MockTracker::new();
        tracker.record_call("foo", vec![]);
        tracker.verify_called("foo");
    }

    #[test]
    #[should_panic(expected = "Mock verification failed")]
    fn test_mock_tracker_verify_called_fails() {
        let tracker = MockTracker::new();
        tracker.verify_called("foo");
    }

    #[test]
    fn test_mock_tracker_verify_not_called() {
        let tracker = MockTracker::new();
        tracker.verify_not_called("foo");
    }

    #[test]
    #[should_panic(expected = "Mock verification failed")]
    fn test_mock_tracker_verify_not_called_fails() {
        let tracker = MockTracker::new();
        tracker.record_call("foo", vec![]);
        tracker.verify_not_called("foo");
    }

    #[test]
    fn test_mock_return() {
        let mock = MockReturn::new(vec![1, 2, 3]);

        assert_eq!(mock.next(), Some(1));
        assert_eq!(mock.next(), Some(2));
        assert_eq!(mock.next(), Some(3));
        assert_eq!(mock.next(), None);

        mock.reset();
        assert_eq!(mock.next(), Some(1));
    }
}
