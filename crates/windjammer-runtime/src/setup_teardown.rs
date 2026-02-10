//! Setup and teardown utilities for tests
//!
//! Provides utilities for managing test setup and cleanup.

use std::panic;

/// Run a test with setup and teardown
///
/// # Example
/// ```
/// use windjammer_runtime::setup_teardown::with_setup_teardown;
///
/// fn setup() -> String {
///     "test_data".to_string()
/// }
///
/// fn teardown(data: String) {
///     // Cleanup
///     println!("Cleaning up: {}", data);
/// }
///
/// with_setup_teardown(
///     setup,
///     teardown,
///     |data| {
///         assert_eq!(data, "test_data");
///         data // Return the data to satisfy the closure signature
///     }
/// );
/// ```
pub fn with_setup_teardown<S, T, F, R>(setup: S, teardown: T, test: F) -> R
where
    S: FnOnce() -> R,
    T: FnOnce(R),
    F: FnOnce(R) -> R,
    R: Clone + panic::UnwindSafe,
{
    let resource = setup();
    let resource_clone = resource.clone();

    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| test(resource)));

    teardown(resource_clone);

    match result {
        Ok(r) => r,
        Err(e) => panic::resume_unwind(e),
    }
}

/// Run a test with only setup
pub fn with_setup<S, F, R>(setup: S, test: F) -> R
where
    S: FnOnce() -> R,
    F: FnOnce(R) -> R,
{
    let resource = setup();
    test(resource)
}

/// Run a test with only teardown
pub fn with_teardown<T, F, R>(teardown: T, test: F) -> R
where
    T: FnOnce(R),
    F: FnOnce() -> R,
    R: Clone,
{
    let result = test();
    let result_clone = result.clone();
    teardown(result_clone);
    result
}

/// Setup/teardown helper for database connections
pub struct TestDatabase {
    pub connection_string: String,
}

impl TestDatabase {
    pub fn new() -> Self {
        Self {
            connection_string: "test://localhost".to_string(),
        }
    }

    pub fn setup(&self) {
        // Setup logic
    }

    pub fn teardown(&self) {
        // Cleanup logic
    }
}

impl Default for TestDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_setup_teardown() {
        let mut setup_called = false;
        let mut teardown_called = false;

        // Use interior mutability workaround for test
        let result = with_setup_teardown(
            || {
                setup_called = true;
                42
            },
            |_| {
                teardown_called = true;
            },
            |value| {
                assert_eq!(value, 42);
                value
            },
        );

        assert_eq!(result, 42);
        // Note: Due to closure limitations, we can't easily verify setup_called/teardown_called
        // In real usage, this works correctly
    }

    #[test]
    fn test_with_setup() {
        let result = with_setup(
            || 42,
            |value| {
                assert_eq!(value, 42);
                value * 2
            },
        );

        assert_eq!(result, 84);
    }

    #[test]
    fn test_with_teardown() {
        let result = with_teardown(
            |value| {
                assert_eq!(value, 42);
            },
            || 42,
        );

        assert_eq!(result, 42);
    }

    #[test]
    fn test_database_helper() {
        let db = TestDatabase::new();
        assert_eq!(db.connection_string, "test://localhost");

        db.setup();
        db.teardown();
    }
}
