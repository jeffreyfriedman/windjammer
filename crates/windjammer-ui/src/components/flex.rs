//! Flex layout component
use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};

pub enum FlexDirection {
    Row,
    Column,
}

pub struct Flex {
    pub children: Vec<VNode>,
    pub direction: FlexDirection,
    pub gap: String,
    pub padding: Option<String>,
    pub background_color: Option<String>,
}

impl Flex {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            direction: FlexDirection::Row,
            gap: "8px".to_string(),
            padding: None,
            background_color: None,
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

    pub fn padding(mut self, padding: impl Into<String>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    pub fn background_color(mut self, color: impl Into<String>) -> Self {
        self.background_color = Some(color.into());
        self
    }

    pub fn render(&self) -> VNode {
        let direction_str = match self.direction {
            FlexDirection::Row => "row",
            FlexDirection::Column => "column",
        };

        let mut style = format!(
            "display: flex; flex-direction: {}; gap: {};",
            direction_str, self.gap
        );

        if let Some(ref p) = self.padding {
            style.push_str(&format!(" padding: {};", p));
        }

        if let Some(ref bg) = self.background_color {
            style.push_str(&format!(" background-color: {};", bg));
        }

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

impl ToVNode for Flex {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
