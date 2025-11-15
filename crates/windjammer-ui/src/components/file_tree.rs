//! File Tree component for file browser

use crate::prelude::*;
use crate::simple_vnode::{VAttr, VNode};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub is_expanded: bool,
    pub children: Vec<FileNode>,
}

impl FileNode {
    pub fn new(name: impl Into<String>, path: impl Into<String>, is_directory: bool) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            is_directory,
            is_expanded: false,
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<FileNode>) -> Self {
        self.children = children;
        self
    }
}

pub struct FileTree {
    pub root: FileNode,
    pub selected_path: Signal<Option<String>>,
    pub on_select: Option<Rc<RefCell<dyn FnMut(String)>>>,
}

impl FileTree {
    pub fn new(root: FileNode) -> Self {
        Self {
            root,
            selected_path: Signal::new(None),
            on_select: None,
        }
    }

    pub fn on_select<F: FnMut(String) + 'static>(mut self, handler: F) -> Self {
        self.on_select = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("wj-file-tree".to_string()),
            )],
            children: vec![self.render_node(&self.root, 0)],
        }
    }

    fn render_node(&self, node: &FileNode, depth: usize) -> VNode {
        let mut children = vec![];

        // Render the node itself
        let mut node_classes = vec!["wj-file-tree-node".to_string()];

        if let Some(ref selected) = self.selected_path.get() {
            if selected == &node.path {
                node_classes.push("wj-file-tree-node-selected".to_string());
            }
        }

        let icon = if node.is_directory {
            if node.is_expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            "📄"
        };

        let node_element = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                ("class".to_string(), VAttr::Static(node_classes.join(" "))),
                (
                    "style".to_string(),
                    VAttr::Static(format!("padding-left: {}px;", depth * 16)),
                ),
                ("data-path".to_string(), VAttr::Static(node.path.clone())),
            ],
            children: vec![VNode::Text(format!("{} {}", icon, node.name))],
        };

        children.push(node_element);

        // Render children if expanded
        if node.is_expanded {
            for child in &node.children {
                children.push(self.render_node(child, depth + 1));
            }
        }

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![],
            children,
        }
    }
}
