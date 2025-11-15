//! Grid layout component
use crate::simple_vnode::{VAttr, VNode};

pub struct Grid {
    pub children: Vec<VNode>,
    pub columns: usize,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            columns: 2,
        }
    }

    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn render(&self) -> VNode {
        let style = format!(
            "display: grid; grid-template-columns: repeat({}, 1fr); gap: 16px;",
            self.columns
        );

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static("wj-grid".to_string())),
                ("style".to_string(), VAttr::Static(style)),
            ],
            children: self.children.clone(),
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}
