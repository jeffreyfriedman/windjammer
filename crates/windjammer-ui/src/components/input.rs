//! Input component
use crate::simple_vnode::{VAttr, VNode};

pub struct Input {
    pub value: String,
    pub placeholder: String,
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
        }
    }

    pub fn render(&self) -> VNode {
        VNode::Element {
            tag: "input".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static("wj-input".to_string())),
                ("value".to_string(), VAttr::Static(self.value.clone())),
                (
                    "placeholder".to_string(),
                    VAttr::Static(self.placeholder.clone()),
                ),
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
