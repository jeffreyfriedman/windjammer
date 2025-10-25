//! Simple VNode implementation for generated component code
//!
//! This is a minimal virtual DOM implementation specifically for compiled components.
//! It's designed to be simple and transparent, not a full-featured virtual DOM.

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Document, Element, Event};

/// A simple virtual DOM node
#[derive(Clone)]
pub enum VNode {
    Element {
        tag: String,
        attrs: Vec<(String, VAttr)>,
        children: Vec<VNode>,
    },
    Text(String),
}

/// Attribute value
#[derive(Clone)]
pub enum VAttr {
    Static(String),
    Dynamic(String),
    Event(Rc<RefCell<dyn FnMut()>>),
}

impl VNode {
    /// Create an element node
    pub fn element(tag: &str, attrs: Vec<(&str, VAttr)>, children: Vec<VNode>) -> Self {
        VNode::Element {
            tag: tag.to_string(),
            attrs: attrs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            children,
        }
    }

    /// Create a text node
    pub fn text(content: &str) -> Self {
        VNode::Text(content.to_string())
    }

    /// Render this VNode to the DOM
    #[cfg(target_arch = "wasm32")]
    pub fn render(&self, document: &Document) -> Result<web_sys::Node, JsValue> {
        match self {
            VNode::Element {
                tag,
                attrs,
                children,
            } => {
                let element = document.create_element(tag)?;

                // Set attributes
                for (name, value) in attrs {
                    match value {
                        VAttr::Static(v) | VAttr::Dynamic(v) => {
                            if name == "class" {
                                element.set_attribute("class", v)?;
                            } else if name.starts_with("on_") {
                                // Event handlers are handled separately
                            } else {
                                element.set_attribute(name, v)?;
                            }
                        }
                        VAttr::Event(handler) => {
                            if let Some(event_name) = name.strip_prefix("on_") {
                                let handler_clone = handler.clone();
                                let closure = Closure::wrap(Box::new(move |_event: Event| {
                                    handler_clone.borrow_mut()();
                                })
                                    as Box<dyn FnMut(Event)>);

                                element.add_event_listener_with_callback(
                                    event_name,
                                    closure.as_ref().unchecked_ref(),
                                )?;
                                closure.forget(); // Keep the closure alive
                            }
                        }
                    }
                }

                // Render children
                for child in children {
                    let child_node = child.render(document)?;
                    element.append_child(&child_node)?;
                }

                Ok(element.into())
            }
            VNode::Text(content) => {
                let text_node = document.create_text_node(content);
                Ok(text_node.into())
            }
        }
    }

    /// Mount this VNode to a parent element
    #[cfg(target_arch = "wasm32")]
    pub fn mount(&self, parent: &Element) -> Result<(), JsValue> {
        let document = parent
            .owner_document()
            .ok_or_else(|| JsValue::from_str("No document"))?;
        let node = self.render(&document)?;
        parent.append_child(&node)?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl VNode {
    pub fn render(&self) -> Result<String, String> {
        match self {
            VNode::Element {
                tag,
                attrs: _,
                children,
            } => {
                let mut html = format!("<{}>", tag);
                for child in children {
                    html.push_str(&child.render()?);
                }
                html.push_str(&format!("</{}>", tag));
                Ok(html)
            }
            VNode::Text(content) => Ok(content.clone()),
        }
    }
}
