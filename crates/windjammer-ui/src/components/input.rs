//! Input component
use crate::simple_vnode::{VAttr, VNode};

pub struct Input {
    pub value: String,
    pub placeholder: String,
}

impl Input {
    pub fn new() -> Self {
        Self { value: String::new(), placeholder: String::new() }
    }
    
    pub fn render(&self) -> VNode {
        VNode::Element {
            tag: "input".to_string(),
            attrs: vec![
                VAttr { name: "class".to_string(), value: "wj-input".to_string() },
                VAttr { name: "value".to_string(), value: self.value.clone() },
                VAttr { name: "placeholder".to_string(), value: self.placeholder.clone() },
            ],
            children: vec![],
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
