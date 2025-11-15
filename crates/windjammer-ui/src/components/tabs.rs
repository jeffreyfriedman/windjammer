//! Tabs component
use crate::simple_vnode::{VAttr, VNode};

pub struct Tab {
    pub label: String,
    pub content: VNode,
}

impl Tab {
    pub fn new(label: impl Into<String>, content: VNode) -> Self {
        Self {
            label: label.into(),
            content,
        }
    }
}

pub struct Tabs {
    pub tabs: Vec<Tab>,
    pub active_index: usize,
}

impl Tabs {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_index: 0,
        }
    }

    pub fn tab(mut self, tab: Tab) -> Self {
        self.tabs.push(tab);
        self
    }

    pub fn active(mut self, index: usize) -> Self {
        self.active_index = index;
        self
    }

    pub fn render(&self) -> VNode {
        let tab_headers: Vec<VNode> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let mut classes = vec!["wj-tab-header".to_string()];
                if i == self.active_index {
                    classes.push("wj-tab-header-active".to_string());
                }

                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![("class".to_string(), VAttr::Static(classes.join(" ")))],
                    children: vec![VNode::Text(tab.label.clone())],
                }
            })
            .collect();

        let active_content = if let Some(tab) = self.tabs.get(self.active_index) {
            tab.content.clone()
        } else {
            VNode::Text("".to_string())
        };

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static("wj-tabs".to_string()))],
            children: vec![
                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![
                        (
                            "class".to_string(),
                            VAttr::Static("wj-tabs-headers".to_string()),
                        ),
                        (
                            "style".to_string(),
                            VAttr::Static(
                                "display: flex; gap: 4px; border-bottom: 1px solid #ddd;"
                                    .to_string(),
                            ),
                        ),
                    ],
                    children: tab_headers,
                },
                VNode::Element {
                    tag: "div".to_string(),
                    attrs: vec![(
                        "class".to_string(),
                        VAttr::Static("wj-tabs-content".to_string()),
                    )],
                    children: vec![active_content],
                },
            ],
        }
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}
