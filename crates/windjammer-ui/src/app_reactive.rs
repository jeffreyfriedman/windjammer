/// Reactive application runtime for WASM
/// Simpler version without the complex winit+wgpu setup
use crate::simple_vnode::VNode;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

// Global render callback for triggering re-renders
static mut RENDER_CALLBACK: Option<Box<dyn Fn()>> = None;

pub fn set_render_callback<F: Fn() + 'static>(callback: F) {
    unsafe {
        RENDER_CALLBACK = Some(Box::new(callback));
    }
}

pub fn trigger_rerender() {
    unsafe {
        if let Some(callback) = &RENDER_CALLBACK {
            callback();
        }
    }
}

/// Reactive application that automatically re-renders when signals change
pub struct ReactiveApp {
    title: String,
    render_fn: Rc<dyn Fn() -> VNode>,
}

impl ReactiveApp {
    pub fn new<F>(title: String, render_fn: F) -> Self
    where
        F: Fn() -> VNode + 'static,
    {
        Self {
            title,
            render_fn: Rc::new(render_fn),
        }
    }

    pub fn run(self) {
        use wasm_bindgen::JsCast;
        use web_sys::{window, HtmlElement};

        let document = window().unwrap().document().unwrap();
        let root = document
            .get_element_by_id("app")
            .expect("Failed to find #app element")
            .dyn_into::<HtmlElement>()
            .unwrap();

        let render_fn = self.render_fn.clone();
        let needs_rerender = Rc::new(RefCell::new(true));

        let needs_rerender_clone = needs_rerender.clone();
        set_render_callback(move || {
            *needs_rerender_clone.borrow_mut() = true;
        });

        let render_fn_clone = render_fn.clone();
        let needs_rerender_clone = needs_rerender.clone();

        let render_loop = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
        let render_loop_clone = render_loop.clone();

        *render_loop.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if *needs_rerender_clone.borrow() {
                *needs_rerender_clone.borrow_mut() = false;

                let vnode = render_fn_clone();
                let html = crate::renderer::render_to_html(&vnode);
                root.set_inner_html(&html);

                crate::renderer::attach_event_listeners(&root, &vnode);
            }

            web_sys::window()
                .unwrap()
                .request_animation_frame(
                    render_loop_clone
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unchecked_ref(),
                )
                .unwrap();
        }) as Box<dyn FnMut()>));

        web_sys::window()
            .unwrap()
            .request_animation_frame(
                render_loop
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            )
            .unwrap();
    }
}
