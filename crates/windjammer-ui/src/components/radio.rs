//! Radio button and Radio Group components

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct RadioOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl RadioOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone)]
pub struct RadioGroup {
    pub name: String,
    pub options: Vec<RadioOption>,
    pub selected: Option<String>,
    pub disabled: bool,
    pub on_change: Option<Rc<RefCell<dyn FnMut(String)>>>,
}

impl RadioGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            options: Vec::new(),
            selected: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(RadioOption::new(value, label));
        self
    }

    pub fn option_with(mut self, option: RadioOption) -> Self {
        self.options.push(option);
        self
    }

    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change<F: FnMut(String) + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        let mut children = Vec::new();

        for option in &self.options {
            let is_checked = self.selected.as_ref() == Some(&option.value);
            let is_disabled = self.disabled || option.disabled;

            // Create radio input
            let mut input_attrs = vec![
                ("type".to_string(), VAttr::Static("radio".to_string())),
                ("name".to_string(), VAttr::Static(self.name.clone())),
                ("value".to_string(), VAttr::Static(option.value.clone())),
                (
                    "class".to_string(),
                    VAttr::Static("wj-radio-input".to_string()),
                ),
            ];

            if is_checked {
                input_attrs.push(("checked".to_string(), VAttr::Static("true".to_string())));
            }

            if is_disabled {
                input_attrs.push(("disabled".to_string(), VAttr::Static("true".to_string())));
            }

            // Add change handler if present
            // Note: Current event system doesn't support parameters, so we wrap the handler
            if let Some(ref handler) = self.on_change {
                let handler_clone = handler.clone();
                let value_clone = option.value.clone();
                let wrapper = Rc::new(RefCell::new(move || {
                    handler_clone.borrow_mut()(value_clone.clone());
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
                    VAttr::Static("wj-radio-label".to_string()),
                )],
                children: vec![VNode::Text(option.label.clone())],
            };

            // Wrap in label element
            let mut label_classes = vec!["wj-radio-option".to_string()];
            if is_disabled {
                label_classes.push("wj-radio-disabled".to_string());
            }

            let radio_option = VNode::Element {
                tag: "label".to_string(),
                attrs: vec![("class".to_string(), VAttr::Static(label_classes.join(" ")))],
                children: vec![input, label_text],
            };

            children.push(radio_option);
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-radio-group".to_string()),
            )],
            children,
        }
    }
}

impl ToVNode for RadioGroup {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
