//! Advanced interface (trait) mocking utilities
//!
//! Provides runtime support for mocking traits/interfaces.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Method call record
#[derive(Debug, Clone)]
pub struct MethodCall {
    pub method_name: String,
    pub args: Vec<String>,
}

impl MethodCall {
    pub fn new(method_name: String, args: Vec<String>) -> Self {
        Self { method_name, args }
    }
}

/// Expectation for a method call
#[derive(Debug, Clone)]
pub struct Expectation {
    pub method_name: String,
    pub expected_args: Option<Vec<String>>,
    pub return_value: Option<String>,
    pub call_count: Option<usize>,
}

impl Expectation {
    pub fn new(method_name: String) -> Self {
        Self {
            method_name,
            expected_args: None,
            return_value: None,
            call_count: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.expected_args = Some(args);
        self
    }

    pub fn returns(mut self, value: String) -> Self {
        self.return_value = Some(value);
        self
    }

    pub fn times(mut self, count: usize) -> Self {
        self.call_count = Some(count);
        self
    }
}

/// Mock object for traits
pub struct MockObject {
    calls: Arc<Mutex<Vec<MethodCall>>>,
    expectations: Arc<Mutex<Vec<Expectation>>>,
    return_values: Arc<Mutex<HashMap<String, Vec<Box<dyn Any + Send>>>>>,
}

impl MockObject {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            expectations: Arc::new(Mutex::new(Vec::new())),
            return_values: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record a method call
    pub fn record_call(&self, method_name: &str, args: Vec<String>) {
        self.calls
            .lock()
            .unwrap()
            .push(MethodCall::new(method_name.to_string(), args));
    }

    /// Add an expectation
    pub fn expect(&self, expectation: Expectation) {
        self.expectations.lock().unwrap().push(expectation);
    }

    /// Set return value for a method
    pub fn set_return<T: 'static + Send>(&self, method_name: &str, value: T) {
        self.return_values
            .lock()
            .unwrap()
            .entry(method_name.to_string())
            .or_default()
            .push(Box::new(value));
    }

    /// Get return value for a method
    pub fn get_return<T: 'static>(&self, method_name: &str) -> Option<T> {
        let mut values = self.return_values.lock().unwrap();
        let method_values = values.get_mut(method_name)?;
        if method_values.is_empty() {
            return None;
        }
        let boxed = method_values.remove(0);
        boxed.downcast::<T>().ok().map(|b| *b)
    }

    /// Verify all expectations
    pub fn verify(&self) {
        let calls = self.calls.lock().unwrap();
        let expectations = self.expectations.lock().unwrap();

        for expectation in expectations.iter() {
            let matching_calls: Vec<_> = calls
                .iter()
                .filter(|call| call.method_name == expectation.method_name)
                .collect();

            if let Some(expected_count) = expectation.call_count {
                if matching_calls.len() != expected_count {
                    panic!(
                        "Expected {} calls to '{}', but got {}",
                        expected_count,
                        expectation.method_name,
                        matching_calls.len()
                    );
                }
            }

            if let Some(expected_args) = &expectation.expected_args {
                let found = matching_calls.iter().any(|call| &call.args == expected_args);
                if !found {
                    panic!(
                        "Expected call to '{}' with args {:?}, but not found",
                        expectation.method_name, expected_args
                    );
                }
            }
        }
    }

    /// Get call count for a method
    pub fn call_count(&self, method_name: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| call.method_name == method_name)
            .count()
    }

    /// Was method called?
    pub fn was_called(&self, method_name: &str) -> bool {
        self.call_count(method_name) > 0
    }

    /// Get all calls for a method
    pub fn get_calls(&self, method_name: &str) -> Vec<MethodCall> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| call.method_name == method_name)
            .cloned()
            .collect()
    }

    /// Reset mock state
    pub fn reset(&self) {
        self.calls.lock().unwrap().clear();
        self.expectations.lock().unwrap().clear();
        self.return_values.lock().unwrap().clear();
    }
}

impl Default for MockObject {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockObject {
    fn clone(&self) -> Self {
        Self {
            calls: Arc::clone(&self.calls),
            expectations: Arc::clone(&self.expectations),
            return_values: Arc::clone(&self.return_values),
        }
    }
}

/// Trait mock helper macro (would be generated by compiler)
///
/// Example usage in generated code:
/// ```
/// // For trait Database { fn query(&self, sql: &str) -> Vec<Row>; }
/// 
/// struct MockDatabase {
///     mock: MockObject,
/// }
///
/// impl MockDatabase {
///     fn new() -> Self {
///         Self {
///             mock: MockObject::new(),
///         }
///     }
/// 
///     fn expect_query(&self) -> Expectation {
///         Expectation::new("query".to_string())
///     }
/// }
///
/// impl Database for MockDatabase {
///     fn query(&self, sql: &str) -> Vec<Row> {
///         self.mock.record_call("query", vec![sql.to_string()]);
///         self.mock.get_return("query").unwrap_or_default()
///     }
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_object_creation() {
        let mock = MockObject::new();
        assert_eq!(mock.call_count("test_method"), 0);
    }

    #[test]
    fn test_record_call() {
        let mock = MockObject::new();
        mock.record_call("query", vec!["SELECT *".to_string()]);
        assert_eq!(mock.call_count("query"), 1);
        assert!(mock.was_called("query"));
    }

    #[test]
    fn test_multiple_calls() {
        let mock = MockObject::new();
        mock.record_call("query", vec!["SELECT *".to_string()]);
        mock.record_call("query", vec!["INSERT".to_string()]);
        mock.record_call("insert", vec!["data".to_string()]);
        
        assert_eq!(mock.call_count("query"), 2);
        assert_eq!(mock.call_count("insert"), 1);
    }

    #[test]
    fn test_get_calls() {
        let mock = MockObject::new();
        mock.record_call("query", vec!["SELECT *".to_string()]);
        mock.record_call("query", vec!["INSERT".to_string()]);
        
        let calls = mock.get_calls("query");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].args[0], "SELECT *");
        assert_eq!(calls[1].args[0], "INSERT");
    }

    #[test]
    fn test_set_and_get_return() {
        let mock = MockObject::new();
        mock.set_return("query", 42);
        
        let result: Option<i32> = mock.get_return("query");
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_return_values_fifo() {
        let mock = MockObject::new();
        mock.set_return("query", 1);
        mock.set_return("query", 2);
        mock.set_return("query", 3);
        
        assert_eq!(mock.get_return::<i32>("query"), Some(1));
        assert_eq!(mock.get_return::<i32>("query"), Some(2));
        assert_eq!(mock.get_return::<i32>("query"), Some(3));
        assert_eq!(mock.get_return::<i32>("query"), None);
    }

    #[test]
    fn test_expectation_builder() {
        let expectation = Expectation::new("query".to_string())
            .with_args(vec!["SELECT *".to_string()])
            .returns("result".to_string())
            .times(2);
        
        assert_eq!(expectation.method_name, "query");
        assert_eq!(expectation.expected_args, Some(vec!["SELECT *".to_string()]));
        assert_eq!(expectation.return_value, Some("result".to_string()));
        assert_eq!(expectation.call_count, Some(2));
    }

    #[test]
    fn test_verify_success() {
        let mock = MockObject::new();
        mock.expect(Expectation::new("query".to_string()).times(2));
        
        mock.record_call("query", vec!["SELECT *".to_string()]);
        mock.record_call("query", vec!["INSERT".to_string()]);
        
        mock.verify(); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Expected 2 calls")]
    fn test_verify_call_count_failure() {
        let mock = MockObject::new();
        mock.expect(Expectation::new("query".to_string()).times(2));
        
        mock.record_call("query", vec!["SELECT *".to_string()]);
        
        mock.verify(); // Should panic
    }

    #[test]
    #[should_panic(expected = "Expected call to 'query' with args")]
    fn test_verify_args_failure() {
        let mock = MockObject::new();
        mock.expect(
            Expectation::new("query".to_string())
                .with_args(vec!["SELECT *".to_string()])
        );
        
        mock.record_call("query", vec!["INSERT".to_string()]);
        
        mock.verify(); // Should panic
    }

    #[test]
    fn test_reset() {
        let mock = MockObject::new();
        mock.record_call("query", vec!["SELECT *".to_string()]);
        mock.set_return("query", 42);
        
        assert_eq!(mock.call_count("query"), 1);
        
        mock.reset();
        
        assert_eq!(mock.call_count("query"), 0);
        assert_eq!(mock.get_return::<i32>("query"), None);
    }

    #[test]
    fn test_mock_clone() {
        let mock1 = MockObject::new();
        mock1.record_call("query", vec!["SELECT *".to_string()]);
        
        let mock2 = mock1.clone();
        
        // Both share the same state
        assert_eq!(mock2.call_count("query"), 1);
        
        mock2.record_call("query", vec!["INSERT".to_string()]);
        assert_eq!(mock1.call_count("query"), 2); // Changes visible in mock1
    }
}

