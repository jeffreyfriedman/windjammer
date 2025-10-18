//! Web Counter Example - Demonstrates WebRenderer with actual browser DOM
//!
//! To run this example:
//! ```bash
//! wasm-pack build --target web crates/windjammer-ui --example web_counter
//! # Then serve the generated files and open in a browser
//! ```

use windjammer_ui::vdom::{VElement, VNode, VText};

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        // Initialize panic hook for better error messages
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        // Create renderer and mount to DOM
        let mut renderer = WebRenderer::new();
        renderer.init().expect("Failed to initialize renderer");

        // Create a simple counter UI
        let counter_ui = create_counter_ui(0);

        // Render to DOM
        renderer
            .render(&counter_ui)
            .expect("Failed to render counter");

        // Log success
        web_sys::console::log_1(&"Counter rendered successfully!".into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("This example only runs in WASM/browser environment");
        println!(
            "Build with: wasm-pack build --target web crates/windjammer-ui --example web_counter"
        );
    }
}

#[allow(dead_code)]
fn create_counter_ui(count: i64) -> VNode {
    VElement::new("div")
        .attr("class", "counter")
        .attr("style", "padding: 20px; font-family: sans-serif;")
        .child(VNode::Element(
            VElement::new("h1").child(VNode::Text(VText::new(format!("Count: {}", count)))),
        ))
        .child(VNode::Element(
            VElement::new("button")
                .attr("id", "increment")
                .attr(
                    "style",
                    "padding: 10px 20px; font-size: 16px; cursor: pointer;",
                )
                .child(VNode::Text(VText::new("Increment"))),
        ))
        .child(VNode::Element(
            VElement::new("p")
                .attr("style", "margin-top: 20px; color: #666;")
                .child(VNode::Text(VText::new(
                    "✨ Rendered with Windjammer UI WebRenderer",
                ))),
        ))
        .into()
}
