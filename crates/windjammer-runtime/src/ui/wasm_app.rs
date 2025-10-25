// Complete WASM-based reactive application runtime
// This provides a zero-JavaScript UI framework that runs entirely in WebAssembly

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Document, Element, Event, HtmlElement, KeyboardEvent, MouseEvent};

use super::{Signal, VElement, VNode, VText};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Main WASM application that manages reactive UI
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmApp {
    root_id: String,
    state: Rc<RefCell<AppState>>,
    event_handlers: Rc<RefCell<HashMap<String, Vec<EventHandler>>>>,
}

#[cfg(target_arch = "wasm32")]
struct AppState {
    vdom: Option<VNode>,
    mounted_elements: HashMap<String, Element>,
}

#[cfg(target_arch = "wasm32")]
struct EventHandler {
    event_type: String,
    element_id: String,
    callback: Rc<dyn Fn(Event)>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmApp {
    #[wasm_bindgen(constructor)]
    pub fn new(root_id: String) -> Result<WasmApp, JsValue> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        Ok(WasmApp {
            root_id,
            state: Rc::new(RefCell::new(AppState {
                vdom: None,
                mounted_elements: HashMap::new(),
            })),
            event_handlers: Rc::new(RefCell::new(HashMap::new())),
        })
    }

    /// Mount the application to the DOM
    pub fn mount(&mut self, vnode: VNode) -> Result<(), JsValue> {
        let window = window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let root = document
            .get_element_by_id(&self.root_id)
            .ok_or("Root element not found")?;

        // Clear existing content
        root.set_inner_html("");

        // Render VNode to DOM
        let element = self.render_vnode(&document, &vnode)?;
        root.append_child(&element)?;

        // Store in state
        self.state.borrow_mut().vdom = Some(vnode);

        Ok(())
    }

    /// Update the application with a new VNode tree
    pub fn update(&mut self, new_vnode: VNode) -> Result<(), JsValue> {
        let window = window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let root = document
            .get_element_by_id(&self.root_id)
            .ok_or("Root element not found")?;

        let mut state = self.state.borrow_mut();

        if let Some(old_vnode) = &state.vdom {
            // Diff and patch
            let patches = self.diff_vnodes(old_vnode, &new_vnode);
            self.apply_patches(&document, &root, &patches)?;
        } else {
            // First render
            root.set_inner_html("");
            let element = self.render_vnode(&document, &new_vnode)?;
            root.append_child(&element)?;
        }

        state.vdom = Some(new_vnode);
        Ok(())
    }

    /// Render a VNode to a DOM element
    fn render_vnode(&self, document: &Document, vnode: &VNode) -> Result<Element, JsValue> {
        match vnode {
            VNode::Element(el) => {
                let element = document.create_element(&el.tag)?;

                // Set attributes
                for (key, value) in &el.attributes {
                    element.set_attribute(key, value)?;
                }

                // Add children
                for child in &el.children {
                    let child_element = self.render_vnode(document, child)?;
                    element.append_child(&child_element)?;
                }

                Ok(element)
            }
            VNode::Text(text) => {
                let text_node = document.create_text_node(&text.content);
                Ok(text_node.into())
            }
            VNode::Component(_) => {
                // Components should be resolved before rendering
                Err("Unresolved component".into())
            }
        }
    }

    /// Diff two VNode trees
    fn diff_vnodes(&self, old: &VNode, new: &VNode) -> Vec<Patch> {
        let mut patches = Vec::new();
        self.diff_recursive(old, new, vec![0], &mut patches);
        patches
    }

    fn diff_recursive(&self, old: &VNode, new: &VNode, path: Vec<usize>, patches: &mut Vec<Patch>) {
        match (old, new) {
            (VNode::Text(old_text), VNode::Text(new_text)) => {
                if old_text.content != new_text.content {
                    patches.push(Patch::SetText {
                        path: path.clone(),
                        text: new_text.content.clone(),
                    });
                }
            }
            (VNode::Element(old_el), VNode::Element(new_el)) => {
                // Check tag
                if old_el.tag != new_el.tag {
                    patches.push(Patch::Replace {
                        path: path.clone(),
                        new_vnode: new.clone(),
                    });
                    return;
                }

                // Diff attributes
                for (key, new_value) in &new_el.attributes {
                    if old_el.attributes.get(key) != Some(new_value) {
                        patches.push(Patch::SetAttribute {
                            path: path.clone(),
                            key: key.clone(),
                            value: new_value.clone(),
                        });
                    }
                }

                for key in old_el.attributes.keys() {
                    if !new_el.attributes.contains_key(key) {
                        patches.push(Patch::RemoveAttribute {
                            path: path.clone(),
                            key: key.clone(),
                        });
                    }
                }

                // Diff children
                let old_len = old_el.children.len();
                let new_len = new_el.children.len();
                let min_len = old_len.min(new_len);

                for i in 0..min_len {
                    let mut child_path = path.clone();
                    child_path.push(i);
                    self.diff_recursive(
                        &old_el.children[i],
                        &new_el.children[i],
                        child_path,
                        patches,
                    );
                }

                // Handle length differences
                if new_len > old_len {
                    for i in old_len..new_len {
                        let mut child_path = path.clone();
                        child_path.push(i);
                        patches.push(Patch::AppendChild {
                            path: path.clone(),
                            new_vnode: new_el.children[i].clone(),
                        });
                    }
                } else if old_len > new_len {
                    for i in new_len..old_len {
                        let mut child_path = path.clone();
                        child_path.push(i);
                        patches.push(Patch::RemoveChild { path: child_path });
                    }
                }
            }
            _ => {
                // Different node types, replace
                patches.push(Patch::Replace {
                    path: path.clone(),
                    new_vnode: new.clone(),
                });
            }
        }
    }

    /// Apply patches to the DOM
    fn apply_patches(
        &self,
        document: &Document,
        root: &Element,
        patches: &[Patch],
    ) -> Result<(), JsValue> {
        for patch in patches {
            match patch {
                Patch::SetText { path, text } => {
                    if let Some(element) = self.find_element_by_path(root, path) {
                        element.set_text_content(Some(text));
                    }
                }
                Patch::SetAttribute { path, key, value } => {
                    if let Some(element) = self.find_element_by_path(root, path) {
                        element.set_attribute(key, value)?;
                    }
                }
                Patch::RemoveAttribute { path, key } => {
                    if let Some(element) = self.find_element_by_path(root, path) {
                        element.remove_attribute(key)?;
                    }
                }
                Patch::Replace { path, new_vnode } => {
                    if let Some(old_element) = self.find_element_by_path(root, path) {
                        let new_element = self.render_vnode(document, new_vnode)?;
                        if let Some(parent) = old_element.parent_element() {
                            parent.replace_child(&new_element, &old_element)?;
                        }
                    }
                }
                Patch::AppendChild { path, new_vnode } => {
                    if let Some(parent) = self.find_element_by_path(root, path) {
                        let new_element = self.render_vnode(document, new_vnode)?;
                        parent.append_child(&new_element)?;
                    }
                }
                Patch::RemoveChild { path } => {
                    if let Some(element) = self.find_element_by_path(root, path) {
                        if let Some(parent) = element.parent_element() {
                            parent.remove_child(&element)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn find_element_by_path(&self, root: &Element, path: &[usize]) -> Option<Element> {
        if path.is_empty() {
            return Some(root.clone());
        }

        let mut current = root.clone();
        for &index in &path[1..] {
            let children = current.children();
            if index < children.length() as usize {
                current = children.item(index as u32)?.into();
            } else {
                return None;
            }
        }
        Some(current)
    }
}

#[derive(Clone, Debug)]
enum Patch {
    SetText {
        path: Vec<usize>,
        text: String,
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
    Replace {
        path: Vec<usize>,
        new_vnode: VNode,
    },
    AppendChild {
        path: Vec<usize>,
        new_vnode: VNode,
    },
    RemoveChild {
        path: Vec<usize>,
    },
}

// Non-WASM stubs for compilation on other targets
#[cfg(not(target_arch = "wasm32"))]
pub struct WasmApp;

#[cfg(not(target_arch = "wasm32"))]
impl WasmApp {
    pub fn new(_root_id: String) -> Result<Self, String> {
        Ok(WasmApp)
    }
}
