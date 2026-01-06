//! Function mocking utilities
//!
//! Provides runtime support for mocking global functions.
//!
//! Note: Full function mocking requires unsafe code for runtime function
//! replacement. This module provides a safe framework using function pointers.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

thread_local! {
    static FUNCTION_MOCKS: RefCell<HashMap<String, Arc<Mutex<Box<dyn std::any::Any + Send>>>>> = RefCell::new(HashMap::new());
}

/// Mock a function with a closure
///
/// # Safety
/// This is a safe version that requires explicit checking in the function being mocked.
/// For true runtime function replacement, unsafe code would be needed.
///
/// # Example
/// ```
/// use windjammer_runtime::mock_function::{mock_function, get_mock, clear_mock};
///
/// // Original function
/// fn get_time() -> i64 {
///     // Check for mock
///     if let Some(mock) = get_mock::<fn() -> i64>("get_time") {
///         return mock();
///     }
///     // Real implementation
///     std::time::SystemTime::now()
///         .duration_since(std::time::UNIX_EPOCH)
///         .unwrap()
///         .as_secs() as i64
/// }
///
/// // In test:
/// mock_function("get_time", || 12345i64);
/// assert_eq!(get_time(), 12345);
/// clear_mock("get_time");
/// ```
pub fn mock_function<F: 'static + Send>(name: &str, mock_fn: F) {
    FUNCTION_MOCKS.with(|mocks| {
        mocks
            .borrow_mut()
            .insert(name.to_string(), Arc::new(Mutex::new(Box::new(mock_fn))));
    });
}

/// Get a mocked function
pub fn get_mock<F: 'static>(name: &str) -> Option<F>
where
    F: Clone,
{
    FUNCTION_MOCKS.with(|mocks| {
        mocks
            .borrow()
            .get(name)
            .and_then(|mock| {
                let guard = mock.lock().unwrap();
                let any_ref = &**guard;
                // Try to downcast to the function type
                (any_ref as &dyn std::any::Any).downcast_ref::<F>().cloned()
            })
    })
}

/// Check if a function is mocked
pub fn is_mocked(name: &str) -> bool {
    FUNCTION_MOCKS.with(|mocks| mocks.borrow().contains_key(name))
}

/// Clear a specific mock
pub fn clear_mock(name: &str) {
    FUNCTION_MOCKS.with(|mocks| {
        mocks.borrow_mut().remove(name);
    });
}

/// Clear all mocks
pub fn clear_all_mocks() {
    FUNCTION_MOCKS.with(|mocks| {
        mocks.borrow_mut().clear();
    });
}

/// Run a function with a temporary mock
///
/// Automatically restores the original function after execution.
///
/// # Example
/// ```
/// use windjammer_runtime::mock_function::with_mock;
///
/// fn get_time() -> i64 { 0 } // Simplified
///
/// with_mock("get_time", || 12345i64, || {
///     // Mock is active here
///     // let time = get_time();
///     // assert_eq!(time, 12345);
/// });
/// // Mock is automatically cleared
/// ```
pub fn with_mock<F, R, T>(function_name: &str, mock_fn: F, test_fn: T) -> R
where
    F: 'static + Send,
    T: FnOnce() -> R,
{
    mock_function(function_name, mock_fn);
    let result = test_fn();
    clear_mock(function_name);
    result
}

/// Mock registry for tracking mocked functions
#[derive(Debug, Default)]
pub struct MockRegistry {
    mocks: HashMap<String, usize>, // Function name -> call count
}

impl MockRegistry {
    pub fn new() -> Self {
        Self {
            mocks: HashMap::new(),
        }
    }

    /// Record that a function was called
    pub fn record_call(&mut self, function_name: &str) {
        *self.mocks.entry(function_name.to_string()).or_insert(0) += 1;
    }

    /// Get call count for a function
    pub fn call_count(&self, function_name: &str) -> usize {
        self.mocks.get(function_name).copied().unwrap_or(0)
    }

    /// Was function called?
    pub fn was_called(&self, function_name: &str) -> bool {
        self.call_count(function_name) > 0
    }

    /// Verify function was called exactly N times
    pub fn verify_called_times(&self, function_name: &str, expected_count: usize) {
        let actual = self.call_count(function_name);
        if actual != expected_count {
            panic!(
                "Expected {} to be called {} times, but it was called {} times",
                function_name, expected_count, actual
            );
        }
    }

    /// Reset all counts
    pub fn reset(&mut self) {
        self.mocks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_function() {
        mock_function("test_func", || 100);
        assert!(is_mocked("test_func"));
        clear_mock("test_func");
        assert!(!is_mocked("test_func"));
    }

    #[test]
    fn test_is_mocked() {
        assert!(!is_mocked("nonexistent"));
        mock_function("exists", || 1);
        assert!(is_mocked("exists"));
        clear_mock("exists");
        assert!(!is_mocked("exists"));
    }

    #[test]
    fn test_clear_all_mocks() {
        mock_function("func1", || 1);
        mock_function("func2", || 2);
        assert!(is_mocked("func1"));
        assert!(is_mocked("func2"));
        
        clear_all_mocks();
        
        assert!(!is_mocked("func1"));
        assert!(!is_mocked("func2"));
    }

    #[test]
    fn test_with_mock_cleanup() {
        with_mock("test_function", || 999, || {
            // Test body
        });
        
        assert!(!is_mocked("test_function")); // Auto-cleared
    }

    #[test]
    fn test_mock_registry() {
        let mut registry = MockRegistry::new();
        
        registry.record_call("func1");
        registry.record_call("func1");
        registry.record_call("func2");
        
        assert_eq!(registry.call_count("func1"), 2);
        assert_eq!(registry.call_count("func2"), 1);
        assert!(registry.was_called("func1"));
        assert!(!registry.was_called("func3"));
    }

    #[test]
    fn test_verify_called_times_success() {
        let mut registry = MockRegistry::new();
        registry.record_call("func");
        registry.record_call("func");
        
        registry.verify_called_times("func", 2); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Expected func to be called 3 times")]
    fn test_verify_called_times_failure() {
        let mut registry = MockRegistry::new();
        registry.record_call("func");
        registry.record_call("func");
        
        registry.verify_called_times("func", 3); // Should panic
    }

    #[test]
    fn test_registry_reset() {
        let mut registry = MockRegistry::new();
        registry.record_call("func");
        assert_eq!(registry.call_count("func"), 1);
        
        registry.reset();
        assert_eq!(registry.call_count("func"), 0);
    }
}

