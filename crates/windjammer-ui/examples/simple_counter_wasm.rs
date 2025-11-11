//! Simple standalone WASM counter
//! This example demonstrates windjammer-ui working in the browser
//!
//! Build: wasm-pack build --target web --example simple_counter_wasm
//! Serve: python3 -m http.server 8080

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    web_sys::console::log_1(&"🚀 Windjammer UI Counter starting...".into());

    // Create reactive counter state
    let count = Signal::new(0);

    // Get the app div
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let app = document
        .get_element_by_id("app")
        .expect("should have #app element");

    // Render function
    let render = {
        let count = count.clone();
        let app = app.clone();
        move || {
            // Create UI using windjammer-ui components
            let button_dec = Button::new("-")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Large);

            let button_inc = Button::new("+")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Large);

            let button_reset = Button::new("Reset")
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Medium);

            // Build the UI
            let ui_html = format!(
                r#"
                <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
                            display: flex; justify-content: center; align-items: center; 
                            min-height: 100vh; background: #f0f0f0;">
                    <div style="background: white; padding: 3rem; border-radius: 12px; 
                                box-shadow: 0 4px 20px rgba(0,0,0,0.1); text-align: center;">
                        <h1 style="color: #333; margin: 0 0 2rem 0; font-size: 2.5rem;">
                            Windjammer Counter
                        </h1>
                        <div style="font-size: 4rem; font-weight: bold; color: #007bff; margin: 2rem 0;">
                            {}
                        </div>
                        <div style="display: flex; gap: 1rem; justify-content: center; margin-top: 2rem;">
                            <button id="dec-btn" style="background: #6c757d; color: white; border: none; 
                                                        padding: 1rem 2rem; border-radius: 8px; cursor: pointer; 
                                                        font-size: 1.5rem; font-weight: bold;">
                                -
                            </button>
                            <button id="reset-btn" style="background: #dc3545; color: white; border: none; 
                                                          padding: 1rem 2rem; border-radius: 8px; cursor: pointer; 
                                                          font-size: 1rem;">
                                Reset
                            </button>
                            <button id="inc-btn" style="background: #007bff; color: white; border: none; 
                                                        padding: 1rem 2rem; border-radius: 8px; cursor: pointer; 
                                                        font-size: 1.5rem; font-weight: bold;">
                                +
                            </button>
                        </div>
                        <p style="margin-top: 2rem; color: #666; font-size: 0.9rem;">
                            Built with Windjammer UI 🚀
                        </p>
                    </div>
                </div>
                "#,
                count.get()
            );

            app.set_inner_html(&ui_html);
        }
    };

    // Initial render
    render();

    // Set up event handlers
    let setup_handlers = {
        let count = count.clone();
        let render = Rc::new(RefCell::new(render));

        move || -> Result<(), JsValue> {
            let document = web_sys::window().unwrap().document().unwrap();

            // Decrement button
            {
                let count = count.clone();
                let render = render.clone();
                let dec_btn = document.get_element_by_id("dec-btn").unwrap();
                let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    count.update(|c| *c -= 1);
                    render.borrow()();
                }) as Box<dyn FnMut(_)>);
                dec_btn
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
                closure.forget();
            }

            // Increment button
            {
                let count = count.clone();
                let render = render.clone();
                let inc_btn = document.get_element_by_id("inc-btn").unwrap();
                let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    count.update(|c| *c += 1);
                    render.borrow()();
                }) as Box<dyn FnMut(_)>);
                inc_btn
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
                closure.forget();
            }

            // Reset button
            {
                let count = count.clone();
                let render = render.clone();
                let reset_btn = document.get_element_by_id("reset-btn").unwrap();
                let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    count.set(0);
                    render.borrow()();
                }) as Box<dyn FnMut(_)>);
                reset_btn
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
                closure.forget();
            }

            Ok(())
        }
    };

    setup_handlers()?;

    web_sys::console::log_1(&"✅ Counter ready!".into());

    Ok(())
}
