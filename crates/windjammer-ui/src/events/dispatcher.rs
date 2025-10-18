//! Event dispatcher for connecting DOM events to component methods
//!
//! Design Philosophy:
//! - Events should feel natural, like writing vanilla JS
//! - State updates should automatically trigger re-renders
//! - No boilerplate - Windjammer handles the wiring
//!
//! Example:
//! ```windjammer
//! @component
//! struct Counter {
//!     count: Signal<int>
//! }
//!
//! impl Counter {
//!     fn increment() {
//!         count.update(|c| *c += 1)  // Auto re-renders
//!     }
//! }
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Event handler function type
pub type EventHandler = Box<dyn Fn() + 'static>;

/// Component event dispatcher manages the mapping between event names and handlers
/// This is specifically for component-level event handling (e.g., onclick="increment")
pub struct ComponentEventDispatcher {
    handlers: Rc<RefCell<HashMap<String, EventHandler>>>,
}

impl ComponentEventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Register an event handler
    pub fn register<F>(&mut self, event_name: String, handler: F)
    where
        F: Fn() + 'static,
    {
        self.handlers
            .borrow_mut()
            .insert(event_name, Box::new(handler));
    }

    /// Dispatch an event by name
    pub fn dispatch(&self, event_name: &str) -> Result<(), String> {
        let handlers = self.handlers.borrow();
        if let Some(handler) = handlers.get(event_name) {
            handler();
            Ok(())
        } else {
            Err(format!("No handler registered for event: {}", event_name))
        }
    }

    /// Get a clone of the handlers map (for WASM interop)
    pub fn handlers_clone(&self) -> Rc<RefCell<HashMap<String, EventHandler>>> {
        self.handlers.clone()
    }
}

impl Default for ComponentEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_event_registration() {
        let mut dispatcher = ComponentEventDispatcher::new();
        let called = Rc::new(RefCell::new(false));
        let called_clone = called.clone();

        dispatcher.register("test_event".to_string(), move || {
            *called_clone.borrow_mut() = true;
        });

        assert!(!*called.borrow());
        dispatcher.dispatch("test_event").unwrap();
        assert!(*called.borrow());
    }

    #[test]
    fn test_multiple_handlers() {
        let mut dispatcher = ComponentEventDispatcher::new();
        let count = Rc::new(RefCell::new(0));

        let count1 = count.clone();
        dispatcher.register("increment".to_string(), move || {
            *count1.borrow_mut() += 1;
        });

        let count2 = count.clone();
        dispatcher.register("decrement".to_string(), move || {
            *count2.borrow_mut() -= 1;
        });

        dispatcher.dispatch("increment").unwrap();
        assert_eq!(*count.borrow(), 1);

        dispatcher.dispatch("increment").unwrap();
        assert_eq!(*count.borrow(), 2);

        dispatcher.dispatch("decrement").unwrap();
        assert_eq!(*count.borrow(), 1);
    }

    #[test]
    fn test_unknown_event() {
        let dispatcher = ComponentEventDispatcher::new();
        let result = dispatcher.dispatch("unknown");
        assert!(result.is_err());
    }
}
