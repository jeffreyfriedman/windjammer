//! Reactive app runtime with automatic re-rendering
//!
//! This module provides a reactive application runtime that automatically
//! re-renders the UI when signals change.

use crate::simple_vnode::VNode;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Element};

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Global render callback that can be triggered by signal updates
    static RENDER_CALLBACK: RefCell<Option<Rc<dyn Fn()>>> = RefCell::new(None);
}

/// Set the global render callback
#[cfg(target_arch = "wasm32")]
pub fn set_render_callback(callback: impl Fn() + 'static) {
    RENDER_CALLBACK.with(|rc| {
        *rc.borrow_mut() = Some(Rc::new(callback));
    });
}

/// Trigger a re-render (called by signals when they change)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn trigger_rerender() {
    web_sys::console::log_1(&"🔄 Triggering re-render...".into());
    RENDER_CALLBACK.with(|rc| {
        if let Some(callback) = rc.borrow().as_ref() {
            callback();
        } else {
            web_sys::console::warn_1(&"⚠️ No render callback set!".into());
        }
    });
}

/// Reactive app that re-renders when signals change
#[cfg(target_arch = "wasm32")]
pub struct ReactiveApp {
    title: String,
    render_fn: Rc<dyn Fn() -> VNode>,
    root_element: Option<Element>,
    document: Option<web_sys::Document>,
}

#[cfg(target_arch = "wasm32")]
impl ReactiveApp {
    /// Create a new reactive app
    pub fn new(title: impl Into<String>, render_fn: impl Fn() -> VNode + 'static) -> Self {
        Self {
            title: title.into(),
            render_fn: Rc::new(render_fn),
            root_element: None,
            document: None,
        }
    }

    /// Mount and run the app
    pub fn run(mut self) {
        match self.run_internal() {
            Ok(_) => {}
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to mount app: {:?}", e).into());
            }
        }
    }

    fn run_internal(&mut self) -> Result<(), JsValue> {
        // Set up panic hook
        console_error_panic_hook::set_once();

        web_sys::console::log_1(&"🔧 Starting ReactiveApp".into());

        // Get window and document
        let window = window().ok_or("No window found")?;
        let document = window.document().ok_or("No document found")?;

        // Set document title
        document.set_title(&self.title);

        // Get root element
        let root_element = document
            .get_element_by_id("app")
            .or_else(|| document.body().map(|b| b.into()))
            .ok_or("No root element found")?;

        // Store for re-renders
        self.root_element = Some(root_element.clone());
        self.document = Some(document.clone());

        // Initial render
        self.render()?;

        // Set up re-render callback
        let render_fn = self.render_fn.clone();
        let root_elem = root_element.clone();
        let doc = document.clone();

        set_render_callback(move || {
            web_sys::console::log_1(&"🎨 Re-rendering...".into());

            // Clear and re-render
            root_elem.set_inner_html("");
            let vnode = render_fn();
            match vnode.render(&doc) {
                Ok(rendered) => {
                    if let Err(e) = root_elem.append_child(&rendered) {
                        web_sys::console::error_1(&format!("Re-render error: {:?}", e).into());
                    } else {
                        web_sys::console::log_1(&"✅ Re-render complete!".into());
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Render error: {:?}", e).into());
                }
            }
        });

        web_sys::console::log_1(&"✅ ReactiveApp mounted!".into());

        Ok(())
    }

    fn render(&self) -> Result<(), JsValue> {
        let root_element = self.root_element.as_ref().ok_or("No root element")?;
        let document = self.document.as_ref().ok_or("No document")?;

        // Clear existing content
        root_element.set_inner_html("");

        // Render the VNode
        let vnode = (self.render_fn)();
        let rendered = vnode.render(document)?;
        root_element.append_child(&rendered)?;

        Ok(())
    }
}
