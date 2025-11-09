//! Component stub
use crate::simple_vnode::{VAttr, VNode};

pub struct Component {
    pub children: Vec<VNode>,
}

impl Component {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }
    
    pub fn render(&self) -> VNode {
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![VAttr { name: "class".to_string(), value: "wj-component".to_string() }],
            children: self.children.clone(),
        }
    }
}

impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}
