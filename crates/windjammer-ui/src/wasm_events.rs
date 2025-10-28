//! WASM event handling system
//!
//! This module provides a way to attach event handlers to DOM elements
//! and manage their lifecycle properly.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Event handler registry for managing closures
pub struct EventRegistry {
    handlers: Rc<RefCell<HashMap<String, Vec<Closure<dyn FnMut(web_sys::Event)>>>>>,
}

impl EventRegistry {
    /// Create a new event registry
    pub fn new() -> Self {
        Self {
            handlers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Register an event handler
    pub fn register<F>(&self, element_id: &str, event_type: &str, handler: F) -> Result<(), JsValue>
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);

        // Get the element
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let element = document
            .get_element_by_id(element_id)
            .ok_or_else(|| JsValue::from_str(&format!("Element not found: {}", element_id)))?;

        // Attach the event listener
        element.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;

        // Store the closure to keep it alive
        let key = format!("{}:{}", element_id, event_type);
        self.handlers
            .borrow_mut()
            .entry(key)
            .or_insert_with(Vec::new)
            .push(closure);

        Ok(())
    }

    /// Clear all handlers for an element
    pub fn clear(&self, element_id: &str) {
        let mut handlers = self.handlers.borrow_mut();
        handlers.retain(|key, _| !key.starts_with(&format!("{}:", element_id)));
    }

    /// Clear all handlers
    pub fn clear_all(&self) {
        self.handlers.borrow_mut().clear();
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use std::cell::OnceCell;

thread_local! {
    /// Global event registry (thread-local for WASM single-threaded environment)
    static GLOBAL_REGISTRY: OnceCell<EventRegistry> = OnceCell::new();
}

/// Get the global event registry
pub fn with_global_registry<F, R>(f: F) -> R
where
    F: FnOnce(&EventRegistry) -> R,
{
    GLOBAL_REGISTRY.with(|registry| f(registry.get_or_init(EventRegistry::new)))
}

/// Attach an event handler to an element by ID
pub fn attach_handler<F>(element_id: &str, event_type: &str, handler: F) -> Result<(), JsValue>
where
    F: FnMut(web_sys::Event) + 'static,
{
    with_global_registry(|registry| registry.register(element_id, event_type, handler))
}

/// Helper to attach a click handler
pub fn on_click<F>(element_id: &str, handler: F) -> Result<(), JsValue>
where
    F: FnMut(web_sys::Event) + 'static,
{
    attach_handler(element_id, "click", handler)
}

/// Helper to attach an input handler
pub fn on_input<F>(element_id: &str, handler: F) -> Result<(), JsValue>
where
    F: FnMut(web_sys::Event) + 'static,
{
    attach_handler(element_id, "input", handler)
}

/// Helper to attach a submit handler
pub fn on_submit<F>(element_id: &str, handler: F) -> Result<(), JsValue>
where
    F: FnMut(web_sys::Event) + 'static,
{
    attach_handler(element_id, "submit", handler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_registry_creation() {
        let registry = EventRegistry::new();
        assert_eq!(registry.handlers.borrow().len(), 0);
    }

    #[test]
    fn test_event_registry_clear() {
        let registry = EventRegistry::new();
        registry.clear_all();
        assert_eq!(registry.handlers.borrow().len(), 0);
    }
}
