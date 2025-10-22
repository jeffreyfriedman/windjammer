// windjammer-runtime::ui - Virtual DOM implementation backing std::ui

use std::collections::HashMap;

/// Virtual Node - represents a UI element
#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    Element(VElement),
    Text(VText),
    Component(VComponent),
}

/// Virtual Element - represents an HTML element
#[derive(Debug, Clone, PartialEq)]
pub struct VElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<VNode>,
}

impl VElement {
    pub fn new(tag: &str) -> Self {
        VElement {
            tag: tag.to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    pub fn child(mut self, node: VNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn into_vnode(self) -> VNode {
        VNode::Element(self)
    }
}

/// Virtual Text - represents a text node
#[derive(Debug, Clone, PartialEq)]
pub struct VText {
    pub content: String,
}

impl VText {
    pub fn new(content: &str) -> Self {
        VText {
            content: content.to_string(),
        }
    }

    pub fn into_vnode(self) -> VNode {
        VNode::Text(self)
    }
}

/// Virtual Component - represents a component instance
#[derive(Debug, Clone, PartialEq)]
pub struct VComponent {
    pub name: String,
    pub props: HashMap<String, String>,
}

impl VComponent {
    pub fn new(name: &str) -> Self {
        VComponent {
            name: name.to_string(),
            props: HashMap::new(),
        }
    }

    pub fn prop(mut self, key: &str, value: &str) -> Self {
        self.props.insert(key.to_string(), value.to_string());
        self
    }

    pub fn into_vnode(self) -> VNode {
        VNode::Component(self)
    }
}

// Helper functions for creating elements (matches std::ui API)

pub fn div() -> VElement {
    VElement::new("div")
}

pub fn h1(text: &str) -> VNode {
    VElement::new("h1")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn h2(text: &str) -> VNode {
    VElement::new("h2")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn p(text: &str) -> VNode {
    VElement::new("p")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn button(text: &str) -> VElement {
    VElement::new("button").child(VNode::Text(VText::new(text)))
}

pub fn input() -> VElement {
    VElement::new("input")
}

pub fn text(content: &str) -> VNode {
    VNode::Text(VText::new(content))
}

pub fn ul() -> VElement {
    VElement::new("ul")
}

pub fn li(text: &str) -> VNode {
    VElement::new("li")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

pub fn span(text: &str) -> VNode {
    VElement::new("span")
        .child(VNode::Text(VText::new(text)))
        .into_vnode()
}

/// Render a VNode tree to HTML string
pub fn render_to_string(node: &VNode) -> String {
    match node {
        VNode::Element(el) => {
            let mut html = format!("<{}", el.tag);

            // Add attributes
            for (key, value) in &el.attributes {
                html.push_str(&format!(" {}=\"{}\"", key, value));
            }

            html.push('>');

            // Add children
            for child in &el.children {
                html.push_str(&render_to_string(child));
            }

            html.push_str(&format!("</{}>", el.tag));
            html
        }
        VNode::Text(text) => text.content.clone(),
        VNode::Component(comp) => {
            format!("<component name=\"{}\" />", comp.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velement_creation() {
        let el = VElement::new("div");
        assert_eq!(el.tag, "div");
        assert!(el.children.is_empty());
        assert!(el.attributes.is_empty());
    }

    #[test]
    fn test_velement_with_attributes() {
        let el = VElement::new("div")
            .attr("class", "container")
            .attr("id", "main");

        assert_eq!(el.attributes.get("class"), Some(&"container".to_string()));
        assert_eq!(el.attributes.get("id"), Some(&"main".to_string()));
    }

    #[test]
    fn test_velement_with_children() {
        let el = VElement::new("div").child(VNode::Text(VText::new("Hello")));

        assert_eq!(el.children.len(), 1);
        match &el.children[0] {
            VNode::Text(text) => assert_eq!(text.content, "Hello"),
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_helper_functions() {
        let heading = h1("Title");
        match heading {
            VNode::Element(el) => {
                assert_eq!(el.tag, "h1");
                assert_eq!(el.children.len(), 1);
            }
            _ => panic!("Expected element"),
        }
    }

    #[test]
    fn test_render_to_string() {
        let node = VElement::new("div")
            .attr("class", "container")
            .child(h1("Hello"))
            .into_vnode();

        let html = render_to_string(&node);
        assert!(html.contains("<div"));
        assert!(html.contains("class=\"container\""));
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("</div>"));
    }
}
