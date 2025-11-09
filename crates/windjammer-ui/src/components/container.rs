//! Container component

use crate::simple_vnode::{VAttr, VNode};

pub struct Container {
    pub children: Vec<VNode>,
    pub max_width: Option<String>,
    pub padding: Option<String>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            max_width: None,
            padding: Some("16px".to_string()),
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
    
    pub fn max_width(mut self, width: impl Into<String>) -> Self {
        self.max_width = Some(width.into());
        self
    }
    
    pub fn padding(mut self, padding: impl Into<String>) -> Self {
        self.padding = Some(padding.into());
        self
    }
    
    pub fn render(&self) -> VNode {
        let mut style = String::new();
        
        if let Some(ref max_width) = self.max_width {
            style.push_str(&format!("max-width: {}; ", max_width));
        }
        
        if let Some(ref padding) = self.padding {
            style.push_str(&format!("padding: {}; ", padding));
        }
        
        style.push_str("margin: 0 auto;");
        
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static("wj-container".to_string())),
                ("style".to_string(), VAttr::Static(style)),
            ],
            children: self.children.clone(),
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

