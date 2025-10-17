//! Virtual DOM implementation

use std::collections::HashMap;

/// A virtual DOM node
#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    /// An element node
    Element(VElement),
    /// A text node
    Text(VText),
    /// A component node
    Component(VComponent),
    /// An empty node
    Empty,
}

/// A virtual element
#[derive(Debug, Clone, PartialEq)]
pub struct VElement {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<VNode>,
}

impl VElement {
    /// Create a new virtual element
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            attrs: HashMap::new(),
            children: Vec::new(),
        }
    }

    /// Add an attribute
    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }

    /// Add children
    pub fn children(mut self, children: Vec<VNode>) -> Self {
        self.children = children;
        self
    }

    /// Add a single child
    pub fn child(mut self, child: VNode) -> Self {
        self.children.push(child);
        self
    }
}

/// A virtual text node
#[derive(Debug, Clone, PartialEq)]
pub struct VText {
    pub content: String,
}

impl VText {
    /// Create a new text node
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl From<VElement> for VNode {
    fn from(element: VElement) -> Self {
        VNode::Element(element)
    }
}

impl From<VText> for VNode {
    fn from(text: VText) -> Self {
        VNode::Text(text)
    }
}

impl From<String> for VNode {
    fn from(s: String) -> Self {
        VNode::Text(VText::new(s))
    }
}

impl From<&str> for VNode {
    fn from(s: &str) -> Self {
        VNode::Text(VText::new(s))
    }
}

/// A virtual component node
#[derive(Debug, Clone, PartialEq)]
pub struct VComponent {
    pub name: String,
    pub props: HashMap<String, String>,
}

/// Diff two virtual DOM trees and produce a list of patches
pub fn diff(old: &VNode, new: &VNode) -> Vec<Patch> {
    let mut patches = Vec::new();
    diff_recursive(old, new, &mut patches, vec![0]);
    patches
}

fn diff_recursive(old: &VNode, new: &VNode, patches: &mut Vec<Patch>, path: Vec<usize>) {
    match (old, new) {
        (VNode::Text(old_text), VNode::Text(new_text)) => {
            if old_text.content != new_text.content {
                patches.push(Patch::UpdateText {
                    path: path.clone(),
                    content: new_text.content.clone(),
                });
            }
        }
        (VNode::Element(old_el), VNode::Element(new_el)) => {
            if old_el.tag != new_el.tag {
                patches.push(Patch::Replace {
                    path: path.clone(),
                    node: VNode::Element(new_el.clone()),
                });
                return;
            }

            // Diff attributes
            for (key, new_value) in &new_el.attrs {
                if old_el.attrs.get(key) != Some(new_value) {
                    patches.push(Patch::SetAttribute {
                        path: path.clone(),
                        key: key.clone(),
                        value: new_value.clone(),
                    });
                }
            }

            // Diff children
            let max_len = old_el.children.len().max(new_el.children.len());
            for i in 0..max_len {
                let mut child_path = path.clone();
                child_path.push(i);

                match (old_el.children.get(i), new_el.children.get(i)) {
                    (Some(old_child), Some(new_child)) => {
                        diff_recursive(old_child, new_child, patches, child_path);
                    }
                    (None, Some(new_child)) => {
                        patches.push(Patch::Append {
                            path: path.clone(),
                            node: new_child.clone(),
                        });
                    }
                    (Some(_), None) => {
                        patches.push(Patch::Remove { path: child_path });
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        _ => {
            // Different node types, replace
            patches.push(Patch::Replace {
                path,
                node: new.clone(),
            });
        }
    }
}

/// A patch to apply to the DOM
#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    /// Replace a node
    Replace { path: Vec<usize>, node: VNode },
    /// Update text content
    UpdateText { path: Vec<usize>, content: String },
    /// Set an attribute
    SetAttribute {
        path: Vec<usize>,
        key: String,
        value: String,
    },
    /// Append a child
    Append { path: Vec<usize>, node: VNode },
    /// Remove a node
    Remove { path: Vec<usize> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velement_creation() {
        let el = VElement::new("div")
            .attr("class", "container")
            .child(VNode::Text(VText::new("Hello")));

        assert_eq!(el.tag, "div");
        assert_eq!(el.attrs.get("class"), Some(&"container".to_string()));
        assert_eq!(el.children.len(), 1);
    }

    #[test]
    fn test_vtext_creation() {
        let text = VText::new("Hello, World!");
        assert_eq!(text.content, "Hello, World!");
    }

    #[test]
    fn test_diff_text_update() {
        let old = VNode::Text(VText::new("old"));
        let new = VNode::Text(VText::new("new"));

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::UpdateText { .. }));
    }

    #[test]
    fn test_diff_no_change() {
        let node = VNode::Text(VText::new("same"));
        let patches = diff(&node, &node);
        assert_eq!(patches.len(), 0);
    }
}
