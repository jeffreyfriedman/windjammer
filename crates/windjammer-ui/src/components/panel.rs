//! Panel component for sections
use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};

pub struct Panel {
    pub title: String,
    pub children: Vec<VNode>,
    pub background_color: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub padding: Option<String>,
}

impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
            background_color: None,
            width: None,
            height: None,
            padding: None,
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

    pub fn background_color(mut self, color: impl Into<String>) -> Self {
        self.background_color = Some(color.into());
        self
    }

    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<String>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn padding(mut self, padding: impl Into<String>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    pub fn render(&self) -> VNode {
        let mut style_parts = Vec::new();
        if let Some(bg) = &self.background_color {
            style_parts.push(format!("background-color: {}", bg));
        }
        if let Some(w) = &self.width {
            style_parts.push(format!("width: {}", w));
        }
        if let Some(h) = &self.height {
            style_parts.push(format!("height: {}", h));
        }
        if let Some(p) = &self.padding {
            style_parts.push(format!("padding: {}", p));
        }

        let style = if style_parts.is_empty() {
            None
        } else {
            Some(style_parts.join("; "))
        };

        let mut attrs = vec![("class".to_string(), VAttr::Static("wj-panel".to_string()))];
        if let Some(s) = style {
            attrs.push(("style".to_string(), VAttr::Static(s)));
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs,
            children: vec![
                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![(
                        "class".to_string(),
                        VAttr::Static("wj-panel-header".to_string()),
                    )],
                    children: vec![VNode::Text(self.title.clone())],
                },
                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![(
                        "class".to_string(),
                        VAttr::Static("wj-panel-body".to_string()),
                    )],
                    children: self.children.clone(),
                },
            ],
        }
    }
}

impl ToVNode for Panel {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
