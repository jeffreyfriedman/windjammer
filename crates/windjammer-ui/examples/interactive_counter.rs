//! Interactive Counter - Fully Functional Example
//!
//! This demonstrates a working counter with actual state management and event handling.
//!
//! To build and run:
//! ```bash
//! cd crates/windjammer-ui
//! wasm-pack build --target web --dev --example interactive_counter
//! python3 -m http.server 8000
//! # Open http://localhost:8000/examples/interactive_counter.html
//! ```

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use windjammer_ui::reactivity::Signal;
#[cfg(target_arch = "wasm32")]
use windjammer_ui::vdom::{VElement, VNode, VText};

/// Counter component with reactive state
#[cfg(target_arch = "wasm32")]
struct Counter {
    count: Signal<i32>,
}

#[cfg(target_arch = "wasm32")]
impl Counter {
    fn new() -> Self {
        Self {
            count: Signal::new(0),
        }
    }

    fn render(&self) -> VNode {
        let current_count = self.count.get();

        VElement::new("div")
            .attr("class", "counter-app")
            .attr(
                "style",
                "padding: 40px; font-family: Arial, sans-serif; text-align: center; background: white; border-radius: 15px; box-shadow: 0 10px 40px rgba(0,0,0,0.2);",
            )
            .child(VNode::Element(
                VElement::new("h1")
                    .attr("style", "color: #333; margin-bottom: 20px;")
                    .child(VNode::Text(VText::new("🎉 Windjammer Counter"))),
            ))
            .child(VNode::Element(
                VElement::new("div")
                    .attr("id", "count-display")
                    .attr(
                        "style",
                        "font-size: 72px; font-weight: bold; color: #4CAF50; margin: 30px 0; font-family: 'Courier New', monospace;",
                    )
                    .child(VNode::Text(VText::new(format!("{}", current_count)))),
            ))
            .child(VNode::Element(
                VElement::new("div")
                    .attr("class", "button-group")
                    .attr("style", "display: flex; gap: 15px; justify-content: center; margin: 30px 0;")
                    .child(VNode::Element(
                        VElement::new("button")
                            .attr("id", "decrement-btn")
                            .attr("class", "counter-button")
                            .attr(
                                "style",
                                "padding: 15px 30px; font-size: 24px; background: #f44336; color: white; border: none; border-radius: 8px; cursor: pointer; transition: all 0.2s; font-weight: bold;",
                            )
                            .child(VNode::Text(VText::new("−"))),
                    ))
                    .child(VNode::Element(
                        VElement::new("button")
                            .attr("id", "reset-btn")
                            .attr("class", "counter-button")
                            .attr(
                                "style",
                                "padding: 15px 30px; font-size: 18px; background: #9E9E9E; color: white; border: none; border-radius: 8px; cursor: pointer; transition: all 0.2s;",
                            )
                            .child(VNode::Text(VText::new("Reset"))),
                    ))
                    .child(VNode::Element(
                        VElement::new("button")
                            .attr("id", "increment-btn")
                            .attr("class", "counter-button")
                            .attr(
                                "style",
                                "padding: 15px 30px; font-size: 24px; background: #4CAF50; color: white; border: none; border-radius: 8px; cursor: pointer; transition: all 0.2s; font-weight: bold;",
                            )
                            .child(VNode::Text(VText::new("+"))),
                    )),
            ))
            .child(VNode::Element(
                VElement::new("p")
                    .attr("style", "margin-top: 30px; color: #666; font-size: 14px;")
                    .child(VNode::Text(VText::new(
                        "✨ Built with Windjammer UI Framework • Reactive State Management",
                    ))),
            ))
            .into()
    }

    fn increment(&self) {
        self.count.update(|c| *c += 1);
    }

    fn decrement(&self) {
        self.count.update(|c| *c -= 1);
    }

    fn reset(&self) {
        self.count.set(0);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        panic!("This example is WASM-only. Build with: wasm-pack build --target web --dev");
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Set panic hook for better error messages
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        web_sys::console::log_1(&"🚀 Initializing Windjammer Counter...".into());

        // Create counter component
        let counter = Rc::new(RefCell::new(Counter::new()));

        // Get the window and document
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");

        // Find the app element
        let app_element = document
            .query_selector("#app")
            .expect("query failed")
            .expect("#app not found");

        // Initial render
        render_counter(&counter, &app_element);

        // Wire up event handlers
        {
            let counter_clone = counter.clone();
            let app_clone = app_element.clone();
            let increment_btn = document
                .get_element_by_id("increment-btn")
                .expect("increment button not found");

            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                web_sys::console::log_1(&"➕ Increment clicked".into());
                counter_clone.borrow().increment();
                render_counter(&counter_clone, &app_clone);
            }) as Box<dyn FnMut(_)>);

            increment_btn
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .expect("failed to add event listener");
            closure.forget(); // Keep closure alive
        }

        {
            let counter_clone = counter.clone();
            let app_clone = app_element.clone();
            let decrement_btn = document
                .get_element_by_id("decrement-btn")
                .expect("decrement button not found");

            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                web_sys::console::log_1(&"➖ Decrement clicked".into());
                counter_clone.borrow().decrement();
                render_counter(&counter_clone, &app_clone);
            }) as Box<dyn FnMut(_)>);

            decrement_btn
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .expect("failed to add event listener");
            closure.forget();
        }

        {
            let counter_clone = counter.clone();
            let app_clone = app_element;
            let reset_btn = document
                .get_element_by_id("reset-btn")
                .expect("reset button not found");

            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                web_sys::console::log_1(&"🔄 Reset clicked".into());
                counter_clone.borrow().reset();
                render_counter(&counter_clone, &app_clone);
            }) as Box<dyn FnMut(_)>);

            reset_btn
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .expect("failed to add event listener");
            closure.forget();
        }

        web_sys::console::log_1(&"✅ Counter initialized and ready!".into());
    } // End WASM block
}

#[cfg(target_arch = "wasm32")]
fn render_counter(counter: &Rc<RefCell<Counter>>, target: &web_sys::Element) {
    use windjammer_ui::renderer::WebRenderer;

    // Render the counter
    let vnode = counter.borrow().render();

    // Create DOM from VNode
    let renderer = WebRenderer::new();
    let dom_node = renderer
        .create_element(&vnode)
        .expect("failed to create element");

    // Clear target and append
    while let Some(child) = target.first_child() {
        target.remove_child(&child).expect("failed to remove child");
    }

    target
        .append_child(&dom_node)
        .expect("failed to append child");
}
