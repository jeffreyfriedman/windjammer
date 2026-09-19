//! Synchronization primitives
//!
//! Windjammer's sync module provides thread-safe synchronization primitives
//! with simplified error handling and ergonomic APIs.

pub use std::sync::{
    atomic, mpsc, Arc, Condvar, Mutex, MutexGuard, Once, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

pub use atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

/// Create a new Mutex
pub fn mutex<T>(value: T) -> Mutex<T> {
    Mutex::new(value)
}

/// Create a new RwLock
pub fn rwlock<T>(value: T) -> RwLock<T> {
    RwLock::new(value)
}

/// Create a new Arc
pub fn arc<T>(value: T) -> Arc<T> {
    Arc::new(value)
}

/// Create a new Arc<Mutex<T>>
pub fn arc_mutex<T>(value: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(value))
}

/// Create a new Arc<RwLock<T>>
pub fn arc_rwlock<T>(value: T) -> Arc<RwLock<T>> {
    Arc::new(RwLock::new(value))
}

/// Lock a mutex, panicking on poison
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().expect("Mutex poisoned")
}

/// Try to lock a mutex
pub fn try_lock<T>(mutex: &Mutex<T>) -> Option<MutexGuard<'_, T>> {
    mutex.try_lock().ok()
}

/// Read lock an RwLock
pub fn read<T>(rwlock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    rwlock.read().expect("RwLock poisoned")
}

/// Write lock an RwLock
pub fn write<T>(rwlock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    rwlock.write().expect("RwLock poisoned")
}

/// Try to read lock an RwLock
pub fn try_read<T>(rwlock: &RwLock<T>) -> Option<RwLockReadGuard<'_, T>> {
    rwlock.try_read().ok()
}

/// Try to write lock an RwLock
pub fn try_write<T>(rwlock: &RwLock<T>) -> Option<RwLockWriteGuard<'_, T>> {
    rwlock.try_write().ok()
}

/// Create a channel
pub fn channel<T>() -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    mpsc::channel()
}

/// Idiomatic alias for [`channel`] — `std::sync::unbounded` graduation vocabulary.
pub fn unbounded<T>() -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    mpsc::channel()
}

/// Send on an owned sender; returns the sender for reuse (wj-sync / std::sync shape).
pub fn send<T>(tx: mpsc::Sender<T>, value: T) -> mpsc::Sender<T> {
    let _ = tx.send(value);
    tx
}

/// Blocking recv; returns `(receiver, Some(value))` or `(receiver, None)` on disconnect.
pub fn recv<T>(rx: mpsc::Receiver<T>) -> (mpsc::Receiver<T>, Option<T>) {
    match rx.recv() {
        Ok(v) => (rx, Some(v)),
        Err(_) => (rx, None),
    }
}

/// Thread-safe shared cell — `std::sync::shared` (no Arc/Mutex in WJ source).
pub struct SharedInt {
    inner: Arc<Mutex<i64>>,
}

/// Create a shared int cell.
pub fn shared(value: i64) -> SharedInt {
    SharedInt {
        inner: Arc::new(Mutex::new(value)),
    }
}

/// Add `delta` to a shared int; returns the same handle.
pub fn shared_add(s: SharedInt, delta: i64) -> SharedInt {
    if let Ok(mut g) = s.inner.lock() {
        *g += delta;
    }
    s
}

/// Read the current shared int value.
pub fn shared_get(s: &SharedInt) -> i64 {
    s.inner.lock().map(|g| *g).unwrap_or(0)
}

/// Create a bounded channel
pub fn sync_channel<T>(bound: usize) -> (mpsc::SyncSender<T>, mpsc::Receiver<T>) {
    mpsc::sync_channel(bound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutex() {
        let m = mutex(5);
        {
            let mut guard = lock(&m);
            *guard = 10;
        }
        assert_eq!(*lock(&m), 10);
    }

    #[test]
    fn test_rwlock() {
        let rw = rwlock(String::from("hello"));

        // Multiple readers
        {
            let r1 = read(&rw);
            let r2 = read(&rw);
            assert_eq!(&*r1, "hello");
            assert_eq!(&*r2, "hello");
        }

        // Single writer
        {
            let mut w = write(&rw);
            *w = String::from("world");
        }

        assert_eq!(&*read(&rw), "world");
    }

    #[test]
    fn test_channel() {
        let (tx, rx) = channel();

        tx.send(42).unwrap();
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn test_arc_mutex() {
        let data = arc_mutex(0);
        let data2 = Arc::clone(&data);

        *lock(&data) = 5;
        assert_eq!(*lock(&data2), 5);
    }
}
