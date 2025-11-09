//! Flex layout component
use crate::simple_vnode::{VAttr, VNode};

pub enum FlexDirection {
    Row,
    Column,
}

pub struct Flex {
    pub children: Vec<VNode>,
    pub direction: FlexDirection,
    pub gap: String,
}

impl Flex {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            direction: FlexDirection::Row,
            gap: "8px".to_string(),
        }
    }
    
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }
    
    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }
    
    pub fn children(mut self, children: Vec<VNode>) -> Self {
        self.children = children;
        self
    }
    
    pub fn gap(mut self, gap: impl Into<String>) -> Self {
        self.gap = gap.into();
        self
    }
    
    pub fn render(&self) -> VNode {
        let direction_str = match self.direction {
            FlexDirection::Row => "row",
            FlexDirection::Column => "column",
        };
        
        let style = format!(
            "display: flex; flex-direction: {}; gap: {};",
            direction_str, self.gap
        );
        
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static("wj-flex".to_string())),
                ("style".to_string(), VAttr::Static(style)),
            ],
            children: self.children.clone(),
        }
    }
}

impl Default for Flex {
    fn default() -> Self {
        Self::new()
    }
}
