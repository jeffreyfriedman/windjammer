//! Reactive app runtime with automatic re-rendering
//!
//! This module provides a reactive application runtime that automatically
//! re-renders the UI when signals change.

use crate::simple_vnode::{VAttr, VNode};
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

// ============================================================================
// NATIVE IMPLEMENTATION (Desktop)
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    /// Global render callback that can be triggered by signal updates
    static RENDER_CALLBACK: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
}

/// Set the global render callback (native version)
#[cfg(not(target_arch = "wasm32"))]
pub fn set_render_callback(callback: impl Fn() + 'static) {
    RENDER_CALLBACK.with(|rc| {
        *rc.borrow_mut() = Some(Box::new(callback));
    });
}

/// Trigger a re-render (called by signals when they change) - native version
#[cfg(not(target_arch = "wasm32"))]
pub fn trigger_rerender() {
    println!("🔄 Triggering re-render...");
    RENDER_CALLBACK.with(|rc| {
        if let Some(callback) = rc.borrow().as_ref() {
            callback();
        } else {
            eprintln!("⚠️ No render callback set!");
        }
    });
}

/// Reactive app that re-renders when signals change (native version)
#[cfg(not(target_arch = "wasm32"))]
pub struct ReactiveApp {
    title: String,
    render_fn: Rc<dyn Fn() -> VNode>,
    needs_rerender: Arc<Mutex<bool>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ReactiveApp {
    /// Create a new reactive app
    pub fn new(title: impl Into<String>, render_fn: impl Fn() -> VNode + 'static) -> Self {
        Self {
            title: title.into(),
            render_fn: Rc::new(render_fn),
            needs_rerender: Arc::new(Mutex::new(false)),
        }
    }

    /// Mount and run the app
    pub fn run(mut self) {
        #[cfg(feature = "desktop")]
        {
            match self.run_native() {
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

    #[cfg(feature = "desktop")]
    fn run_native(&mut self) -> Result<(), String> {
        use crate::desktop_renderer::DesktopRenderer;
        use egui_wgpu::wgpu;
        use egui_wgpu::WgpuConfiguration;
        use egui_winit::EventResponse;
        use pollster::FutureExt;
        use winit::application::ApplicationHandler;
        use winit::event::WindowEvent;
        use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
        use winit::window::{Window, WindowId};

        println!("🔧 Starting ReactiveApp (native with egui)");

        // Create event loop
        let event_loop =
            EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
        event_loop.set_control_flow(ControlFlow::Wait);

        // Set up re-render callback
        let needs_rerender = self.needs_rerender.clone();

        set_render_callback(move || {
            println!("🎨 Re-render requested");
            *needs_rerender.lock().unwrap() = true;
        });

        // Create application handler
        struct App {
            window: Option<Arc<Window>>,
            title: String,
            render_fn: Rc<dyn Fn() -> VNode>,
            needs_rerender: Arc<Mutex<bool>>,
            egui_ctx: Option<egui::Context>,
            egui_state: Option<egui_winit::State>,
            egui_renderer: Option<egui_wgpu::Renderer>,
            wgpu_state: Option<WgpuState>,
            renderer: DesktopRenderer,
        }

        struct WgpuState {
            device: wgpu::Device,
            queue: wgpu::Queue,
            surface: wgpu::Surface<'static>,
            surface_config: wgpu::SurfaceConfiguration,
        }

        impl ApplicationHandler for App {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.window.is_none() {
                    let window_attrs = Window::default_attributes()
                        .with_title(&self.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(1200, 800));

                    match event_loop.create_window(window_attrs) {
                        Ok(window) => {
                            println!("✅ Window created");
                            let window = Arc::new(window);

                            // Initialize wgpu
                            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                                backends: wgpu::Backends::PRIMARY,
                                ..Default::default()
                            });

                            let surface = instance.create_surface(window.clone()).unwrap();

                            let adapter = instance
                                .request_adapter(&wgpu::RequestAdapterOptions {
                                    power_preference: wgpu::PowerPreference::default(),
                                    compatible_surface: Some(&surface),
                                    force_fallback_adapter: false,
                                })
                                .block_on()
                                .unwrap();

                            let (device, queue) = adapter
                                .request_device(
                                    &wgpu::DeviceDescriptor {
                                        label: None,
                                        required_features: wgpu::Features::empty(),
                                        required_limits: wgpu::Limits::default(),
                                        memory_hints: wgpu::MemoryHints::default(),
                                    },
                                    None,
                                )
                                .block_on()
                                .unwrap();

                            let size = window.inner_size();
                            let surface_config = wgpu::SurfaceConfiguration {
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                format: surface.get_capabilities(&adapter).formats[0],
                                width: size.width,
                                height: size.height,
                                present_mode: wgpu::PresentMode::Fifo,
                                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                                view_formats: vec![],
                                desired_maximum_frame_latency: 2,
                            };
                            surface.configure(&device, &surface_config);

                            // Initialize egui
                            let egui_ctx = egui::Context::default();
                            let egui_state = egui_winit::State::new(
                                egui_ctx.clone(),
                                egui_ctx.viewport_id(),
                                &window,
                                None,
                                None,
                                None,
                            );

                            let egui_renderer = egui_wgpu::Renderer::new(
                                &device,
                                surface_config.format,
                                None,
                                1,
                                false,
                            );

                            self.egui_ctx = Some(egui_ctx);
                            self.egui_state = Some(egui_state);
                            self.egui_renderer = Some(egui_renderer);
                            self.wgpu_state = Some(WgpuState {
                                device,
                                queue,
                                surface,
                                surface_config,
                            });

                            println!("✅ egui + wgpu initialized");

                            // Request initial draw before moving window
                            window.request_redraw();
                            self.window = Some(window);
                        }
                        Err(e) => {
                            eprintln!("Failed to create window: {}", e);
                        }
                    }
                }
            }

            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                _window_id: WindowId,
                event: WindowEvent,
            ) {
                match event {
                    WindowEvent::CloseRequested => {
                        println!("👋 Window close requested");
                        event_loop.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        self.handle_redraw();
                    }
                    ref event => {
                        // Let egui handle the event
                        if let (Some(egui_state), Some(window)) =
                            (&mut self.egui_state, &self.window)
                        {
                            let event_response = egui_state.on_window_event(window, event);

                            // Request redraw if egui wants it
                            if event_response.repaint {
                                window.request_redraw();
                            }
                        }
                    }
                }
            }
        }

        impl App {
            fn handle_redraw(&mut self) {
                // Render with egui
                if let (
                    Some(window),
                    Some(egui_ctx),
                    Some(egui_state),
                    Some(egui_renderer),
                    Some(wgpu_state),
                ) = (
                    &self.window,
                    &self.egui_ctx,
                    &mut self.egui_state,
                    &mut self.egui_renderer,
                    &mut self.wgpu_state,
                ) {
                    // Get VNode from render function
                    let vnode = (self.render_fn)();

                    // Prepare egui input
                    let raw_input = egui_state.take_egui_input(window);

                    // Run egui
                    let full_output = egui_ctx.run(raw_input, |ctx| {
                        self.renderer.render(ctx, &vnode);
                    });

                    // Handle platform output
                    egui_state.handle_platform_output(window, full_output.platform_output);

                    // Render egui
                    let primitives =
                        egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

                    let surface_texture = wgpu_state.surface.get_current_texture().unwrap();
                    let surface_view = surface_texture
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let screen_descriptor = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [
                            wgpu_state.surface_config.width,
                            wgpu_state.surface_config.height,
                        ],
                        pixels_per_point: window.scale_factor() as f32,
                    };

                    // Update textures
                    for (id, image_delta) in &full_output.textures_delta.set {
                        egui_renderer.update_texture(
                            &wgpu_state.device,
                            &wgpu_state.queue,
                            *id,
                            image_delta,
                        );
                    }

                    // Update buffers - submit immediately to avoid lifetime issues
                    {
                        let mut encoder = wgpu_state.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("egui buffer update"),
                            },
                        );
                        egui_renderer.update_buffers(
                            &wgpu_state.device,
                            &wgpu_state.queue,
                            &mut encoder,
                            &primitives,
                            &screen_descriptor,
                        );
                        wgpu_state.queue.submit(Some(encoder.finish()));
                    }

                    // Render with a new encoder
                    let mut encoder =
                        wgpu_state
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("egui render"),
                            });

                    {
                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("egui render pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &surface_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });

                        // forget_lifetime() is needed because render() requires 'static lifetime
                        egui_renderer.render(
                            &mut render_pass.forget_lifetime(),
                            &primitives,
                            &screen_descriptor,
                        );
                    } // render_pass dropped here

                    let command_buffer = encoder.finish();

                    for id in &full_output.textures_delta.free {
                        egui_renderer.free_texture(id);
                    }

                    wgpu_state.queue.submit(std::iter::once(command_buffer));
                    surface_texture.present();

                    // Only request another redraw if state changed
                    if *self.needs_rerender.lock().unwrap() {
                        *self.needs_rerender.lock().unwrap() = false;
                        window.request_redraw();
                    }
                }
            }
        }

        let mut app = App {
            window: None,
            title: self.title.clone(),
            render_fn: self.render_fn.clone(),
            needs_rerender: self.needs_rerender.clone(),
            egui_ctx: None,
            egui_state: None,
            egui_renderer: None,
            wgpu_state: None,
            renderer: DesktopRenderer::new(),
        };

        event_loop
            .run_app(&mut app)
            .map_err(|e| format!("Event loop error: {}", e))?;

        Ok(())
    }
}

/// Helper function to print VNode structure (for debugging)
#[cfg(not(target_arch = "wasm32"))]
fn print_vnode_summary(vnode: &VNode, indent: usize) {
    let prefix = "  ".repeat(indent);
    match vnode {
        VNode::Element {
            tag,
            attrs,
            children,
        } => {
            let attr_summary = if attrs.is_empty() {
                String::new()
            } else {
                let attr_strs: Vec<String> = attrs
                    .iter()
                    .map(|(name, attr)| match attr {
                        VAttr::Static(value) => format!("{}={}", name, value),
                        VAttr::Dynamic(value) => format!("{}={}", name, value),
                        VAttr::Event(_) => format!("{}=<handler>", name),
                    })
                    .collect();
                format!(" [{}]", attr_strs.join(", "))
            };
            println!("{}<{}{}>", prefix, tag, attr_summary);
            for child in children {
                print_vnode_summary(child, indent + 1);
            }
        }
        VNode::Text(text) => {
            let preview = if text.len() > 50 {
                format!("{}...", &text[..50])
            } else {
                text.clone()
            };
            println!("{}\"{}\"", prefix, preview);
        }
    }
}
