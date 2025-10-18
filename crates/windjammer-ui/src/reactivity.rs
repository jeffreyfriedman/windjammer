//! Fine-grained reactivity system (Svelte-style)

use std::cell::RefCell;
use std::rc::Rc;

/// A reactive signal that can be subscribed to
#[derive(Clone)]
pub struct Signal<T: Clone> {
    value: Rc<RefCell<T>>,
    #[allow(clippy::type_complexity)]
    subscribers: Rc<RefCell<Vec<Box<dyn Fn(&T)>>>>,
}

impl<T: Clone> Signal<T> {
    /// Create a new signal with an initial value
    pub fn new(initial: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(initial)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Get the current value
    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }

    /// Set a new value and notify subscribers
    pub fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value.clone();
        self.notify(&new_value);
    }

    /// Subscribe to changes
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        self.subscribers.borrow_mut().push(Box::new(callback));
    }

    /// Notify all subscribers
    fn notify(&self, value: &T) {
        for subscriber in self.subscribers.borrow().iter() {
            subscriber(value);
        }
    }
}

/// A computed value derived from signals
pub struct Computed<T: Clone> {
    signal: Signal<T>,
}

impl<T: Clone> Computed<T> {
    /// Create a new computed value
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let initial = compute();
        let signal = Signal::new(initial);

        // In a full implementation, we would track dependencies
        // and re-compute when they change

        Self { signal }
    }

    /// Get the current computed value
    pub fn get(&self) -> T {
        self.signal.get()
    }
}

/// An effect that runs when dependencies change
pub struct Effect {
    cleanup: Option<Box<dyn Fn()>>,
}

impl Effect {
    /// Create a new effect
    pub fn new<F>(effect: F) -> Self
    where
        F: Fn() + 'static,
    {
        effect();

        Self { cleanup: None }
    }

    /// Set a cleanup function
    pub fn on_cleanup<F>(&mut self, cleanup: F)
    where
        F: Fn() + 'static,
    {
        self.cleanup = Some(Box::new(cleanup));
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        if let Some(cleanup) = &self.cleanup {
            cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_creation() {
        let signal = Signal::new(42);
        assert_eq!(signal.get(), 42);
    }

    #[test]
    fn test_signal_update() {
        let signal = Signal::new(0);
        signal.set(10);
        assert_eq!(signal.get(), 10);
    }

    #[test]
    fn test_signal_subscription() {
        let signal = Signal::new(0);
        let called = Rc::new(RefCell::new(false));
        let called_clone = called.clone();

        signal.subscribe(move |_| {
            *called_clone.borrow_mut() = true;
        });

        signal.set(42);
        assert!(*called.borrow());
    }

    #[test]
    fn test_computed() {
        let computed = Computed::new(|| 2 + 2);
        assert_eq!(computed.get(), 4);
    }
}
