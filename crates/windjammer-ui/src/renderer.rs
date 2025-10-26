//! Cross-platform renderer

use crate::component::Component;

#[cfg(not(feature = "web"))]
use crate::platform::create_platform;

/// Mount a component to the target selector
#[cfg(feature = "web")]
pub fn mount<C: Component>(selector: &str, component: C) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        // Get the window and document
        let window = web_sys::window().ok_or("No window found")?;
        let document = window.document().ok_or("No document found")?;

        // Find the target element
        let target = document
            .query_selector(selector)
            .map_err(|_| format!("Invalid selector: {}", selector))?
            .ok_or(format!("Element not found: {}", selector))?;

        // Render the component to a VNode
        let vnode = component.render();

        // Create a WebRenderer
        let renderer = WebRenderer::new();

        // Create the DOM element from VNode
        let dom_node = renderer.create_element(&vnode)?;

        // Clear the target and append the rendered content
        while let Some(child) = target.first_child() {
            target
                .remove_child(&child)
                .map_err(|_| "Failed to clear target")?;
        }

        target
            .append_child(&dom_node)
            .map_err(|_| "Failed to mount component")?;

        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (selector, component);
        Err("mount() is only available on WASM target".to_string())
    }
}

/// Mount a component to the target selector (non-web platforms)
#[cfg(not(feature = "web"))]
pub fn mount<C: Component>(_selector: &str, _component: C) -> Result<(), String> {
    let mut platform = create_platform();
    platform.init()?;

    // In a full implementation, this would:
    // 1. Render the component to a VNode tree
    // 2. Convert VNode to platform-specific representation
    // 3. Attach to the DOM/native view

    // For now, just return success
    Ok(())
}

/// Renderer trait for different platforms
#[cfg(not(target_arch = "wasm32"))]
pub trait Renderer: Send + Sync {
    /// Initialize the renderer
    fn init(&mut self) -> Result<(), String>;

    /// Render a virtual DOM tree
    fn render(&mut self, vnode: &crate::vdom::VNode) -> Result<(), String>;

    /// Apply patches to the rendered tree
    fn patch(&mut self, patches: &[crate::vdom::Patch]) -> Result<(), String>;
}

/// Renderer trait for WASM (no Send + Sync required)
#[cfg(target_arch = "wasm32")]
pub trait Renderer {
    /// Initialize the renderer
    fn init(&mut self) -> Result<(), String>;

    /// Render a virtual DOM tree
    fn render(&mut self, vnode: &crate::vdom::VNode) -> Result<(), String>;

    /// Apply patches to the rendered tree
    fn patch(&mut self, patches: &[crate::vdom::Patch]) -> Result<(), String>;
}

/// Web renderer (JavaScript/WASM)
#[cfg(feature = "web")]
pub struct WebRenderer {
    #[cfg(target_arch = "wasm32")]
    document: web_sys::Document,
    #[cfg(target_arch = "wasm32")]
    root: Option<web_sys::Element>,
    #[cfg(not(target_arch = "wasm32"))]
    _dummy: (),
}

#[cfg(not(feature = "web"))]
pub struct WebRenderer {
    initialized: bool,
}

#[cfg(feature = "web")]
impl WebRenderer {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");
            Self {
                document,
                root: None,
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self { _dummy: () }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn create_element(&self, vnode: &crate::vdom::VNode) -> Result<web_sys::Node, String> {
        use crate::vdom::VNode;
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        match vnode {
            VNode::Element(element) => {
                let dom_element = self
                    .document
                    .create_element(&element.tag)
                    .map_err(|_| format!("Failed to create element: {}", element.tag))?;

                // Set attributes and event handlers
                for (key, value) in &element.attrs {
                    // Check if this is an event handler (starts with "on")
                    if key.starts_with("on") {
                        let event_type = key.strip_prefix("on").unwrap_or(key);

                        // For now, we'll just log events
                        // In a full implementation, this would dispatch to component methods
                        let callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
                            web_sys::console::log_1(
                                &format!("Event triggered: {}", event_type).into(),
                            );
                            // Prevent default behavior for some events
                            if event_type == "submit" {
                                event.prevent_default();
                            }
                        })
                            as Box<dyn FnMut(_)>);

                        dom_element
                            .add_event_listener_with_callback(
                                event_type,
                                callback.as_ref().unchecked_ref(),
                            )
                            .map_err(|_| format!("Failed to add event listener: {}", key))?;

                        // Leak the closure to keep it alive
                        // In production, we'd store these and clean them up properly
                        callback.forget();
                    } else {
                        // Regular attribute
                        dom_element
                            .set_attribute(key, value)
                            .map_err(|_| format!("Failed to set attribute: {}", key))?;
                    }
                }

                // Append children
                for child in &element.children {
                    let child_node = self.create_element(child)?;
                    dom_element
                        .append_child(&child_node)
                        .map_err(|_| "Failed to append child".to_string())?;
                }

                Ok(dom_element.into())
            }
            VNode::Text(text) => {
                let text_node = self.document.create_text_node(&text.content);
                Ok(text_node.into())
            }
            VNode::Component(_) => {
                // Components should be expanded to elements before rendering
                Err("Cannot render component directly".to_string())
            }
            VNode::Empty => {
                let text_node = self.document.create_text_node("");
                Ok(text_node.into())
            }
        }
    }
}

#[cfg(not(feature = "web"))]
impl WebRenderer {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for WebRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "web")]
impl Renderer for WebRenderer {
    fn init(&mut self) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            // Get or create root element
            let root = self
                .document
                .get_element_by_id("app")
                .or_else(|| {
                    let body = self.document.body()?;
                    let div = self.document.create_element("div").ok()?;
                    div.set_id("app");
                    body.append_child(&div).ok()?;
                    Some(div)
                })
                .ok_or("Failed to create root element")?;

            self.root = Some(root);
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(())
        }
    }

    fn render(&mut self, vnode: &crate::vdom::VNode) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let root = self.root.as_ref().ok_or("Renderer not initialized")?;

            // Clear existing content
            while let Some(child) = root.first_child() {
                root.remove_child(&child)
                    .map_err(|_| "Failed to remove child")?;
            }

            // Render new content
            let node = self.create_element(vnode)?;
            root.append_child(&node)
                .map_err(|_| "Failed to append root node")?;

            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = vnode;
            Ok(())
        }
    }

    fn patch(&mut self, patches: &[crate::vdom::Patch]) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            use crate::vdom::Patch;
            use wasm_bindgen::JsCast;

            let root = self.root.as_ref().ok_or("No root element")?;

            for patch in patches {
                match patch {
                    Patch::Replace { path, node } => {
                        // Find the node at path and replace it
                        let target = self.find_node_at_path(root, path)?;
                        let new_node = self.create_element(node)?;

                        if let Some(parent) = target.parent_node() {
                            parent
                                .replace_child(&new_node, &target)
                                .map_err(|_| "Failed to replace node")?;
                        }
                    }
                    Patch::UpdateText { path, content } => {
                        // Update text node content
                        let target = self.find_node_at_path(root, path)?;
                        if let Some(text_node) = target.dyn_ref::<web_sys::Text>() {
                            text_node.set_data(content);
                        }
                    }
                    Patch::SetAttribute { path, key, value } => {
                        // Set attribute on element
                        let target = self.find_node_at_path(root, path)?;
                        if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                            if value.is_empty() {
                                element
                                    .remove_attribute(key)
                                    .map_err(|_| format!("Failed to remove attribute: {}", key))?;
                            } else {
                                element
                                    .set_attribute(key, value)
                                    .map_err(|_| format!("Failed to set attribute: {}", key))?;
                            }
                        }
                    }
                    Patch::Append { path, node } => {
                        // Append child node
                        let target = self.find_node_at_path(root, path)?;
                        let new_node = self.create_element(node)?;
                        target
                            .append_child(&new_node)
                            .map_err(|_| "Failed to append child")?;
                    }
                    Patch::Remove { path } => {
                        // Remove child node
                        let target = self.find_node_at_path(root, path)?;
                        if let Some(parent) = target.parent_node() {
                            parent
                                .remove_child(&target)
                                .map_err(|_| "Failed to remove child")?;
                        }
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = patches;
            Ok(())
        }
    }
}

// Helper methods for WebRenderer
#[cfg(target_arch = "wasm32")]
impl WebRenderer {
    fn find_node_at_path(
        &self,
        root: &web_sys::Element,
        path: &[usize],
    ) -> Result<web_sys::Node, String> {
        let mut current: web_sys::Node = root.clone().into();

        for &index in path.iter().skip(1) {
            // Skip first index (root)
            // Node has child_nodes() method, but we need to access it correctly
            let children = current.child_nodes();
            current = children
                .get(index as u32)
                .ok_or(format!("Child not found at index {}", index))?;
        }

        Ok(current)
    }
}

#[cfg(not(feature = "web"))]
impl Renderer for WebRenderer {
    fn init(&mut self) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn render(&mut self, _vnode: &crate::vdom::VNode) -> Result<(), String> {
        Ok(())
    }

    fn patch(&mut self, _patches: &[crate::vdom::Patch]) -> Result<(), String> {
        Ok(())
    }
}

/// Desktop renderer (Tauri)
#[cfg(feature = "desktop")]
pub struct DesktopRenderer {
    html_content: String,
    pending_updates: Vec<String>,
}

#[cfg(not(feature = "desktop"))]
pub struct DesktopRenderer {
    initialized: bool,
}

#[cfg(feature = "desktop")]
impl DesktopRenderer {
    pub fn new() -> Self {
        Self {
            html_content: String::new(),
            pending_updates: Vec::new(),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn vnode_to_html(&self, vnode: &crate::vdom::VNode) -> String {
        use crate::vdom::VNode;

        match vnode {
            VNode::Element(element) => {
                let mut html = format!("<{}", element.tag);

                // Add attributes
                for (key, value) in &element.attrs {
                    html.push_str(&format!(" {}=\"{}\"", key, value));
                }

                html.push('>');

                // Add children
                for child in &element.children {
                    html.push_str(&self.vnode_to_html(child));
                }

                html.push_str(&format!("</{}>", element.tag));
                html
            }
            VNode::Text(text) => text.content.clone(),
            VNode::Component(_) => String::new(),
            VNode::Empty => String::new(),
        }
    }

    #[cfg(feature = "desktop")]
    fn send_to_webview(&mut self, html: &str) -> Result<(), String> {
        // In a full Tauri implementation, this would use:
        // tauri::Manager::emit() to send updates to the webview
        // For now, store for testing
        self.html_content = html.to_string();
        Ok(())
    }
}

#[cfg(not(feature = "desktop"))]
impl DesktopRenderer {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for DesktopRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "desktop")]
impl Renderer for DesktopRenderer {
    fn init(&mut self) -> Result<(), String> {
        // Initialize Tauri webview
        self.html_content = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <title>Windjammer App</title>
                <style>
                    body { margin: 0; padding: 0; font-family: system-ui; }
                    #app { width: 100vw; height: 100vh; }
                </style>
            </head>
            <body>
                <div id="app"></div>
            </body>
            </html>
        "#
        .to_string();
        Ok(())
    }

    fn render(&mut self, vnode: &crate::vdom::VNode) -> Result<(), String> {
        let html = self.vnode_to_html(vnode);
        self.send_to_webview(&html)?;
        Ok(())
    }

    fn patch(&mut self, patches: &[crate::vdom::Patch]) -> Result<(), String> {
        use crate::vdom::Patch;

        // Convert patches to JavaScript commands
        for patch in patches {
            let js_command = match patch {
                Patch::Replace { .. } => "document.getElementById('app').innerHTML = ...;",
                Patch::UpdateText { .. } => "element.textContent = ...;",
                Patch::SetAttribute { .. } => "element.setAttribute(...);",
                // RemoveAttribute is not a separate variant, it's handled by SetAttribute with empty value
                Patch::Append { .. } => "element.appendChild(...);",
                Patch::Remove { .. } => "element.removeChild(...);",
            };
            self.pending_updates.push(js_command.to_string());
        }

        Ok(())
    }
}

#[cfg(not(feature = "desktop"))]
impl Renderer for DesktopRenderer {
    fn init(&mut self) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn render(&mut self, _vnode: &crate::vdom::VNode) -> Result<(), String> {
        Ok(())
    }

    fn patch(&mut self, _patches: &[crate::vdom::Patch]) -> Result<(), String> {
        Ok(())
    }
}

/// Mobile renderer (iOS/Android)
#[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
pub struct MobileRenderer {
    view_hierarchy: Vec<NativeView>,
    root_view: Option<usize>,
}

#[cfg(not(any(feature = "mobile-ios", feature = "mobile-android")))]
pub struct MobileRenderer {
    initialized: bool,
}

#[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NativeView {
    id: usize,
    view_type: String,
    properties: std::collections::HashMap<String, String>,
    children: Vec<usize>,
}

#[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
impl MobileRenderer {
    pub fn new() -> Self {
        Self {
            view_hierarchy: Vec::new(),
            root_view: None,
        }
    }

    fn vnode_to_native_view(&mut self, vnode: &crate::vdom::VNode) -> Option<usize> {
        use crate::vdom::VNode;

        match vnode {
            VNode::Element(element) => {
                let view_type = self.map_html_to_native(&element.tag);
                let id = self.view_hierarchy.len();

                let mut properties = std::collections::HashMap::new();
                for (key, value) in &element.attrs {
                    properties.insert(key.clone(), value.clone());
                }

                let mut view = NativeView {
                    id,
                    view_type,
                    properties,
                    children: Vec::new(),
                };

                // Process children
                for child in &element.children {
                    if let Some(child_id) = self.vnode_to_native_view(child) {
                        view.children.push(child_id);
                    }
                }

                self.view_hierarchy.push(view);
                Some(id)
            }
            VNode::Text(text) => {
                let id = self.view_hierarchy.len();
                let mut properties = std::collections::HashMap::new();
                properties.insert("text".to_string(), text.content.clone());

                let view = NativeView {
                    id,
                    view_type: "TextView".to_string(),
                    properties,
                    children: Vec::new(),
                };

                self.view_hierarchy.push(view);
                Some(id)
            }
            VNode::Component(_) => None,
            VNode::Empty => None,
        }
    }

    fn map_html_to_native(&self, tag: &str) -> String {
        // Map HTML tags to native view types
        match tag {
            "div" => "ContainerView",
            "span" => "TextView",
            "button" => "Button",
            "input" => "TextField",
            "img" => "ImageView",
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "HeaderView",
            "p" => "TextView",
            "a" => "LinkView",
            "ul" | "ol" => "ListView",
            "li" => "ListItemView",
            _ => "View",
        }
        .to_string()
    }

    #[cfg(feature = "mobile-ios")]
    fn create_uikit_views(&self) -> Result<(), String> {
        // In a full implementation, would use objc/cocoa to create UIViews
        // For now, just validate the hierarchy
        Ok(())
    }

    #[cfg(feature = "mobile-android")]
    fn create_android_views(&self) -> Result<(), String> {
        // In a full implementation, would use jni to create Android Views
        // For now, just validate the hierarchy
        Ok(())
    }
}

#[cfg(not(any(feature = "mobile-ios", feature = "mobile-android")))]
impl MobileRenderer {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for MobileRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(feature = "mobile-ios", feature = "mobile-android"))]
impl Renderer for MobileRenderer {
    fn init(&mut self) -> Result<(), String> {
        // Initialize native view system
        self.view_hierarchy.clear();
        self.root_view = None;
        Ok(())
    }

    fn render(&mut self, vnode: &crate::vdom::VNode) -> Result<(), String> {
        // Clear existing views
        self.view_hierarchy.clear();

        // Build native view hierarchy
        self.root_view = self.vnode_to_native_view(vnode);

        // Create platform-specific views
        #[cfg(feature = "mobile-ios")]
        self.create_uikit_views()?;

        #[cfg(feature = "mobile-android")]
        self.create_android_views()?;

        Ok(())
    }

    fn patch(&mut self, patches: &[crate::vdom::Patch]) -> Result<(), String> {
        use crate::vdom::Patch;

        // Apply patches to native views
        for patch in patches {
            match patch {
                Patch::Replace { .. } => {
                    // Replace native view
                }
                Patch::UpdateText { .. } => {
                    // Update native text view
                }
                Patch::SetAttribute { .. } => {
                    // Update view property (or remove if value is empty)
                }
                Patch::Append { .. } => {
                    // Add subview
                }
                Patch::Remove { .. } => {
                    // Remove subview
                }
            }
        }

        Ok(())
    }
}

#[cfg(not(any(feature = "mobile-ios", feature = "mobile-android")))]
impl Renderer for MobileRenderer {
    fn init(&mut self) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn render(&mut self, _vnode: &crate::vdom::VNode) -> Result<(), String> {
        Ok(())
    }

    fn patch(&mut self, _patches: &[crate::vdom::Patch]) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_renderer_creation() {
        let mut renderer = WebRenderer::new();
        assert!(renderer.init().is_ok());
    }

    #[test]
    fn test_desktop_renderer_creation() {
        let mut renderer = DesktopRenderer::new();
        assert!(renderer.init().is_ok());
    }

    #[test]
    fn test_mobile_renderer_creation() {
        let mut renderer = MobileRenderer::new();
        assert!(renderer.init().is_ok());
    }

    #[test]
    fn test_web_renderer_render() {
        let mut renderer = WebRenderer::new();
        renderer.init().unwrap();

        use crate::vdom::{VElement, VNode, VText};
        let vnode: VNode = VElement::new("div")
            .child(VNode::Text(VText::new("Hello World")))
            .into();

        assert!(renderer.render(&vnode).is_ok());
    }

    #[test]
    fn test_desktop_renderer_render() {
        let mut renderer = DesktopRenderer::new();
        renderer.init().unwrap();

        use crate::vdom::{VElement, VNode, VText};
        let vnode: VNode = VElement::new("div")
            .child(VNode::Text(VText::new("Hello Desktop")))
            .into();

        assert!(renderer.render(&vnode).is_ok());
    }

    #[test]
    fn test_mobile_renderer_render() {
        let mut renderer = MobileRenderer::new();
        renderer.init().unwrap();

        use crate::vdom::{VElement, VNode, VText};
        let vnode: VNode = VElement::new("div")
            .child(VNode::Text(VText::new("Hello Mobile")))
            .into();

        assert!(renderer.render(&vnode).is_ok());
    }
}
