//! Button component

use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Success,
    Danger,
    Ghost,
}

#[derive(Clone, Debug)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone)]
pub struct Button {
    pub label: String,
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub disabled: bool,
    pub on_click: Option<Rc<RefCell<dyn FnMut()>>>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Primary,
            size: ButtonSize::Medium,
            disabled: false,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click<F: FnMut() + 'static>(mut self, handler: F) -> Self {
        self.on_click = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-button".to_string()];

        // Add variant class
        classes.push(
            match self.variant {
                ButtonVariant::Primary => "wj-button-primary",
                ButtonVariant::Secondary => "wj-button-secondary",
                ButtonVariant::Success => "wj-button-success",
                ButtonVariant::Danger => "wj-button-danger",
                ButtonVariant::Ghost => "wj-button-ghost",
            }
            .to_string(),
        );

        // Add size class
        classes.push(
            match self.size {
                ButtonSize::Small => "wj-button-sm",
                ButtonSize::Medium => "wj-button-md",
                ButtonSize::Large => "wj-button-lg",
            }
            .to_string(),
        );

        // Add disabled class
        if self.disabled {
            classes.push("wj-button-disabled".to_string());
        }

        let mut attrs = vec![
            ("class".to_string(), VAttr::Static(classes.join(" "))),
            (
                "disabled".to_string(),
                VAttr::Static(if self.disabled { "true" } else { "false" }.to_string()),
            ),
        ];

        // Add event handler if present
        if let Some(handler) = &self.on_click {
            attrs.push(("on_click".to_string(), VAttr::Event(handler.clone())));
        }

        VNode::Element {
            tag: "button".to_string(),
            attrs,
            children: vec![VNode::Text(self.label.clone())],
        }
    }
}

impl ToVNode for Button {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
