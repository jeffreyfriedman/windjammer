//! Card component
use crate::simple_vnode::{VAttr, VNode};

pub struct Card {
    pub children: Vec<VNode>,
}

impl Card {
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
            attrs: vec![("class".to_string(), VAttr::Static("wj-card".to_string()))],
            children: self.children.clone(),
        }
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}
