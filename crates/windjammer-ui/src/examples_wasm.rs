//! WASM examples entry point
//!
//! This module exports example functions that can be called from JavaScript

use crate::reactivity::Signal;
use crate::renderer::WebRenderer;
use crate::vdom::{VElement, VNode, VText};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Counter component with reactive state
struct Counter {
    count: Signal<i32>,
}

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

/// Helper function to render counter to DOM
fn render_counter(counter: &Rc<RefCell<Counter>>, target: &web_sys::Element) {
    let vnode = counter.borrow().render();
    let renderer = WebRenderer::new();

    match renderer.create_element(&vnode) {
        Ok(dom_node) => {
            // Clear target
            while let Some(child) = target.first_child() {
                let _ = target.remove_child(&child);
            }
            // Append new content
            let _ = target.append_child(&dom_node);
        }
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to render: {}", e).into());
        }
    }
}

/// Initialize and run the interactive counter example
#[wasm_bindgen]
pub fn run_interactive_counter() {
    use wasm_bindgen::JsCast;

    // Set panic hook for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    web_sys::console::log_1(&"🚀 Initializing Windjammer Counter...".into());

    // Get window and document
    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            web_sys::console::error_1(&"No window found".into());
            return;
        }
    };

    let document = match window.document() {
        Some(d) => d,
        None => {
            web_sys::console::error_1(&"No document found".into());
            return;
        }
    };

    // Find the app element
    let app_element = match document.query_selector("#app") {
        Ok(Some(elem)) => elem,
        _ => {
            web_sys::console::error_1(&"#app element not found".into());
            return;
        }
    };

    // Create counter component
    let counter = Rc::new(RefCell::new(Counter::new()));

    // Initial render
    render_counter(&counter, &app_element);

    // Wire up event handlers
    {
        let counter_clone = counter.clone();
        let app_clone = app_element.clone();

        if let Some(increment_btn) = document.get_element_by_id("increment-btn") {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                web_sys::console::log_1(&"➕ Increment clicked".into());
                counter_clone.borrow().increment();
                render_counter(&counter_clone, &app_clone);
            }) as Box<dyn FnMut(_)>);

            let _ = increment_btn
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
            closure.forget(); // Keep closure alive
        }
    }

    {
        let counter_clone = counter.clone();
        let app_clone = app_element.clone();

        if let Some(decrement_btn) = document.get_element_by_id("decrement-btn") {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                web_sys::console::log_1(&"➖ Decrement clicked".into());
                counter_clone.borrow().decrement();
                render_counter(&counter_clone, &app_clone);
            }) as Box<dyn FnMut(_)>);

            let _ = decrement_btn
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }

    {
        let counter_clone = counter.clone();
        let app_clone = app_element;

        if let Some(reset_btn) = document.get_element_by_id("reset-btn") {
            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                web_sys::console::log_1(&"🔄 Reset clicked".into());
                counter_clone.borrow().reset();
                render_counter(&counter_clone, &app_clone);
            }) as Box<dyn FnMut(_)>);

            let _ = reset_btn
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }

    web_sys::console::log_1(&"✅ Counter initialized and ready!".into());
}
