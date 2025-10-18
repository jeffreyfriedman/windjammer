//! Component runtime - manages lifecycle, state, and re-rendering
//!
//! Design Philosophy:
//! - Components are just data + render function
//! - State changes trigger automatic re-renders
//! - Runtime handles all the complexity
//!
//! Architecture:
//! ```
//! Component State → Signal<T> → Update → Notify Runtime → Re-render → Patch DOM
//! ```

use crate::component::Component;
use crate::events::ComponentEventDispatcher;
use crate::vdom::VNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Component runtime manages the lifecycle of a mounted component
pub struct ComponentRuntime<C: Component> {
    component: Rc<RefCell<C>>,
    current_vnode: Rc<RefCell<Option<VNode>>>,
    event_dispatcher: Rc<RefCell<ComponentEventDispatcher>>,
    #[cfg(target_arch = "wasm32")]
    root_element: Option<web_sys::Element>,
}

impl<C: Component + 'static> ComponentRuntime<C> {
    /// Create a new component runtime
    pub fn new(component: C) -> Self {
        Self {
            component: Rc::new(RefCell::new(component)),
            current_vnode: Rc::new(RefCell::new(None)),
            event_dispatcher: Rc::new(RefCell::new(ComponentEventDispatcher::new())),
            #[cfg(target_arch = "wasm32")]
            root_element: None,
        }
    }

    /// Mount the component to a DOM element
    #[cfg(target_arch = "wasm32")]
    pub fn mount(&mut self, target: web_sys::Element) -> Result<(), String> {
        use crate::renderer::WebRenderer;

        // Initial render
        let vnode = self.component.borrow().render();

        // Create DOM from VNode
        let renderer = WebRenderer::new();
        let dom_node = renderer.create_element(&vnode)?;

        // Clear target and append
        while let Some(child) = target.first_child() {
            target
                .remove_child(&child)
                .map_err(|_| "Failed to clear target")?;
        }

        target
            .append_child(&dom_node)
            .map_err(|_| "Failed to mount component")?;

        // Store current state
        *self.current_vnode.borrow_mut() = Some(vnode);
        self.root_element = Some(target);

        Ok(())
    }

    /// Re-render the component (called when state changes)
    #[cfg(target_arch = "wasm32")]
    pub fn re_render(&self) -> Result<(), String> {
        use crate::renderer::WebRenderer;

        let new_vnode = self.component.borrow().render();

        // For now, do a full re-render (optimized diffing/patching coming later)
        if let Some(root) = &self.root_element {
            // Clear existing content
            while let Some(child) = root.first_child() {
                root.remove_child(&child)
                    .map_err(|_| "Failed to remove child")?;
            }

            // Render new content
            let renderer = WebRenderer::new();
            let dom_node = renderer.create_element(&new_vnode)?;
            root.append_child(&dom_node)
                .map_err(|_| "Failed to append new content")?;
        }

        // Update stored VNode
        *self.current_vnode.borrow_mut() = Some(new_vnode);

        Ok(())
    }

    /// Register an event handler
    pub fn register_event<F>(&mut self, event_name: String, handler: F)
    where
        F: Fn() + 'static,
    {
        self.event_dispatcher
            .borrow_mut()
            .register(event_name, handler);
    }

    /// Get the event dispatcher (for wiring up DOM events)
    pub fn dispatcher(&self) -> Rc<RefCell<ComponentEventDispatcher>> {
        self.event_dispatcher.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdom::{VElement, VNode, VText};

    struct TestComponent {
        count: i32,
    }

    impl Component for TestComponent {
        fn render(&self) -> VNode {
            VElement::new("div")
                .child(VNode::Text(VText::new(format!("Count: {}", self.count))))
                .into()
        }
    }

    #[test]
    fn test_runtime_creation() {
        let component = TestComponent { count: 0 };
        let runtime = ComponentRuntime::new(component);
        assert!(runtime.current_vnode.borrow().is_none());
    }

    #[test]
    fn test_event_registration() {
        let component = TestComponent { count: 0 };
        let mut runtime = ComponentRuntime::new(component);

        let called = Rc::new(RefCell::new(false));
        let called_clone = called.clone();

        runtime.register_event("click".to_string(), move || {
            *called_clone.borrow_mut() = true;
        });

        assert!(!*called.borrow());
        runtime.dispatcher().borrow().dispatch("click").unwrap();
        assert!(*called.borrow());
    }
}
