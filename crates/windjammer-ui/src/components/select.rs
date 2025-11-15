//! Select (dropdown) component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
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
pub struct Select {
    pub placeholder: Option<String>,
    pub options: Vec<SelectOption>,
    pub value: Option<String>,
    pub disabled: bool,
    pub on_change: Option<Rc<RefCell<dyn FnMut(String)>>>,
}

impl Select {
    pub fn new() -> Self {
        Self {
            placeholder: None,
            options: Vec::new(),
            value: None,
            disabled: false,
            on_change: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(SelectOption::new(value, label));
        self
    }

    pub fn option_with(mut self, option: SelectOption) -> Self {
        self.options.push(option);
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
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

        // Add placeholder option if present
        if let Some(ref placeholder_text) = self.placeholder {
            let placeholder_option = VNode::Element {
                tag: "option".to_string(),
                attrs: vec![
                    ("value".to_string(), VAttr::Static("".to_string())),
                    ("disabled".to_string(), VAttr::Static("true".to_string())),
                    ("selected".to_string(), VAttr::Static("true".to_string())),
                ],
                children: vec![VNode::Text(placeholder_text.clone())],
            };
            children.push(placeholder_option);
        }

        // Add options
        for option in &self.options {
            let is_selected = self.value.as_ref() == Some(&option.value);

            let mut option_attrs = vec![("value".to_string(), VAttr::Static(option.value.clone()))];

            if is_selected {
                option_attrs.push(("selected".to_string(), VAttr::Static("true".to_string())));
            }

            if option.disabled {
                option_attrs.push(("disabled".to_string(), VAttr::Static("true".to_string())));
            }

            let select_option = VNode::Element {
                tag: "option".to_string(),
                attrs: option_attrs,
                children: vec![VNode::Text(option.label.clone())],
            };

            children.push(select_option);
        }

        let mut select_attrs = vec![("class".to_string(), VAttr::Static("wj-select".to_string()))];

        if self.disabled {
            select_attrs.push(("disabled".to_string(), VAttr::Static("true".to_string())));
        }

        // Add change handler if present
        // Note: Current event system doesn't support parameters, so this is a placeholder
        // In a real implementation, we'd need to get the selected value from the event
        if let Some(ref _handler) = self.on_change {
            // TODO: Implement proper event handling with value extraction
            // For now, select changes won't trigger the handler
        }

        VNode::Element {
            tag: "select".to_string(),
            attrs: select_attrs,
            children,
        }
    }
}

impl Default for Select {
    fn default() -> Self {
        Self::new()
    }
}

impl ToVNode for Select {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
