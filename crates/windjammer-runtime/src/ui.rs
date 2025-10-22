// windjammer-runtime::ui - Virtual DOM implementation backing std::ui

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use web_sys::{Document, Element, Node, Window};

/// Virtual Node - represents a UI element
#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    Element(VElement),
    Text(VText),
    Component(VComponent),
}

/// Virtual Element - represents an HTML element
#[derive(Debug, Clone, PartialEq)]
pub struct VElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<VNode>,
}

impl VElement {
    pub fn new(tag: &str) -> Self {
        VElement {
            tag: tag.to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    pub fn child(mut self, node: VNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn into_vnode(self) -> VNode {
        VNode::Element(self)
    }
}

/// Virtual Text - represents a text node
#[derive(Debug, Clone, PartialEq)]
pub struct VText {
    pub content: String,
}

impl VText {
    pub fn new(content: &str) -> Self {
        VText {
            content: content.to_string(),
        }
    }

    pub fn into_vnode(self) -> VNode {
        VNode::Text(self)
    }
}

/// Virtual Component - represents a component instance
#[derive(Debug, Clone, PartialEq)]
pub struct VComponent {
    pub name: String,
    pub props: HashMap<String, String>,
}

impl VComponent {
    pub fn new(name: &str) -> Self {
        VComponent {
            name: name.to_string(),
            props: HashMap::new(),
        }
    }

    pub fn prop(mut self, key: &str, value: &str) -> Self {
        self.props.insert(key.to_string(), value.to_string());
        self
    }

    pub fn into_vnode(self) -> VNode {
        VNode::Component(self)
    }
}

// Helper functions for creating elements (matches std::ui API)

pub fn div() -> VElement {
    VElement::new("div")
}

pub fn h1(text: &str) -> VNode {
    VElement::new("h1")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn h2(text: &str) -> VNode {
    VElement::new("h2")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn p(text: &str) -> VNode {
    VElement::new("p")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn button(text: &str) -> VElement {
    VElement::new("button").child(VNode::Text(VText::new(text)))
}

pub fn input() -> VElement {
    VElement::new("input")
}

pub fn text(content: &str) -> VNode {
    VNode::Text(VText::new(content))
}

pub fn ul() -> VElement {
    VElement::new("ul")
}

pub fn li(text: &str) -> VNode {
    VElement::new("li")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn span(text: &str) -> VNode {
    VElement::new("span")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

/// Render a VNode tree to HTML string
pub fn render_to_string(node: &VNode) -> String {
    match node {
        VNode::Element(el) => {
            let mut html = format!("<{}", el.tag);

            // Add attributes
            for (key, value) in &el.attributes {
                html.push_str(&format!(" {}=\"{}\"", key, value));
            }

            html.push('>');

            // Add children
            for child in &el.children {
                html.push_str(&render_to_string(child));
            }

            html.push_str(&format!("</{}>", el.tag));
            html
        }
        VNode::Text(text) => text.content.clone(),
        VNode::Component(comp) => {
            format!("<component name=\"{}\" />", comp.name)
        }
    }
}

// ============================================================================
// DOM Rendering (WASM only)
// ============================================================================

#[cfg(feature = "wasm")]
pub fn get_window() -> Option<Window> {
    web_sys::window()
}

#[cfg(feature = "wasm")]
pub fn get_document() -> Option<Document> {
    get_window()?.document()
}

#[cfg(feature = "wasm")]
pub fn render_to_dom(node: &VNode, parent: &Element) -> Result<(), JsValue> {
    let document = get_document().ok_or_else(|| JsValue::from_str("No document"))?;
    let dom_node = vnode_to_dom(node, &document)?;
    parent.append_child(&dom_node)?;
    Ok(())
}

#[cfg(feature = "wasm")]
fn vnode_to_dom(node: &VNode, document: &Document) -> Result<Node, JsValue> {
    match node {
        VNode::Element(el) => {
            let element = document.create_element(&el.tag)?;

            // Set attributes
            for (key, value) in &el.attributes {
                element.set_attribute(key, value)?;
            }

            // Add children
            for child in &el.children {
                let child_node = vnode_to_dom(child, document)?;
                element.append_child(&child_node)?;
            }

            Ok(element.into())
        }
        VNode::Text(text) => {
            let text_node = document.create_text_node(&text.content);
            Ok(text_node.into())
        }
        VNode::Component(_comp) => {
            // For now, components render as empty text
            let text_node = document.create_text_node("");
            Ok(text_node.into())
        }
    }
}

#[cfg(feature = "wasm")]
pub fn mount(node: &VNode, selector: &str) -> Result<(), JsValue> {
    let document = get_document().ok_or_else(|| JsValue::from_str("No document"))?;
    let container = document
        .query_selector(selector)?
        .ok_or_else(|| JsValue::from_str("Container not found"))?;

    render_to_dom(node, &container)
}

// ============================================================================
// Reactivity System - Signals
// ============================================================================

/// Signal<T> - Reactive state container
#[derive(Clone)]
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<Vec<Box<dyn Fn(&T)>>>>,
}

impl<T> Signal<T> {
    /// Create a new signal with an initial value
    pub fn new(initial: T) -> Self {
        Signal {
            value: Rc::new(RefCell::new(initial)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Get the current value (clones the value)
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }

    /// Set a new value and notify subscribers
    pub fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value;
        self.notify();
    }

    /// Update the value using a function
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut *self.value.borrow_mut());
        self.notify();
    }

    /// Subscribe to changes
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        self.subscribers.borrow_mut().push(Box::new(callback));
    }

    /// Notify all subscribers
    fn notify(&self) {
        let value = self.value.borrow();
        for callback in self.subscribers.borrow().iter() {
            callback(&*value);
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("value", &self.value.borrow())
            .finish()
    }
}

/// Helper to create a signal
pub fn signal<T>(initial: T) -> Signal<T> {
    Signal::new(initial)
}

/// Computed signal - derived from other signals
pub struct Computed<T> {
    value: Rc<RefCell<T>>,
}

impl<T> Computed<T> {
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn() -> T + 'static,
        T: 'static,
    {
        let value = Rc::new(RefCell::new(compute()));
        Computed { value }
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }
}

/// Effect - side effect that runs when dependencies change
pub fn effect<F>(f: F)
where
    F: Fn() + 'static,
{
    // Run immediately
    f();
    // In a full implementation, would track dependencies and re-run on changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velement_creation() {
        let el = VElement::new("div");
        assert_eq!(el.tag, "div");
        assert!(el.children.is_empty());
        assert!(el.attributes.is_empty());
    }

    #[test]
    fn test_velement_with_attributes() {
        let el = VElement::new("div")
            .attr("class", "container")
            .attr("id", "main");

        assert_eq!(el.attributes.get("class"), Some(&"container".to_string()));
        assert_eq!(el.attributes.get("id"), Some(&"main".to_string()));
    }

    #[test]
    fn test_velement_with_children() {
        let el = VElement::new("div").child(VNode::Text(VText::new("Hello")));

        assert_eq!(el.children.len(), 1);
        match &el.children[0] {
            VNode::Text(text) => assert_eq!(text.content, "Hello"),
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_helper_functions() {
        let heading = h1("Title");
        match heading {
            VNode::Element(el) => {
                assert_eq!(el.tag, "h1");
                assert_eq!(el.children.len(), 1);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_render_to_string() {
        let node = VElement::new("div")
            .attr("class", "container")
            .child(h1("Hello"))
            .into_vnode();

        let html = render_to_string(&node);
        assert!(html.contains("<div"));
        assert!(html.contains("class=\"container\""));
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn test_signal_creation() {
        let sig = signal(42);
        assert_eq!(sig.get(), 42);
    }

    #[test]
    fn test_signal_set() {
        let sig = signal(10);
        sig.set(20);
        assert_eq!(sig.get(), 20);
    }

    #[test]
    fn test_signal_update() {
        let sig = signal(5);
        sig.update(|v| *v += 10);
        assert_eq!(sig.get(), 15);
    }

    #[test]
    fn test_signal_subscribe() {
        let sig = signal(0);
        let called = Rc::new(RefCell::new(false));
        let called_clone = called.clone();

        sig.subscribe(move |_| {
            *called_clone.borrow_mut() = true;
        });

        sig.set(1);
        assert!(*called.borrow());
    }

    #[test]
    fn test_computed() {
        let base = signal(10);
        let doubled = Computed::new({
            let base = base.clone();
            move || base.get() * 2
        });

        assert_eq!(doubled.get(), 20);
    }
}
