//! Opaque crate values (ADR-0031): a `regex::Regex` lives in the crate host; the program holds a handle.
//!
//! A handle is a `Value::Native` holding the host's id and the qualified type (`regex::Regex`). Copies of the
//! value share it (an `Rc`), so the object dies when the last copy does: the handle's `Drop` queues its id, and
//! the next call to the host first sends the queued ids in one release frame. A handle read back from an effect
//! log (`--replay`) has no host behind it and releases nothing; calls on it are answered from the log too.

use super::super::native::Native;
use super::super::value::Value;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// Ids whose last handle is gone, waiting for the next request to the host.
pub type ReleaseQueue = Arc<Mutex<Vec<u64>>>;

pub struct HandleObj {
    pub id: u64,
    /// `crate::Type`.
    pub qual: String,
    type_name: &'static str,
    release: Option<ReleaseQueue>,
}

impl Drop for HandleObj {
    fn drop(&mut self) {
        if let Some(q) = &self.release {
            if let Ok(mut q) = q.lock() {
                q.push(self.id);
            }
        }
    }
}

impl Native for HandleObj {
    fn type_name(&self) -> &'static str {
        self.type_name
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn debug(&self) -> String {
        format!("{} {{ .. }}", self.type_name)
    }
}

/// A live handle (released when its last copy is dropped) or, with `release: None`, a replayed one.
/// `type_name` is the type's last segment (`Regex`), interned by the host (`CrateHost::type_name`).
pub fn value(qual: &str, type_name: &'static str, id: u64, release: Option<ReleaseQueue>) -> Value {
    Value::Native(std::rc::Rc::new(HandleObj { id, qual: qual.to_string(), type_name, release }))
}

/// `(crate::Type, id)` of a handle value.
pub fn parts(v: &Value) -> Option<(&str, u64)> {
    super::super::native::downcast::<HandleObj>(v).map(|h| (h.qual.as_str(), h.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_copy_releases_once() {
        let q: ReleaseQueue = Arc::new(Mutex::new(Vec::new()));
        let a = value("ctr::Counter", "Counter", 7, Some(q.clone()));
        let b = a.clone();
        assert_eq!(parts(&b), Some(("ctr::Counter", 7)));
        drop(a);
        assert!(q.lock().unwrap().is_empty());
        drop(b);
        assert_eq!(*q.lock().unwrap(), vec![7]);
        drop(value("ctr::Counter", "Counter", 8, None)); // replayed: no host to tell
        assert_eq!(*q.lock().unwrap(), vec![7]);
    }
}
