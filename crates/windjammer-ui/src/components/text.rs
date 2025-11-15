//! Text component
use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};

pub enum TextSize {
    Small,
    Medium,
    Large,
    XLarge,
}

pub enum TextWeight {
    Normal,
    Bold,
}

pub struct Text {
    pub content: String,
    pub size: TextSize,
    pub weight: TextWeight,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: TextSize::Medium,
            weight: TextWeight::Normal,
        }
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    pub fn bold(mut self) -> Self {
        self.weight = TextWeight::Bold;
        self
    }

    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-text".to_string()];

        classes.push(
            match self.size {
                TextSize::Small => "wj-text-sm",
                TextSize::Medium => "wj-text-md",
                TextSize::Large => "wj-text-lg",
                TextSize::XLarge => "wj-text-xl",
            }
            .to_string(),
        );

        classes.push(
            match self.weight {
                TextWeight::Normal => "wj-text-normal",
                TextWeight::Bold => "wj-text-bold",
            }
            .to_string(),
        );

        VNode::Element {
            tag: "span".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static(classes.join(" ")))],
            children: vec![VNode::Text(self.content.clone())],
        }
    }
}

impl ToVNode for Text {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
