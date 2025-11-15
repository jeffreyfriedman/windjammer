//! Slider (range input) component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct Slider {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub value: f64,
    pub disabled: bool,
    pub label: Option<String>,
    pub on_change: Option<Rc<RefCell<dyn FnMut(f64)>>>,
}

impl Slider {
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: 50.0,
            disabled: false,
            label: None,
            on_change: None,
        }
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn on_change<F: FnMut(f64) + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        let mut input_attrs = vec![
            ("type".to_string(), VAttr::Static("range".to_string())),
            (
                "class".to_string(),
                VAttr::Static("wj-slider-input".to_string()),
            ),
            ("min".to_string(), VAttr::Static(self.min.to_string())),
            ("max".to_string(), VAttr::Static(self.max.to_string())),
            ("step".to_string(), VAttr::Static(self.step.to_string())),
            ("value".to_string(), VAttr::Static(self.value.to_string())),
        ];

        if self.disabled {
            input_attrs.push(("disabled".to_string(), VAttr::Static("true".to_string())));
        }

        // Note: Current event system doesn't support parameters
        // In a real implementation, we'd extract the value from the event
        if let Some(ref _handler) = self.on_change {
            // TODO: Implement proper event handling with value extraction
        }

        let input = VNode::Element {
            tag: "input".to_string(),
            attrs: input_attrs,
            children: vec![],
        };

        let value_display = VNode::Element {
            tag: "span".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-slider-value".to_string()),
            )],
            children: vec![VNode::Text(format!("{:.1}", self.value))],
        };

        let mut children = vec![input, value_display];

        // Add label if present
        if let Some(ref label_text) = self.label {
            let label = VNode::Element {
                tag: "label".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("wj-slider-label".to_string()),
                )],
                children: vec![VNode::Text(label_text.clone())],
            };
            children.insert(0, label);
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-slider-container".to_string()),
            )],
            children,
        }
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new()
    }
}

impl ToVNode for Slider {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
