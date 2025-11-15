//! Toolbar component
use crate::simple_vnode::{VAttr, VNode};

pub struct Toolbar {
    pub children: Vec<VNode>,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn render(&self) -> VNode {
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static("wj-toolbar".to_string())),
                (
                    "style".to_string(),
                    VAttr::Static(
                        "display: flex; gap: 8px; padding: 8px; border-bottom: 1px solid #ddd;"
                            .to_string(),
                    ),
                ),
            ],
            children: self.children.clone(),
        }
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}
