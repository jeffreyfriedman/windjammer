use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};
use std::cell::RefCell;
use std::rc::Rc;

/// TreeView component for displaying hierarchical data (file trees, etc.)
pub struct TreeView {
    items: Vec<TreeItem>,
}

#[derive(Clone)]
pub struct TreeItem {
    pub label: String,
    pub icon: String,
    pub children: Vec<TreeItem>,
    pub expanded: bool,
    #[allow(clippy::type_complexity)]
    pub on_click: Option<Rc<RefCell<dyn FnMut()>>>,
}

impl TreeView {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn items(mut self, items: Vec<TreeItem>) -> Self {
        self.items = items;
        self
    }

    pub fn render(&self) -> VNode {
        let mut children = Vec::new();
        for item in &self.items {
            children.push(self.render_item(item, 0));
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static("tree-view".to_string()))],
            children,
        }
    }

    fn render_item(&self, item: &TreeItem, depth: usize) -> VNode {
        let mut children = vec![VNode::Text(format!(
            "{}{} {}",
            "  ".repeat(depth),
            item.icon,
            item.label
        ))];

        if item.expanded {
            for child in &item.children {
                children.push(self.render_item(child, depth + 1));
            }
        }

        let mut attrs = vec![("class".to_string(), VAttr::Static("tree-item".to_string()))];

        if let Some(handler) = &item.on_click {
            attrs.push(("on_click".to_string(), VAttr::Event(handler.clone())));
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs,
            children,
        }
    }
}

impl TreeItem {
    pub fn new(label: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: icon.into(),
            children: Vec::new(),
            expanded: false,
            on_click: None,
        }
    }

    pub fn children(mut self, children: Vec<TreeItem>) -> Self {
        self.children = children;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: FnMut() + 'static,
    {
        self.on_click = Some(Rc::new(RefCell::new(handler)));
        self
    }
}

impl ToVNode for TreeView {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
