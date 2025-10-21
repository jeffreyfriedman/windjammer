//! Simple UI renderer for basic elements
//! This provides a minimal but functional UI rendering system

use crate::vdom::{VElement, VNode};

/// Simple UI renderer that converts VNodes to console output
/// This is a minimal implementation for testing and demos
pub struct SimpleRenderer {
    output: Vec<String>,
}

impl SimpleRenderer {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }

    /// Render a VNode tree to console output
    pub fn render(&mut self, vnode: &VNode) {
        self.output.clear();
        self.render_node(vnode, 0);
    }

    fn render_node(&mut self, vnode: &VNode, depth: usize) {
        let indent = "  ".repeat(depth);

        match vnode {
            VNode::Element(element) => {
                self.render_element(element, depth);
            }
            VNode::Text(text) => {
                self.output
                    .push(format!("{}Text: {}", indent, text.content));
            }
            VNode::Component(component) => {
                self.output
                    .push(format!("{}Component: {}", indent, component.name));
            }
            VNode::Empty => {
                // Empty nodes don't render anything
            }
        }
    }

    fn render_element(&mut self, element: &VElement, depth: usize) {
        let indent = "  ".repeat(depth);

        // Render opening tag
        let mut tag_str = format!("{}<{}", indent, element.tag);

        // Add attributes
        for (key, value) in &element.attrs {
            tag_str.push_str(&format!(" {}=\"{}\"", key, value));
        }
        tag_str.push('>');

        self.output.push(tag_str);

        // Render children
        for child in &element.children {
            self.render_node(child, depth + 1);
        }

        // Render closing tag
        self.output.push(format!("{}</{}>", indent, element.tag));
    }

    /// Get the rendered output as a string
    pub fn get_output(&self) -> String {
        self.output.join("\n")
    }

    /// Print the rendered output
    pub fn print(&self) {
        println!("{}", self.get_output());
    }
}

impl Default for SimpleRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdom::{VComponent, VElement, VText};
    use std::collections::HashMap;

    #[test]
    fn test_render_text() {
        let mut renderer = SimpleRenderer::new();
        let vnode = VNode::Text(VText::new("Hello, World!"));

        renderer.render(&vnode);
        let output = renderer.get_output();

        assert!(output.contains("Hello, World!"));
    }

    #[test]
    fn test_render_element() {
        let mut renderer = SimpleRenderer::new();
        let element = VElement {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![VNode::Text(VText::new("Content"))],
        };

        renderer.render(&VNode::Element(element));
        let output = renderer.get_output();

        assert!(output.contains("<div>"));
        assert!(output.contains("Content"));
        assert!(output.contains("</div>"));
    }

    #[test]
    fn test_render_component() {
        let mut renderer = SimpleRenderer::new();
        let component = VComponent {
            name: "Counter".to_string(),
            props: HashMap::new(),
        };

        renderer.render(&VNode::Component(component));
        let output = renderer.get_output();

        assert!(output.contains("Component: Counter"));
    }
}
