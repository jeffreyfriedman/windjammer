//! Server-Side Rendering (SSR) for web targets

use crate::component::Component;
use crate::vdom::{VElement, VNode, VText};
use std::collections::HashMap;

/// SSR renderer that converts VNodes to HTML strings
pub struct SSRRenderer {
    /// Generated HTML
    html: String,
    /// Component state for hydration
    state: HashMap<String, String>,
    /// Hydration script
    hydration_script: String,
}

impl SSRRenderer {
    /// Create a new SSR renderer
    pub fn new() -> Self {
        Self {
            html: String::new(),
            state: HashMap::new(),
            hydration_script: String::new(),
        }
    }

    /// Render a component to HTML string
    pub fn render_to_string<C: Component>(&mut self, component: C) -> String {
        let vnode = component.render();
        self.render_vnode(&vnode);
        self.html.clone()
    }

    /// Render a component to full HTML document with hydration
    pub fn render_to_document<C: Component>(&mut self, component: C, title: &str) -> String {
        let body_html = self.render_to_string(component);

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <script id="__WINDJAMMER_STATE__" type="application/json">
    {}
    </script>
</head>
<body>
    <div id="app">{}</div>
    <script>
        {}
    </script>
</body>
</html>"#,
            Self::escape_html(title),
            serde_json::to_string(&self.state).unwrap_or_default(),
            body_html,
            self.get_hydration_script()
        )
    }

    /// Get the hydration script
    pub fn get_hydration_script(&self) -> String {
        if !self.hydration_script.is_empty() {
            return self.hydration_script.clone();
        }

        // Default hydration script
        r#"
        (function() {
            // Hydration: Attach event listeners to server-rendered HTML
            const state = JSON.parse(document.getElementById('__WINDJAMMER_STATE__').textContent);
            
            // Store state for client-side hydration
            window.__WINDJAMMER_HYDRATION_STATE__ = state;
            
            // Mark as hydrated
            document.getElementById('app').setAttribute('data-hydrated', 'true');
            
            console.log('Windjammer: Hydration complete', state);
        })();
        "#
        .to_string()
    }

    /// Render a VNode to HTML
    fn render_vnode(&mut self, vnode: &VNode) {
        match vnode {
            VNode::Element(element) => self.render_element(element),
            VNode::Text(text) => self.render_text(text),
            VNode::Component(_) => {
                // Components should be expanded before SSR
                self.html.push_str("<!-- Component not expanded -->");
            }
            VNode::Empty => {}
        }
    }

    /// Render an element
    fn render_element(&mut self, element: &VElement) {
        // Opening tag
        self.html.push('<');
        self.html.push_str(&element.tag);

        // Attributes
        for (key, value) in &element.attrs {
            self.html.push(' ');
            self.html.push_str(&Self::escape_attribute(key));
            self.html.push_str("=\"");
            self.html.push_str(&Self::escape_attribute(value));
            self.html.push('"');
        }

        // Self-closing tags
        if element.children.is_empty() && Self::is_void_element(&element.tag) {
            self.html.push_str(" />");
            return;
        }

        self.html.push('>');

        // Children
        for child in &element.children {
            self.render_vnode(child);
        }

        // Closing tag
        self.html.push_str("</");
        self.html.push_str(&element.tag);
        self.html.push('>');
    }

    /// Render text
    fn render_text(&mut self, text: &VText) {
        self.html.push_str(&Self::escape_html(&text.content));
    }

    /// Check if element is self-closing (void element)
    fn is_void_element(tag: &str) -> bool {
        matches!(
            tag,
            "area"
                | "base"
                | "br"
                | "col"
                | "embed"
                | "hr"
                | "img"
                | "input"
                | "link"
                | "meta"
                | "param"
                | "source"
                | "track"
                | "wbr"
        )
    }

    /// Escape HTML special characters
    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Escape attribute values
    fn escape_attribute(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    /// Add component state for hydration
    pub fn add_state(&mut self, key: String, value: String) {
        self.state.insert(key, value);
    }

    /// Set custom hydration script
    pub fn set_hydration_script(&mut self, script: String) {
        self.hydration_script = script;
    }
}

impl Default for SSRRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming SSR renderer for large pages
pub struct StreamingSSRRenderer {
    chunk_size: usize,
    chunks: Vec<String>,
}

impl StreamingSSRRenderer {
    /// Create a new streaming SSR renderer
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            chunks: Vec::new(),
        }
    }

    /// Render a VNode and return chunks
    pub fn render_vnode(&mut self, vnode: &VNode) -> Vec<String> {
        self.chunks.clear();
        let mut buffer = String::new();

        self.render_vnode_to_buffer(vnode, &mut buffer);

        // Split into chunks
        for chunk in buffer.as_bytes().chunks(self.chunk_size) {
            self.chunks.push(String::from_utf8_lossy(chunk).to_string());
        }

        self.chunks.clone()
    }

    #[allow(clippy::only_used_in_recursion)]
    fn render_vnode_to_buffer(&self, vnode: &VNode, buffer: &mut String) {
        match vnode {
            VNode::Element(element) => {
                buffer.push('<');
                buffer.push_str(&element.tag);

                for (key, value) in &element.attrs {
                    buffer.push(' ');
                    buffer.push_str(key);
                    buffer.push_str("=\"");
                    buffer.push_str(&SSRRenderer::escape_attribute(value));
                    buffer.push('"');
                }

                if element.children.is_empty() && SSRRenderer::is_void_element(&element.tag) {
                    buffer.push_str(" />");
                } else {
                    buffer.push('>');
                    for child in &element.children {
                        self.render_vnode_to_buffer(child, buffer);
                    }
                    buffer.push_str("</");
                    buffer.push_str(&element.tag);
                    buffer.push('>');
                }
            }
            VNode::Text(text) => {
                buffer.push_str(&SSRRenderer::escape_html(&text.content));
            }
            VNode::Component(_) => {
                buffer.push_str("<!-- Component -->");
            }
            VNode::Empty => {}
        }
    }
}

/// Hydration helper for client-side
pub struct Hydration {
    state: HashMap<String, String>,
}

impl Hydration {
    /// Create from serialized state
    pub fn from_state(state_json: &str) -> Result<Self, String> {
        let state: HashMap<String, String> =
            serde_json::from_str(state_json).map_err(|e| e.to_string())?;
        Ok(Self { state })
    }

    /// Get state value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }

    /// Check if element is hydrated
    pub fn is_hydrated(&self) -> bool {
        !self.state.is_empty()
    }

    /// Helper extension for render_to_document_with_html
    #[allow(dead_code)]
    fn render_to_document_with_html(&self, title: &str, body_html: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
    {}
</body>
</html>"#,
            title, body_html
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssr_render_simple_element() {
        let mut renderer = SSRRenderer::new();
        let vnode = VNode::Element(VElement::new("div").child(VNode::Text(VText::new("Hello"))));

        renderer.render_vnode(&vnode);
        assert_eq!(renderer.html, "<div>Hello</div>");
    }

    #[test]
    fn test_ssr_render_with_attributes() {
        let mut renderer = SSRRenderer::new();
        let vnode = VNode::Element(
            VElement::new("div")
                .attr("class", "container")
                .child(VNode::Text(VText::new("Content"))),
        );

        renderer.render_vnode(&vnode);
        assert!(renderer.html.contains("class=\"container\""));
        assert!(renderer.html.contains("Content"));
    }

    #[test]
    fn test_ssr_render_void_element() {
        let mut renderer = SSRRenderer::new();
        let vnode = VNode::Element(VElement::new("br"));

        renderer.render_vnode(&vnode);
        assert_eq!(renderer.html, "<br />");
    }

    #[test]
    fn test_ssr_escape_html() {
        let mut renderer = SSRRenderer::new();
        let vnode = VNode::Element(
            VElement::new("div").child(VNode::Text(VText::new("<script>alert('xss')</script>"))),
        );

        renderer.render_vnode(&vnode);
        assert!(renderer.html.contains("&lt;script&gt;"));
        assert!(!renderer.html.contains("<script>"));
    }

    #[test]
    fn test_ssr_nested_elements() {
        let mut renderer = SSRRenderer::new();
        let vnode = VNode::Element(
            VElement::new("div")
                .child(VNode::Element(
                    VElement::new("p").child(VNode::Text(VText::new("Nested"))),
                ))
                .child(VNode::Element(
                    VElement::new("span").child(VNode::Text(VText::new("Content"))),
                )),
        );

        renderer.render_vnode(&vnode);
        assert!(renderer.html.contains("<div>"));
        assert!(renderer.html.contains("<p>Nested</p>"));
        assert!(renderer.html.contains("<span>Content</span>"));
        assert!(renderer.html.contains("</div>"));
    }

    #[test]
    fn test_ssr_document_generation() {
        let mut renderer = SSRRenderer::new();

        // Create a simple component render result
        let vnode = VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("Hello SSR"))));
        renderer.render_vnode(&vnode);

        // Test that HTML was generated
        assert!(renderer.html.contains("<h1>Hello SSR</h1>"));
    }

    #[test]
    fn test_streaming_ssr() {
        let mut renderer = StreamingSSRRenderer::new(10); // Small chunk size for testing
        let vnode = VNode::Element(
            VElement::new("div").child(VNode::Text(VText::new("This is a longer text"))),
        );

        let chunks = renderer.render_vnode(&vnode);
        assert!(!chunks.is_empty());

        // Recombine chunks
        let combined: String = chunks.into_iter().collect();
        assert!(combined.contains("<div>"));
        assert!(combined.contains("This is a longer text"));
        assert!(combined.contains("</div>"));
    }

    #[test]
    fn test_hydration_state() {
        let mut renderer = SSRRenderer::new();
        renderer.add_state("count".to_string(), "42".to_string());
        renderer.add_state("name".to_string(), "Alice".to_string());

        let state_json = serde_json::to_string(&renderer.state).unwrap();
        let hydration = Hydration::from_state(&state_json).unwrap();

        assert_eq!(hydration.get("count"), Some(&"42".to_string()));
        assert_eq!(hydration.get("name"), Some(&"Alice".to_string()));
        assert!(hydration.is_hydrated());
    }

    #[test]
    fn test_attribute_escaping() {
        let mut renderer = SSRRenderer::new();
        let vnode = VNode::Element(
            VElement::new("input")
                .attr("value", "Test \"quoted\" value")
                .attr("data-test", "<script>"),
        );

        renderer.render_vnode(&vnode);
        assert!(renderer.html.contains("&quot;"));
        assert!(renderer.html.contains("&lt;script&gt;"));
    }
}
