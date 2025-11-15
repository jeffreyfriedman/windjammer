/// Reactive application runtime using eframe
/// This is a simpler, more robust implementation than the manual winit+wgpu+egui approach
use crate::simple_vnode::VNode;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
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
        let ptr = &raw const RENDER_CALLBACK;
        if let Some(callback) = &*ptr {
            callback();
        }
    }
}

/// Reactive application that automatically re-renders when signals change
pub struct ReactiveApp {
    title: String,
    render_fn: Rc<dyn Fn() -> VNode>,
    needs_rerender: Arc<Mutex<bool>>,
}

impl ReactiveApp {
    pub fn new<F>(title: String, render_fn: F) -> Self
    where
        F: Fn() -> VNode + 'static,
    {
        Self {
            title,
            render_fn: Rc::new(render_fn),
            needs_rerender: Arc::new(Mutex::new(true)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn run(self) {
        // WASM implementation remains the same
        use wasm_bindgen::JsCast;
        use web_sys::{window, HtmlElement};

        let document = window().unwrap().document().unwrap();
        let root = document
            .get_element_by_id("app")
            .expect("Failed to find #app element")
            .dyn_into::<HtmlElement>()
            .unwrap();

        let render_fn = self.render_fn.clone();
        let needs_rerender = self.needs_rerender.clone();

        set_render_callback(move || {
            *needs_rerender.lock().unwrap() = true;
        });

        let render_fn_clone = render_fn.clone();
        let needs_rerender_clone = needs_rerender.clone();

        let render_loop = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
        let render_loop_clone = render_loop.clone();

        *render_loop.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if *needs_rerender_clone.lock().unwrap() {
                *needs_rerender_clone.lock().unwrap() = false;

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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(mut self) {
        #[cfg(feature = "desktop")]
        {
            match self.run_native_eframe() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to run app: {}", e);
                }
            }
        }

        #[cfg(not(feature = "desktop"))]
        {
            eprintln!("Native ReactiveApp requires 'desktop' feature to be enabled");
            eprintln!("Add to Cargo.toml: windjammer-ui = {{ features = [\"desktop\"] }}");
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    fn run_native_eframe(&mut self) -> Result<(), String> {
        use crate::desktop_renderer::DesktopRenderer;

        println!("🔧 Starting ReactiveApp (native with eframe)");

        // Set up re-render callback
        let needs_rerender = self.needs_rerender.clone();
        set_render_callback(move || {
            println!("🎨 Re-render requested");
            *needs_rerender.lock().unwrap() = true;
        });

        // Create the eframe app
        let render_fn = self.render_fn.clone();
        let needs_rerender = self.needs_rerender.clone();
        let mut renderer = DesktopRenderer::new();

        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_title(&self.title),
            ..Default::default()
        };

        eframe::run_simple_native(&self.title, native_options, move |ctx, _frame| {
            // Check if we need to re-render
            if *needs_rerender.lock().unwrap() {
                *needs_rerender.lock().unwrap() = false;
                ctx.request_repaint();
            }

            // Get VNode from render function
            let vnode = (render_fn)();

            // Render with our desktop renderer
            renderer.render(ctx, &vnode);
        })
        .map_err(|e| format!("eframe error: {}", e))
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[allow(dead_code)]
fn save_screenshot_to_file(image: &egui::ColorImage, path: &str) -> Result<(), String> {
    let width = image.width();
    let height = image.height();

    // Convert egui::ColorImage to PNG
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG header error: {}", e))?;

        // Convert Color32 to RGBA bytes
        let rgba_data: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|color| [color.r(), color.g(), color.b(), color.a()])
            .collect();

        writer
            .write_image_data(&rgba_data)
            .map_err(|e| format!("PNG write error: {}", e))?;
    }

    // Write to file
    std::fs::write(path, png_data).map_err(|e| format!("File write error: {}", e))?;

    Ok(())
}
