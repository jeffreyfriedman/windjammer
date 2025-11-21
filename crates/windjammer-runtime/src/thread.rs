//! Threading utilities
//!
//! Windjammer's thread module provides ergonomic thread spawning and management
//! with simplified error handling.

pub use std::thread::{
    current, panicking, park, park_timeout, sleep, spawn, yield_now, Builder, JoinHandle, Thread,
    ThreadId,
};
pub use std::time::Duration;

/// Spawn a new thread with a closure
pub fn spawn_thread<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    spawn(f)
}

/// Sleep for a number of seconds
pub fn sleep_seconds(secs: u64) {
    sleep(Duration::from_secs(secs))
}

/// Sleep for a number of milliseconds
pub fn sleep_millis(millis: u64) {
    sleep(Duration::from_millis(millis))
}

/// Sleep for a number of microseconds
pub fn sleep_micros(micros: u64) {
    sleep(Duration::from_micros(micros))
}

/// Get the current thread
pub fn current_thread() -> Thread {
    current()
}

/// Get the current thread ID
pub fn current_id() -> ThreadId {
    current().id()
}

/// Get the current thread name
pub fn current_name() -> Option<String> {
    current().name().map(String::from)
}

/// Yield the current thread
pub fn yield_thread() {
    yield_now()
}

/// Check if the current thread is panicking
pub fn is_panicking() -> bool {
    panicking()
}

/// Park the current thread
pub fn park_thread() {
    park()
}

/// Park the current thread with timeout
pub fn park_thread_timeout(duration: Duration) {
    park_timeout(duration)
}

/// Join a thread and unwrap the result
pub fn join<T>(handle: JoinHandle<T>) -> T {
    handle.join().expect("Thread panicked")
}

/// Try to join a thread
pub fn try_join<T>(handle: JoinHandle<T>) -> Result<T, String> {
    handle.join().map_err(|_| "Thread panicked".to_string())
}

/// Get available parallelism (number of logical cores)
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Scope-based thread spawning (safe scoped threads)
pub fn scope<'env, F, R>(f: F) -> R
where
    F: for<'scope> FnOnce(&'scope std::thread::Scope<'scope, 'env>) -> R,
{
    std::thread::scope(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_and_join() {
        let handle = spawn_thread(|| 42);

        assert_eq!(join(handle), 42);
    }

    #[test]
    fn test_sleep() {
        let start = std::time::Instant::now();
        sleep_millis(100);
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 100);
    }

    #[test]
    fn test_current_thread() {
        let thread = current_thread();
        let id = current_id();

        assert_eq!(thread.id(), id);
    }

    #[test]
    fn test_available_parallelism() {
        let n = available_parallelism();
        assert!(n >= 1);
    }

    #[test]
    fn test_scoped_threads() {
        let mut data = [1, 2, 3, 4];

        scope(|s| {
            s.spawn(|| {
                data[0] = 10;
            });
        });

        assert_eq!(data[0], 10);
    }
}
