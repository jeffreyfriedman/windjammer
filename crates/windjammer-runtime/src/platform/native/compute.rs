/// Native implementation of std::compute using rayon for parallelism
use rayon::prelude::*;
use std::thread;

/// Run a computation in parallel across multiple items
pub fn parallel<T, R, F>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send + Sync,
    R: Send,
    F: Fn(T) -> R + Send + Sync,
{
    items.into_par_iter().map(f).collect()
}

/// Run a computation in the background (non-blocking)
/// Returns a handle that can be joined
pub fn background<T, F>(f: F) -> BackgroundTask<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let handle = thread::spawn(f);
    BackgroundTask {
        handle: Some(handle),
    }
}

/// Get the number of available CPU cores
pub fn num_workers() -> usize {
    rayon::current_num_threads()
}

/// Run two computations in parallel and wait for both to complete
pub fn join<A, B, FA, FB>(a: FA, b: FB) -> (A, B)
where
    A: Send,
    B: Send,
    FA: FnOnce() -> A + Send,
    FB: FnOnce() -> B + Send,
{
    rayon::join(a, b)
}

/// Map-reduce pattern: parallel map followed by reduce
pub fn map_reduce<T, R, M, Red>(items: Vec<T>, map_fn: M, reduce_fn: Red, initial: R) -> R
where
    T: Send + Sync,
    R: Send + Sync + Clone,
    M: Fn(T) -> R + Send + Sync,
    Red: Fn(R, R) -> R + Send + Sync,
{
    items
        .into_par_iter()
        .map(map_fn)
        .reduce(|| initial.clone(), reduce_fn)
}

/// Handle for a background task
pub struct BackgroundTask<T> {
    handle: Option<thread::JoinHandle<T>>,
}

impl<T> BackgroundTask<T> {
    /// Wait for the computation to complete and get the result
    pub fn await_result(mut self) -> T {
        self.handle
            .take()
            .expect("BackgroundTask already consumed")
            .join()
            .expect("Background task panicked")
    }

    /// Check if the computation is complete
    pub fn is_ready(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| h.is_finished())
            .unwrap_or(true)
    }
}
