//! Progress bar component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;

#[derive(Clone, Debug, PartialEq)]
pub enum ProgressVariant {
    Default,
    Success,
    Warning,
    Danger,
}

#[derive(Clone)]
pub struct Progress {
    pub value: f64,
    pub max: f64,
    pub variant: ProgressVariant,
    pub show_label: bool,
}

impl Progress {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            max: 100.0,
            variant: ProgressVariant::Default,
            show_label: true,
        }
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    pub fn variant(mut self, variant: ProgressVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    pub fn render(&self) -> VNode {
        let percentage = (self.value / self.max * 100.0).min(100.0).max(0.0);

        let mut bar_classes = vec!["wj-progress-bar".to_string()];
        bar_classes.push(
            match self.variant {
                ProgressVariant::Default => "wj-progress-default",
                ProgressVariant::Success => "wj-progress-success",
                ProgressVariant::Warning => "wj-progress-warning",
                ProgressVariant::Danger => "wj-progress-danger",
            }
            .to_string(),
        );

        let bar = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static(bar_classes.join(" "))),
                (
                    "style".to_string(),
                    VAttr::Static(format!("width: {}%", percentage)),
                ),
            ],
            children: if self.show_label {
                vec![VNode::Text(format!("{:.0}%", percentage))]
            } else {
                vec![]
            },
        };

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-progress-container".to_string()),
            )],
            children: vec![bar],
        }
    }
}

impl ToVNode for Progress {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
