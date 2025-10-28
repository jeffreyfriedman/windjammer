//! Simple counter WASM application
//!
//! Build with: wasm-pack build --target web

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This binary is only for WASM target.");
    println!("Build with: wasm-pack build --target web");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // WASM entry point is handled by wasm_bindgen(start)
}

#[cfg(target_arch = "wasm32")]
mod app {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use windjammer_ui::reactivity::Signal;
    use windjammer_ui::vdom::{VElement, VNode, VText};
    use windjammer_ui::wasm_events;

    /// The counter application state
    pub struct CounterApp {
        count: Signal<i32>,
        root_element: Option<web_sys::Element>,
    }

    impl CounterApp {
        pub fn new() -> Self {
            Self {
                count: Signal::new(0),
                root_element: None,
            }
        }

        /// Render the counter UI
        pub fn render(&self) -> VNode {
            let count_value = self.count.get();

            VNode::Element(
                VElement::new("div")
                    .attr("class", "counter-app")
                    .child(VNode::Element(VElement::new("h1").child(VNode::Text(
                        VText::new(format!("Count: {}", count_value)),
                    ))))
                    .child(VNode::Element(
                        VElement::new("div")
                            .attr("class", "buttons")
                            .child(VNode::Element(
                                VElement::new("button")
                                    .attr("id", "decrement-btn")
                                    .attr("class", "btn")
                                    .child(VNode::Text(VText::new("-"))),
                            ))
                            .child(VNode::Element(
                                VElement::new("button")
                                    .attr("id", "reset-btn")
                                    .attr("class", "btn")
                                    .child(VNode::Text(VText::new("Reset"))),
                            ))
                            .child(VNode::Element(
                                VElement::new("button")
                                    .attr("id", "increment-btn")
                                    .attr("class", "btn")
                                    .child(VNode::Text(VText::new("+"))),
                            )),
                    )),
            )
        }

        /// Mount the app to the DOM
        pub fn mount(&mut self, selector: &str) -> Result<(), JsValue> {
            let window = web_sys::window().ok_or("No window")?;
            let document = window.document().ok_or("No document")?;

            // Find the root element
            let root = document
                .query_selector(selector)?
                .ok_or_else(|| JsValue::from_str(&format!("Element not found: {}", selector)))?;

            // Initial render
            self.update_dom(&root)?;
            self.root_element = Some(root);

            Ok(())
        }

        /// Update the DOM with current state
        fn update_dom(&self, root: &web_sys::Element) -> Result<(), JsValue> {
            let vnode = self.render();

            // Create DOM from VNode
            let window = web_sys::window().ok_or("No window")?;
            let document = window.document().ok_or("No document")?;
            let dom_node = create_dom_element(&document, &vnode)?;

            // Clear and update
            while let Some(child) = root.first_child() {
                root.remove_child(&child)?;
            }
            root.append_child(&dom_node)?;

            Ok(())
        }

        /// Re-render the app
        pub fn rerender(&self) -> Result<(), JsValue> {
            if let Some(root) = &self.root_element {
                self.update_dom(root)?;
            }
            Ok(())
        }
    }

    /// Create a DOM element from a VNode
    fn create_dom_element(
        document: &web_sys::Document,
        vnode: &VNode,
    ) -> Result<web_sys::Node, JsValue> {
        match vnode {
            VNode::Element(element) => {
                let dom_element = document.create_element(&element.tag)?;

                // Set attributes
                for (key, value) in &element.attrs {
                    dom_element.set_attribute(key, value)?;
                }

                // Append children
                for child in &element.children {
                    let child_node = create_dom_element(document, child)?;
                    dom_element.append_child(&child_node)?;
                }

                Ok(dom_element.into())
            }
            VNode::Text(text) => {
                let text_node = document.create_text_node(&text.content);
                Ok(text_node.into())
            }
            VNode::Component(_) => Err(JsValue::from_str("Cannot render component directly")),
            VNode::Empty => {
                let text_node = document.create_text_node("");
                Ok(text_node.into())
            }
        }
    }

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        // Set up panic hook
        console_error_panic_hook::set_once();

        web_sys::console::log_1(&"Windjammer Counter starting...".into());

        // Create the app
        let app = Rc::new(RefCell::new(CounterApp::new()));

        // Mount to DOM
        app.borrow_mut().mount("#app")?;

        web_sys::console::log_1(&"Counter mounted!".into());

        // Set up event handlers
        {
            let app_clone = app.clone();
            wasm_events::on_click("increment-btn", move |_| {
                let app = app_clone.borrow_mut();
                app.count.update(|c| *c += 1);
                let _ = app.rerender();
            })?;
        }

        {
            let app_clone = app.clone();
            wasm_events::on_click("decrement-btn", move |_| {
                let app = app_clone.borrow_mut();
                app.count.update(|c| *c -= 1);
                let _ = app.rerender();
            })?;
        }

        {
            let app_clone = app.clone();
            wasm_events::on_click("reset-btn", move |_| {
                let app = app_clone.borrow_mut();
                app.count.set(0);
                let _ = app.rerender();
            })?;
        }

        web_sys::console::log_1(&"Event handlers attached!".into());

        Ok(())
    }
}
