//! App runtime for mounting and running Windjammer UI applications
//!
//! This module provides the `App` struct and runtime system for:
//! - Mounting UI components to the DOM (WASM)
//! - Running the reactive system
//! - Handling the render loop
//! - Managing application lifecycle

use crate::simple_vnode::VNode;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Document, Element};

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Global app state for re-rendering
    static APP_STATE: RefCell<Option<AppState>> = RefCell::new(None);
}

#[cfg(target_arch = "wasm32")]
struct AppState {
    render_fn: Rc<dyn Fn() -> VNode>,
    root_element: Element,
    document: web_sys::Document,
}

/// Application runtime
pub struct App {
    /// Application title
    pub title: String,
    /// Root UI component (static VNode for now)
    pub root: VNode,
}

impl App {
    /// Create a new application with a static VNode
    pub fn new(title: impl Into<String>, root: VNode) -> Self {
        Self {
            title: title.into(),
            root,
        }
    }

    /// Create a new application with a render function (for reactivity)
    #[cfg(target_arch = "wasm32")]
    pub fn new_reactive(title: impl Into<String>, render_fn: impl Fn() -> VNode + 'static) -> Self {
        let render_fn = Rc::new(render_fn);
        let initial_vnode = render_fn();

        Self {
            title: title.into(),
            root: initial_vnode,
        }
    }

    /// Run the application (WASM only)
    #[cfg(target_arch = "wasm32")]
    pub fn run(self) {
        match self.run_internal() {
            Ok(_) => {}
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to mount app: {:?}", e).into());
            }
        }
    }

    /// Internal run method that returns Result
    #[cfg(target_arch = "wasm32")]
    fn run_internal(self) -> Result<(), JsValue> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        web_sys::console::log_1(&"🔧 Starting App::run_internal".into());

        // Get the window and document
        let window = window().ok_or("No window found")?;
        let document = window.document().ok_or("No document found")?;

        web_sys::console::log_1(&"✓ Got window and document".into());

        // Set the document title
        document.set_title(&self.title);

        // Get or create the root element
        let root_element = document
            .get_element_by_id("app")
            .or_else(|| document.body().map(|b| b.into()))
            .ok_or("No root element found")?;

        web_sys::console::log_1(&"✓ Got root element".into());

        // Clear existing content
        root_element.set_inner_html("");

        web_sys::console::log_1(&"✓ Cleared root element".into());

        // Render the root VNode
        web_sys::console::log_1(&"🎨 Rendering VNode...".into());
        let rendered = self.root.render(&document)?;

        web_sys::console::log_1(&"✓ VNode rendered".into());

        root_element.append_child(&rendered)?;

        web_sys::console::log_1(&"✅ UI mounted successfully!".into());

        Ok(())
    }

    /// Run the application (non-WASM - just prints info)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(self) {
        println!("🎮 {} - Windjammer App", self.title);
        println!("Note: This is a UI application. To see the UI, compile to WASM.");
        println!("Root component created successfully.");
    }
}

/// Mount a component to the DOM (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn mount(root: VNode) {
    App::new("Windjammer App", root).run()
}

/// Mount a component with a custom title (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn mount_with_title(title: impl Into<String>, root: VNode) {
    App::new(title, root).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new("Test App", VNode::Text("Hello".to_string()));
        assert_eq!(app.title, "Test App");
    }
}
