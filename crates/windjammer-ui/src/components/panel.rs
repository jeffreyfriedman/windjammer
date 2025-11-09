//! Panel component for sections
use crate::simple_vnode::{VAttr, VNode};

pub struct Panel {
    pub title: String,
    pub children: Vec<VNode>,
}

impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
        }
    }
    
    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }
    
    pub fn children(mut self, children: Vec<VNode>) -> Self {
        self.children = children;
        self
    }
    
    pub fn render(&self) -> VNode {
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static("wj-panel".to_string())),
            ],
            children: vec![
                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![
                        ("class".to_string(), VAttr::Static("wj-panel-header".to_string())),
                    ],
                    children: vec![VNode::Text(self.title.clone())],
                },
                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![
                        ("class".to_string(), VAttr::Static("wj-panel-body".to_string())),
                    ],
                    children: self.children.clone(),
                },
            ],
        }
    }
}
