//! Checkbox component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum CheckboxSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone)]
pub struct Checkbox {
    pub label: String,
    pub checked: bool,
    pub disabled: bool,
    pub size: CheckboxSize,
    pub on_change: Option<Rc<RefCell<dyn FnMut(bool)>>>,
}

impl Checkbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            disabled: false,
            size: CheckboxSize::Medium,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change<F: FnMut(bool) + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-checkbox-container".to_string()];

        // Add size class
        classes.push(
            match self.size {
                CheckboxSize::Small => "wj-checkbox-sm",
                CheckboxSize::Medium => "wj-checkbox-md",
                CheckboxSize::Large => "wj-checkbox-lg",
            }
            .to_string(),
        );

        if self.disabled {
            classes.push("wj-checkbox-disabled".to_string());
        }

        // Create checkbox input
        let mut input_attrs = vec![
            ("type".to_string(), VAttr::Static("checkbox".to_string())),
            (
                "class".to_string(),
                VAttr::Static("wj-checkbox-input".to_string()),
            ),
        ];

        if self.checked {
            input_attrs.push(("checked".to_string(), VAttr::Static("true".to_string())));
        }

        if self.disabled {
            input_attrs.push(("disabled".to_string(), VAttr::Static("true".to_string())));
        }

        // Add change handler if present
        // Note: Current event system doesn't support parameters, so we wrap the handler
        if let Some(ref handler) = self.on_change {
            let handler_clone = handler.clone();
            let new_checked = !self.checked;
            let wrapper = Rc::new(RefCell::new(move || {
                handler_clone.borrow_mut()(new_checked);
            }));
            input_attrs.push(("on_change".to_string(), VAttr::Event(wrapper)));
        }

        let input = VNode::Element {
            tag: "input".to_string(),
            attrs: input_attrs,
            children: vec![],
        };

        // Create label text
        let label_text = VNode::Element {
            tag: "span".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-checkbox-label".to_string()),
            )],
            children: vec![VNode::Text(self.label.clone())],
        };

        // Wrap in label element
        VNode::Element {
            tag: "label".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static(classes.join(" ")))],
            children: vec![input, label_text],
        }
    }
}

impl ToVNode for Checkbox {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
