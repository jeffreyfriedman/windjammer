//! Dialog (modal) component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct Dialog {
    pub title: String,
    pub open: bool,
    pub children: Vec<VNode>,
    pub on_close: Option<Rc<RefCell<dyn FnMut()>>>,
}

impl Dialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: false,
            children: Vec::new(),
            on_close: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn child(mut self, child: impl ToVNode) -> Self {
        self.children.push(child.to_vnode());
        self
    }

    pub fn on_close<F: FnMut() + 'static>(mut self, handler: F) -> Self {
        self.on_close = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        if !self.open {
            return VNode::Element {
                tag: "div".to_string(),
                attrs: vec![(
                    "style".to_string(),
                    VAttr::Static("display: none;".to_string()),
                )],
                children: vec![],
            };
        }

        // Create overlay
        let mut overlay_attrs = vec![(
            "class".to_string(),
            VAttr::Static("wj-dialog-overlay".to_string()),
        )];

        if let Some(ref handler) = self.on_close {
            overlay_attrs.push(("on_click".to_string(), VAttr::Event(handler.clone())));
        }

        let overlay = VNode::Element {
            tag: "div".to_string(),
            attrs: overlay_attrs,
            children: vec![],
        };

        // Create close button
        let close_button_children = vec![VNode::Text("×".to_string())];
        let mut close_button_attrs = vec![(
            "class".to_string(),
            VAttr::Static("wj-dialog-close".to_string()),
        )];

        if let Some(ref handler) = self.on_close {
            close_button_attrs.push(("on_click".to_string(), VAttr::Event(handler.clone())));
        }

        let close_button = VNode::Element {
            tag: "button".to_string(),
            attrs: close_button_attrs,
            children: close_button_children,
        };

        // Create header
        let title_node = VNode::Element {
            tag: "h2".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-dialog-title".to_string()),
            )],
            children: vec![VNode::Text(self.title.clone())],
        };

        let header = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-dialog-header".to_string()),
            )],
            children: vec![title_node, close_button],
        };

        // Create content
        let content = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-dialog-content".to_string()),
            )],
            children: self.children.clone(),
        };

        // Create dialog container
        let dialog = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static("wj-dialog".to_string()))],
            children: vec![header, content],
        };

        // Wrap everything
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-dialog-container".to_string()),
            )],
            children: vec![overlay, dialog],
        }
    }
}

impl ToVNode for Dialog {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
