//! Switch (toggle) component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum SwitchSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone)]
pub struct Switch {
    pub label: Option<String>,
    pub checked: bool,
    pub disabled: bool,
    pub size: SwitchSize,
    pub on_change: Option<Rc<RefCell<dyn FnMut(bool)>>>,
}

impl Switch {
    pub fn new() -> Self {
        Self {
            label: None,
            checked: false,
            disabled: false,
            size: SwitchSize::Medium,
            on_change: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change<F: FnMut(bool) + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        let mut container_classes = vec!["wj-switch-container".to_string()];

        // Add size class
        container_classes.push(
            match self.size {
                SwitchSize::Small => "wj-switch-sm",
                SwitchSize::Medium => "wj-switch-md",
                SwitchSize::Large => "wj-switch-lg",
            }
            .to_string(),
        );

        if self.disabled {
            container_classes.push("wj-switch-disabled".to_string());
        }

        // Create switch track
        let mut track_classes = vec!["wj-switch-track".to_string()];
        if self.checked {
            track_classes.push("wj-switch-checked".to_string());
        }

        let thumb = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-switch-thumb".to_string()),
            )],
            children: vec![],
        };

        let mut track_attrs = vec![
            ("class".to_string(), VAttr::Static(track_classes.join(" "))),
            ("role".to_string(), VAttr::Static("switch".to_string())),
            (
                "aria-checked".to_string(),
                VAttr::Static(if self.checked { "true" } else { "false" }.to_string()),
            ),
        ];

        if self.disabled {
            track_attrs.push((
                "aria-disabled".to_string(),
                VAttr::Static("true".to_string()),
            ));
        }

        // Add click handler if present
        // Note: Current event system doesn't support parameters, so we wrap the handler
        if let Some(ref handler) = self.on_change {
            let handler_clone = handler.clone();
            let new_checked = !self.checked;
            let wrapper = Rc::new(RefCell::new(move || {
                handler_clone.borrow_mut()(new_checked);
            }));
            track_attrs.push(("on_click".to_string(), VAttr::Event(wrapper)));
        }

        let track = VNode::Element {
            tag: "button".to_string(),
            attrs: track_attrs,
            children: vec![thumb],
        };

        // Create label if present
        let mut children = vec![track];
        if let Some(ref label_text) = self.label {
            let label = VNode::Element {
                tag: "span".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("wj-switch-label".to_string()),
                )],
                children: vec![VNode::Text(label_text.clone())],
            };
            children.push(label);
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static(container_classes.join(" ")),
            )],
            children,
        }
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new()
    }
}

impl ToVNode for Switch {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
