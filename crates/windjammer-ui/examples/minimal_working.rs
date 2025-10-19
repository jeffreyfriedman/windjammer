//! Minimal Working Counter - Pure Rust, no Windjammer transpilation needed
//! This demonstrates that the UI framework actually works
//!
//! To build and run:
//! ```bash
//! cd crates/windjammer-ui
//! wasm-pack build --target web --dev
//! python3 -m http.server 8000
//! # Open http://localhost:8000/examples/minimal_working.html
//! ```

fn main() {
    #[cfg(target_arch = "wasm32")]
    run_wasm();

    #[cfg(not(target_arch = "wasm32"))]
    panic!("This example is WASM-only. Build with: wasm-pack build --target web --dev");
}

#[cfg(target_arch = "wasm32")]
fn run_wasm() {
    use windjammer_ui::vdom::{VElement, VNode, VText};
    // Set panic hook for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    // Log that we're starting
    web_sys::console::log_1(&"Windjammer UI starting...".into());

    // Create a simple counter UI
    let counter_ui = VElement::new("div")
        .attr("class", "counter-app")
        .attr(
            "style",
            "padding: 40px; font-family: Arial, sans-serif; text-align: center;",
        )
        .child(VNode::Element(
            VElement::new("h1")
                .attr("style", "color: #333; margin-bottom: 20px;")
                .child(VNode::Text(VText::new("Windjammer UI Counter"))),
        ))
        .child(VNode::Element(
            VElement::new("div")
                .attr("id", "count-display")
                .attr(
                    "style",
                    "font-size: 48px; font-weight: bold; color: #4CAF50; margin: 20px 0;",
                )
                .child(VNode::Text(VText::new("0"))),
        ))
        .child(VNode::Element(
            VElement::new("button")
                .attr("id", "increment-btn")
                .attr("onclick", "increment")
                .attr(
                    "style",
                    "padding: 15px 30px; font-size: 18px; background: #4CAF50; color: white; border: none; border-radius: 5px; cursor: pointer; margin: 10px;",
                )
                .child(VNode::Text(VText::new("Increment"))),
        ))
        .child(VNode::Element(
            VElement::new("button")
                .attr("id", "decrement-btn")
                .attr("onclick", "decrement")
                .attr(
                    "style",
                    "padding: 15px 30px; font-size: 18px; background: #f44336; color: white; border: none; border-radius: 5px; cursor: pointer; margin: 10px;",
                )
                .child(VNode::Text(VText::new("Decrement"))),
        ))
        .child(VNode::Element(
            VElement::new("p")
                .attr("style", "margin-top: 30px; color: #666; font-size: 14px;")
                .child(VNode::Text(VText::new(
                    "✨ Powered by Windjammer UI Framework",
                ))),
        ))
        .into();

    // Mount to DOM
    match mount("#app", counter_ui) {
        Ok(_) => {
            web_sys::console::log_1(&"✅ Counter mounted successfully!".into());
        }
        Err(e) => {
            web_sys::console::error_1(&format!("❌ Failed to mount: {}", e).into());
        }
    }
}

// Helper function that takes a VNode instead of Component
#[cfg(target_arch = "wasm32")]
fn mount(selector: &str, vnode: VNode) -> Result<(), String> {
    use windjammer_ui::renderer::WebRenderer;

    // Get the window and document
    let window = web_sys::window().ok_or("No window found")?;
    let document = window.document().ok_or("No document found")?;

    // Find the target element
    let target = document
        .query_selector(selector)
        .map_err(|_| format!("Invalid selector: {}", selector))?
        .ok_or(format!("Element not found: {}", selector))?;

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
