// WASM-based reactive runtime for zero-JavaScript UI
// This module provides the core reactivity system that runs in WebAssembly

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Document, Element, HtmlElement};

/// WASM-based Signal that automatically updates DOM
type Subscribers<T> = Rc<RefCell<Vec<Box<dyn Fn(&T)>>>>;

pub struct WasmSignal<T> {
    value: Rc<RefCell<T>>,
    subscribers: Subscribers<T>,
    element_bindings: Rc<RefCell<Vec<String>>>, // DOM element IDs bound to this signal
}

impl<T: Clone> WasmSignal<T> {
    pub fn new(initial: T) -> Self {
        WasmSignal {
            value: Rc::new(RefCell::new(initial)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
            element_bindings: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value;
        self.notify();
        #[cfg(target_arch = "wasm32")]
        self.update_dom();
    }

    pub fn bind_to_element(&self, element_id: &str) {
        self.element_bindings
            .borrow_mut()
            .push(element_id.to_string());
    }

    fn notify(&self) {
        let value = self.value.borrow();
        for callback in self.subscribers.borrow().iter() {
            callback(&*value);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn update_dom(&self) {
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                for element_id in self.element_bindings.borrow().iter() {
                    if let Some(element) = document.get_element_by_id(element_id) {
                        // Update element content based on signal value
                        // This would be more sophisticated in a full implementation
                        element.set_text_content(Some(&format!("{:?}", self.value.borrow())));
                    }
                }
            }
        }
    }

    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        self.subscribers.borrow_mut().push(Box::new(callback));
    }
}

/// Virtual DOM differ for efficient updates
#[derive(Default)]
pub struct VDomDiffer {
    #[allow(dead_code)]
    current_tree: RefCell<Option<super::VNode>>,
}

impl VDomDiffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Diff two VNode trees and return patches
    pub fn diff(&self, old: &super::VNode, new: &super::VNode) -> Vec<DomPatch> {
        let mut patches = Vec::new();
        Self::diff_recursive(old, new, &[], &mut patches);
        patches
    }

    fn diff_recursive(
        old: &super::VNode,
        new: &super::VNode,
        path: &[usize],
        patches: &mut Vec<DomPatch>,
    ) {
        use super::VNode;

        match (old, new) {
            (VNode::Text(old_text), VNode::Text(new_text)) => {
                if old_text.content != new_text.content {
                    patches.push(DomPatch::UpdateText {
                        path: path.to_vec(),
                        new_text: new_text.content.clone(),
                    });
                }
            }
            (VNode::Element(old_el), VNode::Element(new_el)) => {
                // Check if tags match
                if old_el.tag != new_el.tag {
                    patches.push(DomPatch::ReplaceNode {
                        path: path.to_vec(),
                        new_node: new.clone(),
                    });
                    return;
                }

                // Diff attributes
                for (key, new_value) in &new_el.attributes {
                    if old_el.attributes.get(key) != Some(new_value) {
                        patches.push(DomPatch::SetAttribute {
                            path: path.to_vec(),
                            key: key.clone(),
                            value: new_value.clone(),
                        });
                    }
                }

                // Check for removed attributes
                for key in old_el.attributes.keys() {
                    if !new_el.attributes.contains_key(key) {
                        patches.push(DomPatch::RemoveAttribute {
                            path: path.to_vec(),
                            key: key.clone(),
                        });
                    }
                }

                // Diff children
                let old_children = &old_el.children;
                let new_children = &new_el.children;

                for (i, (old_child, new_child)) in
                    old_children.iter().zip(new_children.iter()).enumerate()
                {
                    let mut child_path = path.to_vec();
                    child_path.push(i);
                    Self::diff_recursive(old_child, new_child, &child_path, patches);
                }

                // Handle different lengths
                if new_children.len() > old_children.len() {
                    for (i, new_child) in new_children.iter().skip(old_children.len()).enumerate() {
                        patches.push(DomPatch::AppendChild {
                            path: path.to_vec(),
                            child: new_child.clone(),
                            index: old_children.len() + i,
                        });
                    }
                } else if old_children.len() > new_children.len() {
                    for i in (new_children.len()..old_children.len()).rev() {
                        patches.push(DomPatch::RemoveChild {
                            path: path.to_vec(),
                            index: i,
                        });
                    }
                }
            }
            _ => {
                // Different node types, replace
                patches.push(DomPatch::ReplaceNode {
                    path: path.to_vec(),
                    new_node: new.clone(),
                });
            }
        }
    }

    /// Apply patches to the real DOM
    #[cfg(target_arch = "wasm32")]
    pub fn apply_patches(&self, patches: Vec<DomPatch>, root_element_id: &str) {
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(root) = document.get_element_by_id(root_element_id) {
                    for patch in patches {
                        self.apply_patch(&patch, &root, &document);
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn apply_patch(&self, patch: &DomPatch, root: &Element, document: &Document) {
        match patch {
            DomPatch::UpdateText { path, new_text } => {
                if let Some(element) = self.get_element_at_path(root, path) {
                    element.set_text_content(Some(new_text));
                }
            }
            DomPatch::SetAttribute { path, key, value } => {
                if let Some(element) = self.get_element_at_path(root, path) {
                    let _ = element.set_attribute(key, value);
                }
            }
            DomPatch::RemoveAttribute { path, key } => {
                if let Some(element) = self.get_element_at_path(root, path) {
                    let _ = element.remove_attribute(key);
                }
            }
            DomPatch::ReplaceNode { path, new_node } => {
                if let Some(element) = self.get_element_at_path(root, path) {
                    if let Some(parent) = element.parent_element() {
                        if let Some(new_el) = self.vnode_to_element(new_node, document) {
                            let _ = parent.replace_child(&new_el, &element);
                        }
                    }
                }
            }
            DomPatch::AppendChild { path, child, .. } => {
                if let Some(element) = self.get_element_at_path(root, path) {
                    if let Some(new_child) = self.vnode_to_element(child, document) {
                        let _ = element.append_child(&new_child);
                    }
                }
            }
            DomPatch::RemoveChild { path, index } => {
                if let Some(element) = self.get_element_at_path(root, path) {
                    if let Some(child) = element.children().item(*index as u32) {
                        let _ = element.remove_child(&child);
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn get_element_at_path(&self, root: &Element, path: &[usize]) -> Option<Element> {
        let mut current = root.clone();
        for &index in path {
            current = current.children().item(index as u32)?;
        }
        Some(current)
    }

    #[cfg(target_arch = "wasm32")]
    fn vnode_to_element(&self, vnode: &super::VNode, document: &Document) -> Option<Element> {
        use super::VNode;

        match vnode {
            VNode::Element(vel) => {
                let element = document.create_element(&vel.tag).ok()?;

                // Set attributes
                for (key, value) in &vel.attributes {
                    let _ = element.set_attribute(key, value);
                }

                // Add children
                for child in &vel.children {
                    if let Some(child_element) = self.vnode_to_element(child, document) {
                        let _ = element.append_child(&child_element);
                    }
                }

                Some(element)
            }
            VNode::Text(vtext) => {
                let text_node = document.create_text_node(&vtext.content);
                Some(text_node.into())
            }
        }
    }
}

/// DOM patches generated by diffing
#[derive(Clone, Debug)]
pub enum DomPatch {
    UpdateText {
        path: Vec<usize>,
        new_text: String,
    },
    SetAttribute {
        path: Vec<usize>,
        key: String,
        value: String,
    },
    RemoveAttribute {
        path: Vec<usize>,
        key: String,
    },
    ReplaceNode {
        path: Vec<usize>,
        new_node: super::VNode,
    },
    AppendChild {
        path: Vec<usize>,
        child: super::VNode,
        index: usize,
    },
    RemoveChild {
        path: Vec<usize>,
        index: usize,
    },
}

/// Event handler registry for WASM
#[derive(Default)]
pub struct WasmEventRegistry {
    handlers: RefCell<HashMap<String, Box<dyn Fn()>>>,
}

impl WasmEventRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(&self, event_id: &str, handler: F)
    where
        F: Fn() + 'static,
    {
        self.handlers
            .borrow_mut()
            .insert(event_id.to_string(), Box::new(handler));
    }

    pub fn trigger(&self, event_id: &str) {
        if let Some(handler) = self.handlers.borrow().get(event_id) {
            handler();
        }
    }

    /// Bind event handlers to DOM elements
    #[cfg(target_arch = "wasm32")]
    pub fn bind_to_dom(&self) {
        // This would use wasm_bindgen to attach event listeners
        // to DOM elements based on data attributes
    }
}

/// Main reactive app for WASM
#[allow(dead_code)]
pub struct WasmReactiveApp {
    differ: VDomDiffer,
    event_registry: WasmEventRegistry,
    root_element_id: String,
}

impl WasmReactiveApp {
    pub fn new(root_element_id: &str) -> Self {
        WasmReactiveApp {
            differ: VDomDiffer::new(),
            event_registry: WasmEventRegistry::new(),
            root_element_id: root_element_id.to_string(),
        }
    }

    /// Mount the app and set up reactivity
    #[cfg(target_arch = "wasm32")]
    pub fn mount(&self, initial_vdom: super::VNode) {
        // Render initial VDom to real DOM
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(root) = document.get_element_by_id(&self.root_element_id) {
                    // Clear existing content
                    root.set_inner_html("");

                    // Render VNode tree
                    if let Some(element) = self.differ.vnode_to_element(&initial_vdom, &document) {
                        let _ = root.append_child(&element);
                    }
                }
            }
        }

        // Set up event bindings
        self.event_registry.bind_to_dom();
    }

    /// Update the app with a new VDom tree
    #[cfg(target_arch = "wasm32")]
    pub fn update(&self, old_vdom: &super::VNode, new_vdom: &super::VNode) {
        let patches = self.differ.diff(old_vdom, new_vdom);
        self.differ.apply_patches(patches, &self.root_element_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{VElement, VNode, VText};

    #[test]
    fn test_diff_text_change() {
        let differ = VDomDiffer::new();
        let old = VNode::Text(VText::new("Hello"));
        let new = VNode::Text(VText::new("World"));

        let patches = differ.diff(&old, &new);
        assert_eq!(patches.len(), 1);

        match &patches[0] {
            DomPatch::UpdateText { new_text, .. } => {
                assert_eq!(new_text, "World");
            }
            _ => panic!("Expected UpdateText patch"),
        }
    }

    #[test]
    fn test_diff_attribute_change() {
        let differ = VDomDiffer::new();

        let mut old_el = VElement::new("div");
        old_el
            .attributes
            .insert("class".to_string(), "old".to_string());

        let mut new_el = VElement::new("div");
        new_el
            .attributes
            .insert("class".to_string(), "new".to_string());

        let patches = differ.diff(&VNode::Element(old_el), &VNode::Element(new_el));

        assert!(patches
            .iter()
            .any(|p| matches!(p, DomPatch::SetAttribute { key, value, .. }
            if key == "class" && value == "new")));
    }
}
