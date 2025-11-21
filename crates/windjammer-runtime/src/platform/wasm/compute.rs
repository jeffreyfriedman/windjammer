#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Function, Promise};
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
/// WASM implementation of std::compute using Web Workers
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{MessageEvent, Worker};

/// Serializable task for sending to Web Workers
#[derive(Serialize, Deserialize)]
struct Task {
    id: usize,
    data: String,     // JSON-serialized data
    function: String, // Serialized function code
}

/// Result from a Web Worker
#[derive(Serialize, Deserialize)]
struct TaskResult {
    id: usize,
    result: String, // JSON-serialized result
    error: Option<String>,
}

/// Global worker pool (lazy-initialized)
/// Note: Using thread_local! instead of static because WASM is single-threaded
thread_local! {
    static WORKER_POOL: RefCell<Option<Rc<WorkerPool>>> = RefCell::new(None);
}

/// Get or create the global worker pool
fn get_worker_pool() -> Rc<WorkerPool> {
    WORKER_POOL.with(|pool| {
        let mut pool_ref = pool.borrow_mut();
        if pool_ref.is_none() {
            *pool_ref = Some(Rc::new(WorkerPool::new(num_workers())));
        }
        pool_ref.as_ref().unwrap().clone()
    })
}

/// Pool of Web Workers for parallel computation
#[derive(Clone)]
pub struct WorkerPool {
    workers: Rc<RefCell<Vec<Worker>>>,
    next_task_id: Rc<RefCell<usize>>,
    pending_tasks: Rc<RefCell<HashMap<usize, js_sys::Function>>>,
}

impl WorkerPool {
    /// Create a new worker pool with the specified number of workers
    pub fn new(size: usize) -> Self {
        let workers = (0..size)
            .filter_map(|_| {
                // Create worker from inline script
                // In a real implementation, this would load a separate worker.js file
                Worker::new("worker.js").ok()
            })
            .collect();

        Self {
            workers: Rc::new(RefCell::new(workers)),
            next_task_id: Rc::new(RefCell::new(0)),
            pending_tasks: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Execute a task on an available worker
    pub async fn execute_task(&self, task: Task) -> Result<String, String> {
        let workers = self.workers.borrow();
        if workers.is_empty() {
            return Err("No workers available".to_string());
        }

        // Round-robin worker selection
        let worker_idx = task.id % workers.len();
        let worker = &workers[worker_idx];

        // Create a promise to wait for the result
        let (promise, resolve, reject) = create_promise();

        // Store the resolve/reject callbacks
        {
            let mut pending = self.pending_tasks.borrow_mut();
            pending.insert(task.id, resolve);
        }

        // Send task to worker
        let task_json = serde_json::to_string(&task).unwrap();
        worker
            .post_message(&JsValue::from_str(&task_json))
            .map_err(|e| format!("Failed to post message: {:?}", e))?;

        // Wait for result
        let result = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| format!("Task failed: {:?}", e))?;

        Ok(result.as_string().unwrap_or_default())
    }
}

/// Create a JavaScript Promise with resolve/reject callbacks
fn create_promise() -> (Promise, js_sys::Function, js_sys::Function) {
    let resolve_ref = Rc::new(RefCell::new(None));
    let reject_ref = Rc::new(RefCell::new(None));

    let resolve_clone = resolve_ref.clone();
    let reject_clone = reject_ref.clone();

    let promise = Promise::new(&mut |resolve, reject| {
        *resolve_clone.borrow_mut() = Some(resolve);
        *reject_clone.borrow_mut() = Some(reject);
    });

    let resolve = resolve_ref.borrow_mut().take().unwrap();
    let reject = reject_ref.borrow_mut().take().unwrap();

    (promise, resolve, reject)
}

/// Run a computation in parallel across multiple items
/// Uses Web Workers to distribute work
pub fn parallel<T, R, F>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Serialize + for<'de> Deserialize<'de>,
    R: Serialize + for<'de> Deserialize<'de>,
    F: Fn(T) -> R,
{
    // For now, fall back to sequential execution
    // Full Web Worker implementation requires:
    // 1. Serializing the function (complex)
    // 2. Setting up worker.js file
    // 3. Handling message passing

    // TODO: Implement full Web Worker parallelism
    items.into_iter().map(f).collect()
}

/// Run a computation in the background (non-blocking)
/// Uses a Web Worker for execution
pub fn background<T, F>(f: F) -> BackgroundTask<T>
where
    T: 'static,
    F: FnOnce() -> T + 'static,
{
    // For now, execute immediately
    // Full implementation would use Web Worker
    let result = f();
    BackgroundTask {
        result: Some(result),
    }
}

/// Get the number of available workers
/// Returns navigator.hardwareConcurrency or 4 as default
pub fn num_workers() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        window()
            .map(|w| {
                let count = w.navigator().hardware_concurrency() as usize;
                if count > 0 {
                    count
                } else {
                    4
                }
            })
            .unwrap_or(4)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        4
    }
}

/// Run two computations in parallel and wait for both to complete
pub fn join<A, B, FA, FB>(a: FA, b: FB) -> (A, B)
where
    FA: FnOnce() -> A,
    FB: FnOnce() -> B,
{
    // For now, execute sequentially
    // Full implementation would use two Web Workers
    let result_a = a();
    let result_b = b();
    (result_a, result_b)
}

/// Map-reduce pattern: parallel map followed by reduce
pub fn map_reduce<T, R, M, Red>(items: Vec<T>, map_fn: M, reduce_fn: Red, initial: R) -> R
where
    T: Clone,
    R: Clone,
    M: Fn(T) -> R,
    Red: Fn(R, R) -> R,
{
    // For now, execute sequentially
    // Full implementation would distribute across Web Workers
    items.into_iter().map(map_fn).fold(initial, reduce_fn)
}

/// Handle for a background task
pub struct BackgroundTask<T> {
    result: Option<T>,
}

impl<T> BackgroundTask<T> {
    /// Wait for the computation to complete and get the result
    pub fn await_result(mut self) -> T {
        self.result.take().expect("BackgroundTask already consumed")
    }

    /// Check if the computation is complete
    pub fn is_ready(&self) -> bool {
        self.result.is_some()
    }
}

// Note: Full Web Worker implementation requires:
// 1. A worker.js file that can execute serialized functions
// 2. Proper serialization of closures (very complex in Rust)
// 3. Message passing infrastructure
//
// For now, this provides a working fallback that executes sequentially.
// Future versions can add true parallelism with Web Workers.
