//! Test timeout utilities
//!
//! Provides timeout functionality for tests.

use std::panic;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Run a test with a timeout
///
/// # Example
/// ```
/// use windjammer_runtime::timeout::with_timeout;
/// use std::time::Duration;
///
/// let result = with_timeout(Duration::from_millis(100), || {
///     // Test code that should complete quickly
///     assert_eq!(1 + 1, 2);
/// });
///
/// assert!(result.is_ok());
/// ```
pub fn with_timeout<F, R>(timeout: Duration, test_fn: F) -> Result<R, TimeoutError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    let _handle = thread::spawn(move || {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(test_fn));
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(panic_err)) => {
            // Test panicked (assertion failure)
            panic::resume_unwind(panic_err);
        }
        Err(_) => {
            // Timeout occurred
            Err(TimeoutError { duration: timeout })
        }
    }
}

/// Timeout error
#[derive(Debug, Clone)]
pub struct TimeoutError {
    pub duration: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test timed out after {:?}", self.duration)
    }
}

impl std::error::Error for TimeoutError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_with_timeout_passes() {
        let result = with_timeout(Duration::from_millis(100), || {
            // Fast test
            42
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_timeout_fails() {
        let result = with_timeout(Duration::from_millis(50), || {
            // Slow test
            sleep(Duration::from_millis(200));
            42
        });

        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_with_timeout_panics() {
        let _result = with_timeout(Duration::from_millis(100), || {
            panic!("assertion failed");
        });
    }
}
