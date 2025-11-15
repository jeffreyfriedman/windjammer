//! Spinner (loading indicator) component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;

#[derive(Clone, Debug, PartialEq)]
pub enum SpinnerSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone)]
pub struct Spinner {
    pub size: SpinnerSize,
    pub label: Option<String>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: SpinnerSize::Medium,
            label: None,
        }
    }

    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-spinner".to_string()];

        classes.push(
            match self.size {
                SpinnerSize::Small => "wj-spinner-sm",
                SpinnerSize::Medium => "wj-spinner-md",
                SpinnerSize::Large => "wj-spinner-lg",
            }
            .to_string(),
        );

        let spinner = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static(classes.join(" ")))],
            children: vec![],
        };

        if let Some(ref label_text) = self.label {
            let label = VNode::Element {
                tag: "span".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("wj-spinner-label".to_string()),
                )],
                children: vec![VNode::Text(label_text.clone())],
            };

            VNode::Element {
                tag: "div".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("wj-spinner-container".to_string()),
                )],
                children: vec![spinner, label],
            }
        } else {
            spinner
        }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl ToVNode for Spinner {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
