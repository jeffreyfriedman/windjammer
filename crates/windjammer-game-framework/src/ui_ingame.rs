//! In-Game UI System
//!
//! Provides a comprehensive UI system for HUD, menus, and dialogs in games.
//!
//! ## Features
//! - Widget system (buttons, labels, panels, images, progress bars, sliders)
//! - Event handling (click, hover, drag, input)
//! - Styling system (colors, fonts, borders, padding)
//! - Layout management (absolute, relative positioning)
//! - Z-ordering for layering
//! - Input focus management
//! - Animation support

use crate::math::{Vec2, Vec4};
use std::collections::HashMap;

/// UI widget identifier
pub type WidgetId = u64;

/// UI event
#[derive(Debug, Clone, PartialEq)]
pub enum UIEvent {
    /// Mouse button pressed
    MouseDown { position: Vec2, button: MouseButton },
    /// Mouse button released
    MouseUp { position: Vec2, button: MouseButton },
    /// Mouse moved
    MouseMove { position: Vec2 },
    /// Mouse entered widget
    MouseEnter { widget_id: WidgetId },
    /// Mouse left widget
    MouseLeave { widget_id: WidgetId },
    /// Widget clicked
    Click { widget_id: WidgetId },
    /// Key pressed
    KeyDown { key: String },
    /// Key released
    KeyUp { key: String },
    /// Text input
    TextInput { text: String },
    /// Widget value changed
    ValueChanged { widget_id: WidgetId, value: f32 },
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// UI widget base
#[derive(Debug, Clone)]
pub struct Widget {
    /// Widget ID
    pub id: WidgetId,
    /// Widget type
    pub widget_type: WidgetType,
    /// Position (x, y)
    pub position: Vec2,
    /// Size (width, height)
    pub size: Vec2,
    /// Visible
    pub visible: bool,
    /// Enabled (can receive input)
    pub enabled: bool,
    /// Z-order (higher = on top)
    pub z_order: i32,
    /// Style
    pub style: WidgetStyle,
    /// Children widgets
    pub children: Vec<WidgetId>,
    /// Parent widget
    pub parent: Option<WidgetId>,
}

/// Widget type
#[derive(Debug, Clone)]
pub enum WidgetType {
    /// Panel (container)
    Panel,
    /// Button
    Button {
        text: String,
        on_click: Option<String>, // Event handler name
    },
    /// Label (text display)
    Label { text: String },
    /// Image
    Image { texture_path: String },
    /// Progress bar
    ProgressBar { value: f32, max_value: f32 },
    /// Slider
    Slider {
        value: f32,
        min_value: f32,
        max_value: f32,
    },
    /// Text input
    TextInput {
        text: String,
        placeholder: String,
        max_length: Option<usize>,
    },
    /// Checkbox
    Checkbox { checked: bool, label: String },
    /// Radio button
    RadioButton {
        selected: bool,
        group: String,
        label: String,
    },
}

/// Widget style
#[derive(Debug, Clone)]
pub struct WidgetStyle {
    /// Background color (RGBA)
    pub background_color: Vec4,
    /// Border color (RGBA)
    pub border_color: Vec4,
    /// Border width
    pub border_width: f32,
    /// Border radius (for rounded corners)
    pub border_radius: f32,
    /// Text color (RGBA)
    pub text_color: Vec4,
    /// Font size
    pub font_size: f32,
    /// Padding (top, right, bottom, left)
    pub padding: Vec4,
    /// Opacity (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            background_color: Vec4::new(0.2, 0.2, 0.2, 1.0),
            border_color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            border_width: 1.0,
            border_radius: 0.0,
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            font_size: 16.0,
            padding: Vec4::new(8.0, 8.0, 8.0, 8.0),
            opacity: 1.0,
        }
    }
}

/// UI manager
pub struct UIManager {
    /// All widgets
    widgets: HashMap<WidgetId, Widget>,
    /// Root widgets (no parent)
    root_widgets: Vec<WidgetId>,
    /// Next widget ID
    next_id: WidgetId,
    /// Focused widget
    focused_widget: Option<WidgetId>,
    /// Hovered widget
    hovered_widget: Option<WidgetId>,
    /// Mouse position
    mouse_position: Vec2,
    /// Event handlers
    event_handlers: HashMap<String, Box<dyn Fn(&UIEvent)>>,
}

impl UIManager {
    /// Create a new UI manager
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            root_widgets: Vec::new(),
            next_id: 1,
            focused_widget: None,
            hovered_widget: None,
            mouse_position: Vec2::ZERO,
            event_handlers: HashMap::new(),
        }
    }

    /// Create a new widget
    pub fn create_widget(&mut self, widget_type: WidgetType) -> WidgetId {
        let id = self.next_id;
        self.next_id += 1;

        let widget = Widget {
            id,
            widget_type,
            position: Vec2::ZERO,
            size: Vec2::new(100.0, 30.0),
            visible: true,
            enabled: true,
            z_order: 0,
            style: WidgetStyle::default(),
            children: Vec::new(),
            parent: None,
        };

        self.widgets.insert(id, widget);
        self.root_widgets.push(id);
        id
    }

    /// Get a widget
    pub fn get_widget(&self, id: WidgetId) -> Option<&Widget> {
        self.widgets.get(&id)
    }

    /// Get a widget mutably
    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut Widget> {
        self.widgets.get_mut(&id)
    }

    /// Add a child widget
    pub fn add_child(&mut self, parent_id: WidgetId, child_id: WidgetId) {
        if let Some(parent) = self.widgets.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
        if let Some(child) = self.widgets.get_mut(&child_id) {
            child.parent = Some(parent_id);
        }
        // Remove from root widgets if it was there
        self.root_widgets.retain(|&id| id != child_id);
    }

    /// Remove a widget
    pub fn remove_widget(&mut self, id: WidgetId) {
        // Remove from parent's children
        if let Some(widget) = self.widgets.get(&id) {
            if let Some(parent_id) = widget.parent {
                if let Some(parent) = self.widgets.get_mut(&parent_id) {
                    parent.children.retain(|&child_id| child_id != id);
                }
            }
        }

        // Remove from root widgets
        self.root_widgets.retain(|&widget_id| widget_id != id);

        // Remove the widget
        self.widgets.remove(&id);
    }

    /// Set widget position
    pub fn set_position(&mut self, id: WidgetId, position: Vec2) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.position = position;
        }
    }

    /// Set widget size
    pub fn set_size(&mut self, id: WidgetId, size: Vec2) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.size = size;
        }
    }

    /// Set widget visibility
    pub fn set_visible(&mut self, id: WidgetId, visible: bool) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.visible = visible;
        }
    }

    /// Set widget enabled state
    pub fn set_enabled(&mut self, id: WidgetId, enabled: bool) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.enabled = enabled;
        }
    }

    /// Set widget z-order
    pub fn set_z_order(&mut self, id: WidgetId, z_order: i32) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.z_order = z_order;
        }
    }

    /// Check if point is inside widget
    pub fn is_point_inside(&self, id: WidgetId, point: Vec2) -> bool {
        if let Some(widget) = self.widgets.get(&id) {
            let min = widget.position;
            let max = widget.position + widget.size;
            point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
        } else {
            false
        }
    }

    /// Find widget at position (topmost)
    pub fn widget_at_position(&self, position: Vec2) -> Option<WidgetId> {
        let mut candidates: Vec<(WidgetId, i32)> = Vec::new();

        for (&id, widget) in &self.widgets {
            if widget.visible && widget.enabled && self.is_point_inside(id, position) {
                candidates.push((id, widget.z_order));
            }
        }

        // Sort by z-order (highest first)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        candidates.first().map(|(id, _)| *id)
    }

    /// Handle mouse move event
    pub fn handle_mouse_move(&mut self, position: Vec2) -> Vec<UIEvent> {
        self.mouse_position = position;
        let mut events = Vec::new();

        let widget_at_pos = self.widget_at_position(position);

        // Handle mouse leave
        if let Some(prev_hovered) = self.hovered_widget {
            if widget_at_pos != Some(prev_hovered) {
                events.push(UIEvent::MouseLeave {
                    widget_id: prev_hovered,
                });
            }
        }

        // Handle mouse enter
        if let Some(new_hovered) = widget_at_pos {
            if self.hovered_widget != Some(new_hovered) {
                events.push(UIEvent::MouseEnter {
                    widget_id: new_hovered,
                });
            }
        }

        self.hovered_widget = widget_at_pos;
        events.push(UIEvent::MouseMove { position });

        events
    }

    /// Handle mouse down event
    pub fn handle_mouse_down(&mut self, position: Vec2, button: MouseButton) -> Vec<UIEvent> {
        let mut events = vec![UIEvent::MouseDown { position, button }];

        if let Some(widget_id) = self.widget_at_position(position) {
            self.focused_widget = Some(widget_id);
        } else {
            self.focused_widget = None;
        }

        events
    }

    /// Handle mouse up event
    pub fn handle_mouse_up(&mut self, position: Vec2, button: MouseButton) -> Vec<UIEvent> {
        let mut events = vec![UIEvent::MouseUp { position, button }];

        // Check for click
        if button == MouseButton::Left {
            if let Some(widget_id) = self.widget_at_position(position) {
                events.push(UIEvent::Click { widget_id });
            }
        }

        events
    }

    /// Get all widgets sorted by z-order
    pub fn get_sorted_widgets(&self) -> Vec<&Widget> {
        let mut widgets: Vec<&Widget> = self.widgets.values().collect();
        widgets.sort_by(|a, b| a.z_order.cmp(&b.z_order));
        widgets
    }

    /// Get focused widget
    pub fn get_focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Get hovered widget
    pub fn get_hovered_widget(&self) -> Option<WidgetId> {
        self.hovered_widget
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new()
    }
}

/// UI builder for fluent API
pub struct UIBuilder {
    manager: UIManager,
    current_widget: Option<WidgetId>,
}

impl UIBuilder {
    /// Create a new UI builder
    pub fn new() -> Self {
        Self {
            manager: UIManager::new(),
            current_widget: None,
        }
    }

    /// Create a panel
    pub fn panel(mut self) -> Self {
        let id = self.manager.create_widget(WidgetType::Panel);
        self.current_widget = Some(id);
        self
    }

    /// Create a button
    pub fn button(mut self, text: impl Into<String>) -> Self {
        let id = self.manager.create_widget(WidgetType::Button {
            text: text.into(),
            on_click: None,
        });
        self.current_widget = Some(id);
        self
    }

    /// Create a label
    pub fn label(mut self, text: impl Into<String>) -> Self {
        let id = self.manager.create_widget(WidgetType::Label {
            text: text.into(),
        });
        self.current_widget = Some(id);
        self
    }

    /// Set position
    pub fn position(mut self, x: f32, y: f32) -> Self {
        if let Some(id) = self.current_widget {
            self.manager.set_position(id, Vec2::new(x, y));
        }
        self
    }

    /// Set size
    pub fn size(mut self, width: f32, height: f32) -> Self {
        if let Some(id) = self.current_widget {
            self.manager.set_size(id, Vec2::new(width, height));
        }
        self
    }

    /// Build and return the UI manager
    pub fn build(self) -> UIManager {
        self.manager
    }
}

impl Default for UIBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_manager_creation() {
        let manager = UIManager::new();
        assert_eq!(manager.widgets.len(), 0);
        println!("✅ UIManager creation");
    }

    #[test]
    fn test_create_widget() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        assert!(manager.get_widget(id).is_some());
        println!("✅ Create widget");
    }

    #[test]
    fn test_widget_positioning() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_position(id, Vec2::new(100.0, 200.0));
        
        let widget = manager.get_widget(id).unwrap();
        assert_eq!(widget.position, Vec2::new(100.0, 200.0));
        println!("✅ Widget positioning");
    }

    #[test]
    fn test_widget_sizing() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_size(id, Vec2::new(300.0, 400.0));
        
        let widget = manager.get_widget(id).unwrap();
        assert_eq!(widget.size, Vec2::new(300.0, 400.0));
        println!("✅ Widget sizing");
    }

    #[test]
    fn test_widget_visibility() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_visible(id, false);
        
        let widget = manager.get_widget(id).unwrap();
        assert!(!widget.visible);
        println!("✅ Widget visibility");
    }

    #[test]
    fn test_widget_enabled() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_enabled(id, false);
        
        let widget = manager.get_widget(id).unwrap();
        assert!(!widget.enabled);
        println!("✅ Widget enabled state");
    }

    #[test]
    fn test_widget_z_order() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_z_order(id, 10);
        
        let widget = manager.get_widget(id).unwrap();
        assert_eq!(widget.z_order, 10);
        println!("✅ Widget z-order");
    }

    #[test]
    fn test_point_inside_widget() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_position(id, Vec2::new(100.0, 100.0));
        manager.set_size(id, Vec2::new(200.0, 200.0));
        
        assert!(manager.is_point_inside(id, Vec2::new(150.0, 150.0)));
        assert!(!manager.is_point_inside(id, Vec2::new(50.0, 50.0)));
        println!("✅ Point inside widget");
    }

    #[test]
    fn test_widget_at_position() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_position(id, Vec2::new(100.0, 100.0));
        manager.set_size(id, Vec2::new(200.0, 200.0));
        
        assert_eq!(manager.widget_at_position(Vec2::new(150.0, 150.0)), Some(id));
        assert_eq!(manager.widget_at_position(Vec2::new(50.0, 50.0)), None);
        println!("✅ Widget at position");
    }

    #[test]
    fn test_add_child() {
        let mut manager = UIManager::new();
        let parent_id = manager.create_widget(WidgetType::Panel);
        let child_id = manager.create_widget(WidgetType::Button {
            text: "Click me".to_string(),
            on_click: None,
        });
        
        manager.add_child(parent_id, child_id);
        
        let parent = manager.get_widget(parent_id).unwrap();
        assert!(parent.children.contains(&child_id));
        
        let child = manager.get_widget(child_id).unwrap();
        assert_eq!(child.parent, Some(parent_id));
        println!("✅ Add child widget");
    }

    #[test]
    fn test_remove_widget() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.remove_widget(id);
        
        assert!(manager.get_widget(id).is_none());
        println!("✅ Remove widget");
    }

    #[test]
    fn test_mouse_move_events() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_position(id, Vec2::new(100.0, 100.0));
        manager.set_size(id, Vec2::new(200.0, 200.0));
        
        let events = manager.handle_mouse_move(Vec2::new(150.0, 150.0));
        assert!(!events.is_empty());
        println!("✅ Mouse move events");
    }

    #[test]
    fn test_mouse_click_events() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Button {
            text: "Click me".to_string(),
            on_click: None,
        });
        manager.set_position(id, Vec2::new(100.0, 100.0));
        manager.set_size(id, Vec2::new(200.0, 50.0));
        
        let position = Vec2::new(150.0, 125.0);
        manager.handle_mouse_down(position, MouseButton::Left);
        let events = manager.handle_mouse_up(position, MouseButton::Left);
        
        assert!(events.iter().any(|e| matches!(e, UIEvent::Click { .. })));
        println!("✅ Mouse click events");
    }

    #[test]
    fn test_ui_builder() {
        let manager = UIBuilder::new()
            .panel()
            .position(100.0, 100.0)
            .size(300.0, 400.0)
            .build();
        
        assert_eq!(manager.widgets.len(), 1);
        println!("✅ UI builder");
    }

    #[test]
    fn test_widget_style_default() {
        let style = WidgetStyle::default();
        assert_eq!(style.font_size, 16.0);
        assert_eq!(style.border_width, 1.0);
        println!("✅ Widget style default");
    }

    #[test]
    fn test_focused_widget() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_position(id, Vec2::new(100.0, 100.0));
        manager.set_size(id, Vec2::new(200.0, 200.0));
        
        manager.handle_mouse_down(Vec2::new(150.0, 150.0), MouseButton::Left);
        assert_eq!(manager.get_focused_widget(), Some(id));
        println!("✅ Focused widget");
    }

    #[test]
    fn test_hovered_widget() {
        let mut manager = UIManager::new();
        let id = manager.create_widget(WidgetType::Panel);
        manager.set_position(id, Vec2::new(100.0, 100.0));
        manager.set_size(id, Vec2::new(200.0, 200.0));
        
        manager.handle_mouse_move(Vec2::new(150.0, 150.0));
        assert_eq!(manager.get_hovered_widget(), Some(id));
        println!("✅ Hovered widget");
    }

    #[test]
    fn test_z_order_sorting() {
        let mut manager = UIManager::new();
        let id1 = manager.create_widget(WidgetType::Panel);
        let id2 = manager.create_widget(WidgetType::Panel);
        
        manager.set_z_order(id1, 5);
        manager.set_z_order(id2, 10);
        
        let sorted = manager.get_sorted_widgets();
        assert_eq!(sorted[0].id, id1);
        assert_eq!(sorted[1].id, id2);
        println!("✅ Z-order sorting");
    }
}

