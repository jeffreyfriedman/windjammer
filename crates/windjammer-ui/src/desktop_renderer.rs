/// Desktop renderer using egui + wgpu
/// Converts VNode tree to egui UI
#[cfg(not(target_arch = "wasm32"))]
use crate::simple_vnode::{VAttr, VNode};
#[cfg(not(target_arch = "wasm32"))]
use egui::{Color32, Context, Frame, Margin, RichText, Rounding, Stroke, Ui};
#[cfg(not(target_arch = "wasm32"))]
use std::cell::RefCell;
#[cfg(not(target_arch = "wasm32"))]
use std::rc::Rc;

#[cfg(not(target_arch = "wasm32"))]
pub struct DesktopRenderer {
    // Store event handlers for buttons
    #[allow(dead_code)]
    event_handlers: Rc<RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopRenderer {
    pub fn new() -> Self {
        Self {
            event_handlers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Render a VNode tree into egui
    pub fn render(&mut self, ctx: &Context, vnode: &VNode) {
        // Render the root VNode, which should be a Container
        // We need to handle it specially to fill the entire window
        egui::CentralPanel::default()
            .frame(Frame::none().fill(Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                // Get full screen size
                let screen_size = ctx.screen_rect().size();

                // Allocate the full screen for our content
                ui.allocate_ui_with_layout(
                    screen_size,
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.render_vnode(ui, vnode);
                    },
                );
            });
    }

    pub fn render_vnode(&mut self, ui: &mut Ui, vnode: &VNode) {
        match vnode {
            VNode::Element {
                tag,
                attrs,
                children,
            } => {
                self.render_element(ui, tag, attrs, children);
            }
            VNode::Text(text) => {
                ui.label(text);
            }
        }
    }

    fn render_element(
        &mut self,
        ui: &mut Ui,
        tag: &str,
        attrs: &[(String, VAttr)],
        children: &[VNode],
    ) {
        // Extract classes and styles
        let classes = self.get_attr_value(attrs, "class");
        let style = self.get_attr_value(attrs, "style");

        // Check for special component classes
        if classes.contains("tree-view") {
            self.render_tree_view(ui, children);
        } else if classes.contains("tab-panel") {
            self.render_tab_panel(ui, children);
        } else if classes.contains("split-panel") {
            self.render_split_panel(ui, &classes, children);
        } else if classes.contains("advanced-code-editor") {
            self.render_advanced_code_editor(ui, children);
        } else {
            match tag {
                "button" => self.render_button(ui, attrs, children),
                "div" => self.render_div(ui, &classes, &style, attrs, children),
                "span" | "p" => self.render_text_container(ui, children),
                "input" => self.render_input(ui, attrs),
                "textarea" => self.render_textarea(ui, attrs),
                "pre" => self.render_code_block(ui, children),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.render_heading(ui, tag, children),
                _ => {
                    // Generic container
                    ui.vertical(|ui| {
                        for child in children {
                            self.render_vnode(ui, child);
                        }
                    });
                }
            }
        }
    }

    fn render_button(&mut self, ui: &mut Ui, attrs: &[(String, VAttr)], children: &[VNode]) {
        let label = self.get_text_content(children);
        let disabled = self.get_attr_value(attrs, "disabled") == "true";
        let variant = self.get_variant_from_classes(attrs);

        // Style button based on variant
        let (bg_color, text_color) = match variant.as_str() {
            "primary" => (Color32::from_rgb(0, 122, 204), Color32::WHITE),
            "danger" => (Color32::from_rgb(220, 53, 69), Color32::WHITE),
            "success" => (Color32::from_rgb(40, 167, 69), Color32::WHITE),
            "warning" => (Color32::from_rgb(255, 193, 7), Color32::BLACK),
            _ => (Color32::from_rgb(108, 117, 125), Color32::WHITE),
        };

        let button = egui::Button::new(RichText::new(&label).color(text_color))
            .fill(bg_color)
            .rounding(Rounding::same(3.0));

        let response = ui.add_enabled(!disabled, button);

        // Handle click event
        if response.clicked() {
            println!("🔘 Button '{}' CLICKED!", label);
            if let Some(handler) = self.get_event_handler(attrs, "on_click") {
                handler.borrow_mut()();
            }
        }
    }

    fn render_div(
        &mut self,
        ui: &mut Ui,
        classes: &str,
        style: &str,
        _attrs: &[(String, VAttr)],
        children: &[VNode],
    ) {
        // Parse style for background color, width, height
        let bg_color = if style.contains("background-color:") {
            self.parse_color_from_style(style)
        } else {
            None
        };

        // Determine layout direction from style
        let is_flex_row = style.contains("flex-direction: row") || classes.contains("wj-flex");
        let is_flex_col =
            style.contains("flex-direction: column") || classes.contains("wj-container");

        // Determine if this is a panel
        let is_panel = classes.contains("wj-panel");

        if is_panel {
            self.render_panel(ui, children);
        } else if is_flex_row {
            // Horizontal flex with resizable panels
            let available_width = ui.available_width();
            let available_height = ui.available_height();

            let frame = if let Some(color) = bg_color {
                Frame::none().fill(color).inner_margin(Margin::same(8.0))
            } else {
                Frame::none()
            };

            frame.show(ui, |ui| {
                // Simple horizontal layout with equal distribution
                // TODO: Add proper resizable panels with egui_dock
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    let child_width = (available_width - ((children.len() as f32 - 1.0) * 4.0))
                        / children.len() as f32;

                    for child in children {
                        ui.allocate_ui_with_layout(
                            egui::vec2(child_width, available_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                self.render_vnode(ui, child);
                            },
                        );
                    }
                });
            });
        } else if is_flex_col || classes.contains("wj-container") {
            // For containers, fill available space
            if classes.contains("wj-container") {
                // Container should fill all space and distribute to children
                ui.vertical(|ui| {
                    let total_height = ui.available_height();
                    let remaining_height = total_height;

                    // First pass: render fixed-height children (toolbar, status bar)
                    // Second pass: give remaining space to flexible children
                    let child_count = children.len();

                    for (i, child) in children.iter().enumerate() {
                        // Estimate height for each child
                        let height = if i == 0 {
                            // Toolbar - fixed height
                            50.0
                        } else if i == child_count - 1 {
                            // Status bar - fixed height
                            30.0
                        } else {
                            // Main content - use remaining space
                            remaining_height - 50.0 - 30.0
                        };

                        if height > 0.0 {
                            ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
                                self.render_vnode(ui, child);
                            });
                        } else {
                            self.render_vnode(ui, child);
                        }
                    }
                });
            } else if let Some(color) = bg_color {
                Frame::none().fill(color).show(ui, |ui| {
                    ui.vertical(|ui| {
                        for child in children {
                            self.render_vnode(ui, child);
                        }
                    });
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

    fn render_panel(&mut self, ui: &mut Ui, children: &[VNode]) {
        // Find panel header and body
        let mut header_text = String::new();
        let mut body_children = Vec::new();

        for child in children {
            if let VNode::Element { tag, children, .. } = child {
                if tag == "div" {
                    // Check if this is header or body by looking at children
                    if let Some(VNode::Text(text)) = children.first() {
                        if header_text.is_empty() {
                            header_text = text.clone();
                        } else {
                            body_children.extend(children.clone());
                        }
                    } else {
                        body_children.extend(children.clone());
                    }
                }
            }
        }

        // Allocate all available space for the panel
        let available_size = ui.available_size();
        ui.allocate_ui(available_size, |ui| {
            Frame::group(ui.style())
                .fill(Color32::from_rgb(37, 37, 38))
                .stroke(Stroke::new(1.0, Color32::from_rgb(62, 62, 66)))
                .rounding(Rounding::same(5.0))
                .inner_margin(Margin::same(0.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Collapsible header
                        if !header_text.is_empty() {
                            let header_id = ui.make_persistent_id(&header_text);
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                header_id,
                                true,
                            )
                            .show_header(ui, |ui| {
                                Frame::none()
                                    .fill(Color32::from_rgb(45, 45, 48))
                                    .inner_margin(Margin::same(8.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(&header_text).strong());
                                    });
                            })
                            .body(|ui| {
                                // Body - fill remaining space
                                Frame::none()
                                    .inner_margin(Margin::same(8.0))
                                    .show(ui, |ui| {
                                        egui::ScrollArea::both().auto_shrink([false, false]).show(
                                            ui,
                                            |ui| {
                                                for child in &body_children {
                                                    self.render_vnode(ui, child);
                                                }
                                            },
                                        );
                                    });
                            });
                        } else {
                            // No header, just render body
                            Frame::none()
                                .inner_margin(Margin::same(8.0))
                                .show(ui, |ui| {
                                    egui::ScrollArea::both().auto_shrink([false, false]).show(
                                        ui,
                                        |ui| {
                                            for child in &body_children {
                                                self.render_vnode(ui, child);
                                            }
                                        },
                                    );
                                });
                        }
                    });
                });
        });
    }

    fn render_text_container(&mut self, ui: &mut Ui, children: &[VNode]) {
        let text = self.get_text_content(children);
        ui.label(text);
    }

    fn render_input(&mut self, ui: &mut Ui, attrs: &[(String, VAttr)]) {
        let _placeholder = self.get_attr_value(attrs, "placeholder");
        let value = self.get_attr_value(attrs, "value");

        let mut text = value;
        ui.text_edit_singleline(&mut text);
    }

    fn render_textarea(&mut self, ui: &mut Ui, attrs: &[(String, VAttr)]) {
        let value = self.get_attr_value(attrs, "value");
        let mut text = value;

        ui.add(
            egui::TextEdit::multiline(&mut text)
                .desired_width(f32::INFINITY)
                .desired_rows(10),
        );
    }

    fn render_heading(&mut self, ui: &mut Ui, tag: &str, children: &[VNode]) {
        let text = self.get_text_content(children);
        let size = match tag {
            "h1" => 32.0,
            "h2" => 24.0,
            "h3" => 20.0,
            "h4" => 18.0,
            "h5" => 16.0,
            "h6" => 14.0,
            _ => 16.0,
        };

        ui.heading(RichText::new(text).size(size));
    }

    // Helper methods

    fn get_attr_value(&self, attrs: &[(String, VAttr)], name: &str) -> String {
        attrs
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                VAttr::Static(s) | VAttr::Dynamic(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn get_event_handler(
        &self,
        attrs: &[(String, VAttr)],
        name: &str,
    ) -> Option<Rc<RefCell<dyn FnMut()>>> {
        attrs
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| match v {
                VAttr::Event(handler) => Some(handler.clone()),
                _ => None,
            })
    }

    fn get_text_content(&self, children: &[VNode]) -> String {
        children
            .iter()
            .filter_map(|child| match child {
                VNode::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn get_variant_from_classes(&self, attrs: &[(String, VAttr)]) -> String {
        let classes = self.get_attr_value(attrs, "class");
        if classes.contains("wj-button-primary") {
            "primary".to_string()
        } else if classes.contains("wj-button-danger") {
            "danger".to_string()
        } else if classes.contains("wj-button-success") {
            "success".to_string()
        } else if classes.contains("wj-button-warning") {
            "warning".to_string()
        } else {
            "secondary".to_string()
        }
    }

    fn parse_color_from_style(&self, style: &str) -> Option<Color32> {
        // Simple parser for "background-color: #RRGGBB"
        if let Some(start) = style.find("background-color:") {
            let rest = &style[start + 17..].trim();
            if let Some(end) = rest.find(';').or_else(|| Some(rest.len())) {
                let color_str = rest[..end].trim();
                if color_str.starts_with('#') && color_str.len() == 7 {
                    if let Ok(r) = u8::from_str_radix(&color_str[1..3], 16) {
                        if let Ok(g) = u8::from_str_radix(&color_str[3..5], 16) {
                            if let Ok(b) = u8::from_str_radix(&color_str[5..7], 16) {
                                return Some(Color32::from_rgb(r, g, b));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // Advanced component renderers

    fn render_tree_view(&mut self, ui: &mut Ui, children: &[VNode]) {
        egui::ScrollArea::vertical()
            .id_salt("tree_view_scroll")
            .show(ui, |ui| {
                for child in children {
                    if let VNode::Element {
                        attrs,
                        children: item_children,
                        ..
                    } = child
                    {
                        if self.get_attr_value(attrs, "class").contains("tree-item") {
                            self.render_tree_item(ui, item_children, attrs);
                        }
                    }
                }
            });
    }

    fn render_tree_item(&mut self, ui: &mut Ui, children: &[VNode], attrs: &[(String, VAttr)]) {
        let text = self.get_text_content(children);

        let response = ui.selectable_label(false, RichText::new(text).monospace());

        if response.clicked() {
            if let Some(handler) = self.get_event_handler(attrs, "on_click") {
                handler.borrow_mut()();
            }
        }
    }

    fn render_tab_panel(&mut self, ui: &mut Ui, children: &[VNode]) {
        // Find tab bar and content
        for child in children {
            if let VNode::Element {
                tag: _tag,
                attrs,
                children: tab_children,
            } = child
            {
                let classes = self.get_attr_value(attrs, "class");

                if classes.contains("tab-bar") {
                    // Render tab buttons horizontally
                    ui.horizontal(|ui| {
                        for tab_child in tab_children {
                            if let VNode::Element {
                                tag,
                                children: btn_children,
                                attrs: btn_attrs,
                            } = tab_child
                            {
                                if tag == "button" {
                                    let label = self.get_text_content(btn_children);
                                    let is_active =
                                        self.get_attr_value(btn_attrs, "class").contains("active");

                                    let button = if is_active {
                                        egui::Button::new(RichText::new(label).strong())
                                            .fill(Color32::from_rgb(0, 122, 204))
                                    } else {
                                        egui::Button::new(label).fill(Color32::from_rgb(60, 60, 60))
                                    };

                                    ui.add(button);
                                }
                            }
                        }
                    });
                } else {
                    // Render content
                    self.render_vnode(ui, child);
                }
            }
        }
    }

    fn render_split_panel(&mut self, ui: &mut Ui, classes: &str, children: &[VNode]) {
        let is_horizontal = classes.contains("horizontal");

        if is_horizontal {
            // Horizontal split - use egui's horizontal layout with proper sizing
            ui.horizontal(|ui| {
                // Force the horizontal container to use full width
                ui.set_min_width(ui.available_width());

                let total_width = ui.available_width();
                let height = ui.available_height();

                for (i, child) in children.iter().enumerate() {
                    if i > 0 {
                        ui.separator();
                    }

                    // Calculate width for each panel
                    let width = if i == 0 {
                        total_width * 0.20 // Left: 20%
                    } else if i == children.len() - 1 {
                        total_width * 0.20 // Right: 20%
                    } else {
                        total_width * 0.60 // Middle: 60%
                    };

                    // Use a fixed-size container
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_size(egui::vec2(width, height));
                            self.render_vnode(ui, child);
                        },
                    );
                }
            });
        } else {
            // Vertical split
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width());

                let width = ui.available_width();
                let total_height = ui.available_height();

                for (i, child) in children.iter().enumerate() {
                    if i > 0 {
                        ui.separator();
                    }

                    let height = if i == 0 {
                        total_height * 0.60 // Top: 60%
                    } else {
                        total_height * 0.40 // Bottom: 40%
                    };

                    ui.allocate_ui_with_layout(
                        egui::vec2(width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_size(egui::vec2(width, height));
                            self.render_vnode(ui, child);
                        },
                    );
                }
            });
        }
    }

    fn render_advanced_code_editor(&mut self, ui: &mut Ui, children: &[VNode]) {
        egui::ScrollArea::both()
            .id_salt("code_editor_scroll")
            .show(ui, |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(30, 30, 30))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Find line numbers and code content
                            for child in children {
                                if let VNode::Element {
                                    attrs,
                                    children: content_children,
                                    ..
                                } = child
                                {
                                    let classes = self.get_attr_value(attrs, "class");

                                    if classes.contains("line-numbers") {
                                        ui.vertical(|ui| {
                                            ui.style_mut().override_text_style =
                                                Some(egui::TextStyle::Monospace);
                                            for line_child in content_children {
                                                if let VNode::Text(text) = line_child {
                                                    ui.label(
                                                        RichText::new(text).color(
                                                            Color32::from_rgb(100, 100, 100),
                                                        ),
                                                    );
                                                }
                                            }
                                        });
                                    } else if classes.contains("code-content") {
                                        ui.vertical(|ui| {
                                            ui.style_mut().override_text_style =
                                                Some(egui::TextStyle::Monospace);
                                            for code_child in content_children {
                                                self.render_code_with_highlighting(ui, code_child);
                                            }
                                        });
                                    }
                                }
                            }
                        });
                    });
            });
    }

    fn render_code_with_highlighting(&mut self, ui: &mut Ui, vnode: &VNode) {
        match vnode {
            VNode::Element {
                tag,
                children,
                attrs,
            } => {
                if tag == "pre" {
                    for child in children {
                        self.render_code_span(ui, child);
                    }
                } else if tag == "span" {
                    let classes = self.get_attr_value(attrs, "class");
                    let color = match classes.as_str() {
                        "keyword" => Color32::from_rgb(86, 156, 214), // Blue for keywords
                        "type" => Color32::from_rgb(78, 201, 176),    // Teal for types
                        "string" => Color32::from_rgb(206, 145, 120), // Orange for strings
                        "comment" => Color32::from_rgb(106, 153, 85), // Green for comments
                        _ => Color32::from_rgb(212, 212, 212),        // Default white
                    };

                    let text = self.get_text_content(children);
                    ui.label(RichText::new(text).color(color));
                }
            }
            VNode::Text(text) => {
                ui.label(RichText::new(text).color(Color32::from_rgb(212, 212, 212)));
            }
        }
    }

    fn render_code_span(&mut self, ui: &mut Ui, vnode: &VNode) {
        match vnode {
            VNode::Element { children, .. } => {
                for child in children {
                    self.render_code_span(ui, child);
                }
            }
            VNode::Text(text) => {
                ui.label(RichText::new(text).color(Color32::from_rgb(212, 212, 212)));
            }
        }
    }

    fn render_code_block(&mut self, ui: &mut Ui, children: &[VNode]) {
        Frame::none()
            .fill(Color32::from_rgb(30, 30, 30))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                for child in children {
                    self.render_code_span(ui, child);
                }
            });
    }
}
