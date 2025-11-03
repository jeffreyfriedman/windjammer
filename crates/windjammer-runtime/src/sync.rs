//! Synchronization primitives
//!
//! Windjammer's sync module provides thread-safe synchronization primitives
//! with simplified error handling and ergonomic APIs.

pub use std::sync::{
    mpsc, Arc, Condvar, Mutex, MutexGuard, Once, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

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
