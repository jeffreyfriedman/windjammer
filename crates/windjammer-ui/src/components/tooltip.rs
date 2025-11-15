//! Tooltip component

use crate::simple_vnode::{VAttr, VNode};
use crate::to_vnode::ToVNode;

#[derive(Clone, Debug, PartialEq)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone)]
pub struct Tooltip {
    pub text: String,
    pub position: TooltipPosition,
    pub child: Option<Box<VNode>>,
}

impl Tooltip {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            position: TooltipPosition::Top,
            child: None,
        }
    }

    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    pub fn child(mut self, child: impl ToVNode) -> Self {
        self.child = Some(Box::new(child.to_vnode()));
        self
    }

    pub fn render(&self) -> VNode {
        let position_class = match self.position {
            TooltipPosition::Top => "wj-tooltip-top",
            TooltipPosition::Bottom => "wj-tooltip-bottom",
            TooltipPosition::Left => "wj-tooltip-left",
            TooltipPosition::Right => "wj-tooltip-right",
        };

        let tooltip_text = VNode::Element {
            tag: "span".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static(format!("wj-tooltip-text {}", position_class)),
            )],
            children: vec![VNode::Text(self.text.clone())],
        };

        let mut children = vec![];
        if let Some(ref child) = self.child {
            children.push(*child.clone());
        }
        children.push(tooltip_text);

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-tooltip-container".to_string()),
            )],
            children,
        }
    }
}

impl ToVNode for Tooltip {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
