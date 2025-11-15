//! Badge component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;

#[derive(Clone, Debug, PartialEq)]
pub enum BadgeVariant {
    Default,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BadgeSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone)]
pub struct Badge {
    pub text: String,
    pub variant: BadgeVariant,
    pub size: BadgeSize,
}

impl Badge {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BadgeVariant::Default,
            size: BadgeSize::Medium,
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-badge".to_string()];

        // Add variant class
        classes.push(
            match self.variant {
                BadgeVariant::Default => "wj-badge-default",
                BadgeVariant::Primary => "wj-badge-primary",
                BadgeVariant::Success => "wj-badge-success",
                BadgeVariant::Warning => "wj-badge-warning",
                BadgeVariant::Danger => "wj-badge-danger",
                BadgeVariant::Info => "wj-badge-info",
            }
            .to_string(),
        );

        // Add size class
        classes.push(
            match self.size {
                BadgeSize::Small => "wj-badge-sm",
                BadgeSize::Medium => "wj-badge-md",
                BadgeSize::Large => "wj-badge-lg",
            }
            .to_string(),
        );

        VNode::Element {
            tag: "span".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static(classes.join(" ")))],
            children: vec![VNode::Text(self.text.clone())],
        }
    }
}

impl ToVNode for Badge {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
