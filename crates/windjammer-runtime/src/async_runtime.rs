//! Async runtime utilities
//!
//! Windjammer's `std::async` module maps to these functions.

use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

/// Spawn an async task
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tokio::spawn(future)
}

/// Block on an async function (creates a runtime)
pub fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let rt = Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(future)
}

/// Sleep for a duration (milliseconds)
pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}

/// Sleep for a duration (seconds)
pub async fn sleep(secs: u64) {
    tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sleep() {
        let start = std::time::Instant::now();
        sleep_ms(100).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 100);
    }

    #[test]
    fn test_block_on() {
        let result = block_on(async { 42 });
        assert_eq!(result, 42);
    }
}
