use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};

/// SplitPanel component for resizable split layouts
pub struct SplitPanel {
    direction: SplitDirection,
    left_or_top: Option<VNode>,
    right_or_bottom: Option<VNode>,
    split_ratio: f32, // 0.0 to 1.0
}

#[derive(Clone, Copy)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

impl SplitPanel {
    pub fn new(direction: SplitDirection) -> Self {
        Self {
            direction,
            left_or_top: None,
            right_or_bottom: None,
            split_ratio: 0.5,
        }
    }

    pub fn left(mut self, content: VNode) -> Self {
        self.left_or_top = Some(content);
        self
    }

    pub fn top(mut self, content: VNode) -> Self {
        self.left_or_top = Some(content);
        self
    }

    pub fn right(mut self, content: VNode) -> Self {
        self.right_or_bottom = Some(content);
        self
    }

    pub fn bottom(mut self, content: VNode) -> Self {
        self.right_or_bottom = Some(content);
        self
    }

    pub fn split_ratio(mut self, ratio: f32) -> Self {
        self.split_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    pub fn render(&self) -> VNode {
        let class = match self.direction {
            SplitDirection::Horizontal => "split-panel horizontal",
            SplitDirection::Vertical => "split-panel vertical",
        };

        let mut children = Vec::new();

        if let Some(left) = &self.left_or_top {
            children.push(VNode::Element {
                tag: "div".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("split-pane first".to_string()),
                )],
                children: vec![left.clone()],
            });
        }

        if let Some(right) = &self.right_or_bottom {
            children.push(VNode::Element {
                tag: "div".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("split-pane second".to_string()),
                )],
                children: vec![right.clone()],
            });
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static(class.to_string()))],
            children,
        }
    }
}

impl ToVNode for SplitPanel {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
