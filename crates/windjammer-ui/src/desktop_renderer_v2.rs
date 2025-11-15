/// Desktop renderer using egui - Version 2 with proper layout
#[cfg(not(target_arch = "wasm32"))]
use crate::simple_vnode::{VNode, VAttr};
#[cfg(not(target_arch = "wasm32"))]
use egui::{Context, Ui, RichText, Color32, Frame, Margin, Rounding, Stroke};
#[cfg(not(target_arch = "wasm32"))]
use std::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
pub struct DesktopRendererV2 {
    event_handlers: Rc<RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopRendererV2 {
    pub fn new() -> Self {
        Self {
            event_handlers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Render a VNode tree into egui using proper panel layout
    pub fn render(&mut self, ctx: &Context, vnode: &VNode) {
        // Extract the container's children (toolbar, main, console, status)
        if let VNode::Element { children, .. } = vnode {
            if children.len() >= 4 {
                // Render toolbar at top
                egui::TopBottomPanel::top("toolbar")
                    .exact_height(50.0)
                    .frame(Frame::none().fill(Color32::from_rgb(45, 45, 48)).inner_margin(Margin::same(8.0)))
                    .show(ctx, |ui| {
                        self.render_vnode(ui, &children[0]);
                    });

                // Render status bar at bottom
                egui::TopBottomPanel::bottom("status_bar")
                    .exact_height(30.0)
                    .frame(Frame::none().fill(Color32::from_rgb(0, 122, 204)).inner_margin(Margin::same(4.0)))
                    .show(ctx, |ui| {
                        if children.len() > 2 {
                            self.render_vnode(ui, &children[children.len() - 1]);
                        }
                    });

                // Render console at bottom (above status bar)
                egui::TopBottomPanel::bottom("console")
                    .exact_height(200.0)
                    .frame(Frame::none().fill(Color32::from_rgb(30, 30, 30)))
                    .show(ctx, |ui| {
                        if children.len() > 2 {
                            self.render_vnode(ui, &children[children.len() - 2]);
                        }
                    });

                // Render main content in center (fills remaining space)
                egui::CentralPanel::default()
                    .frame(Frame::none().fill(Color32::from_rgb(30, 30, 30)))
                    .show(ctx, |ui| {
                        if children.len() > 1 {
                            self.render_vnode(ui, &children[1]);
                        }
                    });
            } else {
                // Fallback: render everything in central panel
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_vnode(ui, vnode);
                });
            }
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_vnode(ui, vnode);
            });
        }
    }

    fn render_vnode(&mut self, ui: &mut Ui, vnode: &VNode) {
        match vnode {
            VNode::Element { tag, attrs, children } => {
                self.render_element(ui, tag, attrs, children);
            }
            VNode::Text(text) => {
                ui.label(text);
            }
        }
    }

    fn render_element(&mut self, ui: &mut Ui, tag: &str, attrs: &[(String, VAttr)], children: &[VNode]) {
        let classes = self.get_attr_value(attrs, "class");
        
        // Check for special components
        if classes.contains("split-panel") && classes.contains("horizontal") {
            self.render_horizontal_split(ui, children);
        } else if classes.contains("split-panel") {
            self.render_vertical_split(ui, children);
        } else if classes.contains("wj-panel") {
            self.render_panel(ui, attrs, children);
        } else if classes.contains("tree-view") {
            self.render_tree_view(ui, children);
        } else if tag == "button" {
            self.render_button(ui, attrs, children);
        } else if classes.contains("wj-flex") {
            let style = self.get_attr_value(attrs, "style");
            if style.contains("flex-direction: row") {
                ui.horizontal(|ui| {
                    for child in children {
                        self.render_vnode(ui, child);
                    }
                });
            } else {
                ui.vertical(|ui| {
                    for child in children {
                        self.render_vnode(ui, child);
                    }
                });
            }
        } else {
            // Default: vertical layout
            ui.vertical(|ui| {
                for child in children {
                    self.render_vnode(ui, child);
                }
            });
        }
    }

    fn render_horizontal_split(&mut self, ui: &mut Ui, children: &[VNode]) {
        // Use egui's columns for proper horizontal layout
        ui.columns(3, |columns| {
            if children.len() >= 1 {
                self.render_vnode(&mut columns[0], &children[0]);
            }
            if children.len() >= 2 {
                // Middle column gets the nested split panel
                if let VNode::Element { children: nested_children, .. } = &children[1] {
                    if nested_children.len() >= 2 {
                        // This is another horizontal split
                        columns[1].columns(2, |inner_cols| {
                            self.render_vnode(&mut inner_cols[0], &nested_children[0]);
                            self.render_vnode(&mut inner_cols[1], &nested_children[1]);
                        });
                    } else {
                        self.render_vnode(&mut columns[1], &children[1]);
                    }
                } else {
                    self.render_vnode(&mut columns[1], &children[1]);
                }
            }
            if children.len() >= 3 {
                self.render_vnode(&mut columns[2], &children[2]);
            }
        });
    }

    fn render_vertical_split(&mut self, ui: &mut Ui, children: &[VNode]) {
        let available_height = ui.available_height();
        
        for (i, child) in children.iter().enumerate() {
            if i > 0 {
                ui.separator();
            }
            
            let height = if i == 0 {
                available_height * 0.6
            } else {
                available_height * 0.4
            };
            
            ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                self.render_vnode(ui, child);
            });
        }
    }

    fn render_panel(&mut self, ui: &mut Ui, attrs: &[(String, VAttr)], children: &[VNode]) {
        // Extract title from first child
        let mut title = String::new();
        let mut body_children = Vec::new();
        
        for child in children {
            if let VNode::Element { tag, children: inner, .. } = child {
                if tag == "div" {
                    if let Some(VNode::Text(text)) = inner.first() {
                        if title.is_empty() {
                            title = text.clone();
                        } else {
                            body_children.extend(inner.clone());
                        }
                    } else {
                        body_children.extend(inner.clone());
                    }
                }
            }
        }
        
        Frame::group(ui.style())
            .fill(Color32::from_rgb(37, 37, 38))
            .stroke(Stroke::new(1.0, Color32::from_rgb(62, 62, 66)))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if !title.is_empty() {
                        ui.heading(&title);
                        ui.separator();
                    }
                    
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for child in &body_children {
                            self.render_vnode(ui, child);
                        }
                    });
                });
            });
    }

    fn render_tree_view(&mut self, ui: &mut Ui, children: &[VNode]) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for child in children {
                if let VNode::Element { children: item_children, attrs, .. } = child {
                    let text = self.get_text_content(item_children);
                    let response = ui.selectable_label(false, text);
                    
                    if response.clicked() {
                        if let Some(handler) = self.get_event_handler(attrs, "on_click") {
                            handler.borrow_mut()();
                        }
                    }
                }
            }
        });
    }

    fn render_button(&mut self, ui: &mut Ui, attrs: &[(String, VAttr)], children: &[VNode]) {
        let label = self.get_text_content(children);
        let classes = self.get_attr_value(attrs, "class");
        
        let (bg_color, text_color) = if classes.contains("wj-button-primary") {
            (Color32::from_rgb(0, 122, 204), Color32::WHITE)
        } else if classes.contains("wj-button-success") {
            (Color32::from_rgb(40, 167, 69), Color32::WHITE)
        } else {
            (Color32::from_rgb(108, 117, 125), Color32::WHITE)
        };

        let button = egui::Button::new(RichText::new(&label).color(text_color))
            .fill(bg_color)
            .rounding(Rounding::same(4.0));

        if ui.add(button).clicked() {
            println!("🔘 Button '{}' clicked!", label);
            if let Some(handler) = self.get_event_handler(attrs, "on_click") {
                handler.borrow_mut()();
            }
        }
    }

    // Helper methods
    fn get_attr_value(&self, attrs: &[(String, VAttr)], name: &str) -> String {
        attrs.iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                VAttr::Static(s) | VAttr::Dynamic(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn get_event_handler(&self, attrs: &[(String, VAttr)], name: &str) -> Option<Rc<RefCell<dyn FnMut()>>> {
        attrs.iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                VAttr::Event(handler) => Some(handler.clone()),
                _ => None,
            })
    }

    fn get_text_content(&self, children: &[VNode]) -> String {
        children.iter()
            .filter_map(|child| match child {
                VNode::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

