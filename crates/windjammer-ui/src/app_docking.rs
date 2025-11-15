/// Docking-enabled reactive application using egui_dock
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use crate::simple_vnode::VNode;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct DockingApp {
    title: String,
    render_fn: Rc<dyn Fn() -> VNode>,
    needs_rerender: Arc<Mutex<bool>>,
    dock_state: Arc<Mutex<egui_dock::DockState<String>>>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl DockingApp {
    pub fn new<F>(title: String, render_fn: F) -> Self
    where
        F: Fn() -> VNode + 'static,
    {
        // Create initial dock layout
        let mut dock_state = egui_dock::DockState::new(vec!["main".to_string()]);

        // Set up initial layout: Files | Editor | Properties
        let main_surface = dock_state.main_surface_mut();
        let [_main, _right] = main_surface.split_right(
            egui_dock::NodeIndex::root(),
            0.8, // 80% for left side
            vec!["properties".to_string()],
        );
        let [_files, _editor] = main_surface.split_left(
            egui_dock::NodeIndex::root(),
            0.2, // 20% for files
            vec!["files".to_string()],
        );

        // Add console at bottom
        let [_, _console] = main_surface.split_below(
            egui_dock::NodeIndex::root(),
            0.75, // 75% for top
            vec!["console".to_string()],
        );

        Self {
            title,
            render_fn: Rc::new(render_fn),
            needs_rerender: Arc::new(Mutex::new(true)),
            dock_state: Arc::new(Mutex::new(dock_state)),
        }
    }

    pub fn run(self) {
        use crate::desktop_renderer::DesktopRenderer;

        println!("🔧 Starting DockingApp (native with egui_dock)");

        // Set up re-render callback
        let needs_rerender = self.needs_rerender.clone();
        crate::app_reactive_eframe::set_render_callback(move || {
            *needs_rerender.lock().unwrap() = true;
        });

        let render_fn = self.render_fn.clone();
        let needs_rerender = self.needs_rerender.clone();
        let dock_state = self.dock_state.clone();
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

            // Extract panel data from VNode
            let panels = extract_panels_from_vnode(&vnode);

            // Render with egui_dock
            let mut dock_state_guard = dock_state.lock().unwrap();

            egui_dock::DockArea::new(&mut *dock_state_guard).show(
                ctx,
                &mut TabViewer {
                    panels,
                    renderer: &mut renderer,
                },
            );
        })
        .map_err(|e| format!("eframe error: {}", e))
        .unwrap();
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
struct TabViewer<'a> {
    panels: std::collections::HashMap<String, VNode>,
    renderer: &'a mut crate::desktop_renderer::DesktopRenderer,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        if let Some(vnode) = self.panels.get(tab) {
            self.renderer.render_vnode(ui, vnode);
        } else {
            ui.label(format!("Panel '{}' not found", tab));
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn extract_panels_from_vnode(vnode: &VNode) -> std::collections::HashMap<String, VNode> {
    use std::collections::HashMap;

    let mut panels = HashMap::new();

    // Simple extraction: look for Panel components in the tree
    // For now, just create placeholder panels
    panels.insert("files".to_string(), vnode.clone());
    panels.insert("main".to_string(), vnode.clone());
    panels.insert("properties".to_string(), vnode.clone());
    panels.insert("console".to_string(), vnode.clone());

    panels
}
