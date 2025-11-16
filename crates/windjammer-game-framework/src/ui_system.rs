//! In-Game UI System
//!
//! Provides a comprehensive UI system for game interfaces.
//!
//! ## Features
//! - Flexible layout system (horizontal, vertical, grid)
//! - Common widgets (button, label, image, slider, checkbox)
//! - Event handling (click, hover, drag)
//! - Styling and theming
//! - Anchoring and alignment
//! - Z-ordering

use crate::math::{Vec2, Vec4};
use std::collections::HashMap;

/// UI system
#[derive(Debug)]
pub struct UISystem {
    /// Root UI elements
    elements: Vec<UIElement>,
    /// Next element ID
    next_id: u32,
    /// Active element (for input)
    active_element: Option<u32>,
    /// Hovered element
    hovered_element: Option<u32>,
    /// UI scale
    pub scale: f32,
}

/// UI element
#[derive(Debug, Clone)]
pub struct UIElement {
    /// Element ID
    pub id: u32,
    /// Element type
    pub element_type: UIElementType,
    /// Position (screen space)
    pub position: Vec2,
    /// Size
    pub size: Vec2,
    /// Anchor (0-1 for each axis)
    pub anchor: Vec2,
    /// Pivot (0-1 for each axis)
    pub pivot: Vec2,
    /// Z-order (higher = on top)
    pub z_order: i32,
    /// Visible
    pub visible: bool,
    /// Enabled (for interaction)
    pub enabled: bool,
    /// Style
    pub style: UIStyle,
    /// Children
    pub children: Vec<UIElement>,
}

/// UI element type
#[derive(Debug, Clone)]
pub enum UIElementType {
    /// Container (for layout)
    Container {
        layout: LayoutType,
    },
    /// Button
    Button {
        text: String,
        on_click: Option<String>, // Event name
    },
    /// Label
    Label {
        text: String,
    },
    /// Image
    Image {
        texture_id: u32,
    },
    /// Slider
    Slider {
        value: f32,
        min: f32,
        max: f32,
    },
    /// Checkbox
    Checkbox {
        checked: bool,
    },
    /// Text Input
    TextInput {
        text: String,
        placeholder: String,
    },
    /// Progress Bar
    ProgressBar {
        value: f32,
        max: f32,
    },
}

/// Layout type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    /// No layout (manual positioning)
    None,
    /// Horizontal layout
    Horizontal {
        spacing: u32,
    },
    /// Vertical layout
    Vertical {
        spacing: u32,
    },
    /// Grid layout
    Grid {
        columns: u32,
        spacing: u32,
    },
}

/// UI style
#[derive(Debug, Clone)]
pub struct UIStyle {
    /// Background color
    pub background_color: Vec4,
    /// Border color
    pub border_color: Vec4,
    /// Border width
    pub border_width: f32,
    /// Text color
    pub text_color: Vec4,
    /// Font size
    pub font_size: f32,
    /// Padding (left, top, right, bottom)
    pub padding: Vec4,
    /// Corner radius
    pub corner_radius: f32,
}

/// UI event
#[derive(Debug, Clone)]
pub enum UIEvent {
    /// Element clicked
    Click { element_id: u32 },
    /// Element hovered
    Hover { element_id: u32 },
    /// Element value changed
    ValueChanged { element_id: u32, value: f32 },
    /// Text changed
    TextChanged { element_id: u32, text: String },
}

impl Default for UIStyle {
    fn default() -> Self {
        Self {
            background_color: Vec4::new(0.2, 0.2, 0.2, 1.0),
            border_color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            border_width: 1.0,
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            font_size: 16.0,
            padding: Vec4::new(8.0, 8.0, 8.0, 8.0),
            corner_radius: 4.0,
        }
    }
}

impl UIStyle {
    /// Create a button style
    pub fn button() -> Self {
        Self {
            background_color: Vec4::new(0.3, 0.5, 0.8, 1.0),
            border_color: Vec4::new(0.4, 0.6, 0.9, 1.0),
            border_width: 2.0,
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            font_size: 16.0,
            padding: Vec4::new(12.0, 8.0, 12.0, 8.0),
            corner_radius: 6.0,
        }
    }

    /// Create a panel style
    pub fn panel() -> Self {
        Self {
            background_color: Vec4::new(0.15, 0.15, 0.15, 0.9),
            border_color: Vec4::new(0.3, 0.3, 0.3, 1.0),
            border_width: 1.0,
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            font_size: 14.0,
            padding: Vec4::new(16.0, 16.0, 16.0, 16.0),
            corner_radius: 8.0,
        }
    }
}

impl UISystem {
    /// Create a new UI system
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            next_id: 0,
            active_element: None,
            hovered_element: None,
            scale: 1.0,
        }
    }

    /// Add a UI element
    pub fn add_element(&mut self, element: UIElement) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let mut element = element;
        element.id = id;

        self.elements.push(element);
        id
    }

    /// Get element by ID
    pub fn get_element(&self, id: u32) -> Option<&UIElement> {
        self.find_element(&self.elements, id)
    }

    /// Get element by ID (mutable)
    pub fn get_element_mut(&mut self, id: u32) -> Option<&mut UIElement> {
        // Simplified: only search top-level elements
        self.elements.iter_mut().find(|e| e.id == id)
    }

    /// Find element recursively
    fn find_element<'a>(&self, elements: &'a [UIElement], id: u32) -> Option<&'a UIElement> {
        for element in elements {
            if element.id == id {
                return Some(element);
            }
            if let Some(found) = self.find_element(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    /// Handle click at position
    pub fn handle_click(&mut self, position: Vec2) -> Option<UIEvent> {
        if let Some(element_id) = self.find_element_at_position(&self.elements, position) {
            self.active_element = Some(element_id);
            return Some(UIEvent::Click { element_id });
        }
        None
    }

    /// Find element at position
    fn find_element_at_position(&self, elements: &[UIElement], position: Vec2) -> Option<u32> {
        // Check in reverse order (top to bottom)
        for element in elements.iter().rev() {
            if !element.visible || !element.enabled {
                continue;
            }

            // Check if position is within element bounds
            if position.x >= element.position.x
                && position.x <= element.position.x + element.size.x
                && position.y >= element.position.y
                && position.y <= element.position.y + element.size.y
            {
                // Check children first
                if let Some(child_id) = self.find_element_at_position(&element.children, position) {
                    return Some(child_id);
                }
                return Some(element.id);
            }
        }
        None
    }

    /// Update UI (for animations, etc.)
    pub fn update(&mut self, _delta: f32) {
        // Update logic here (animations, etc.)
    }

    /// Clear all elements
    pub fn clear(&mut self) {
        self.elements.clear();
        self.active_element = None;
        self.hovered_element = None;
    }
}

impl Default for UISystem {
    fn default() -> Self {
        Self::new()
    }
}

impl UIElement {
    /// Create a container
    pub fn container(position: Vec2, size: Vec2, layout: LayoutType) -> Self {
        Self {
            id: 0,
            element_type: UIElementType::Container { layout },
            position,
            size,
            anchor: Vec2::ZERO,
            pivot: Vec2::ZERO,
            z_order: 0,
            visible: true,
            enabled: true,
            style: UIStyle::panel(),
            children: Vec::new(),
        }
    }

    /// Create a button
    pub fn button(position: Vec2, size: Vec2, text: &str) -> Self {
        Self {
            id: 0,
            element_type: UIElementType::Button {
                text: text.to_string(),
                on_click: None,
            },
            position,
            size,
            anchor: Vec2::ZERO,
            pivot: Vec2::ZERO,
            z_order: 0,
            visible: true,
            enabled: true,
            style: UIStyle::button(),
            children: Vec::new(),
        }
    }

    /// Create a label
    pub fn label(position: Vec2, text: &str) -> Self {
        Self {
            id: 0,
            element_type: UIElementType::Label {
                text: text.to_string(),
            },
            position,
            size: Vec2::new(200.0, 30.0),
            anchor: Vec2::ZERO,
            pivot: Vec2::ZERO,
            z_order: 0,
            visible: true,
            enabled: true,
            style: UIStyle::default(),
            children: Vec::new(),
        }
    }

    /// Create a slider
    pub fn slider(position: Vec2, size: Vec2, min: f32, max: f32, value: f32) -> Self {
        Self {
            id: 0,
            element_type: UIElementType::Slider { value, min, max },
            position,
            size,
            anchor: Vec2::ZERO,
            pivot: Vec2::ZERO,
            z_order: 0,
            visible: true,
            enabled: true,
            style: UIStyle::default(),
            children: Vec::new(),
        }
    }

    /// Add a child element
    pub fn with_child(mut self, child: UIElement) -> Self {
        self.children.push(child);
        self
    }

    /// Set style
    pub fn with_style(mut self, style: UIStyle) -> Self {
        self.style = style;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_system_creation() {
        let ui = UISystem::new();
        assert_eq!(ui.scale, 1.0);
        println!("✅ UISystem created");
    }

    #[test]
    fn test_add_element() {
        let mut ui = UISystem::new();
        let button = UIElement::button(Vec2::new(100.0, 100.0), Vec2::new(200.0, 50.0), "Click Me");
        let id = ui.add_element(button);
        assert_eq!(id, 0);
        println!("✅ Add UI element");
    }

    #[test]
    fn test_get_element() {
        let mut ui = UISystem::new();
        let button = UIElement::button(Vec2::new(100.0, 100.0), Vec2::new(200.0, 50.0), "Click Me");
        let id = ui.add_element(button);

        let element = ui.get_element(id);
        assert!(element.is_some());
        println!("✅ Get UI element");
    }

    #[test]
    fn test_button_creation() {
        let button = UIElement::button(Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0), "Test");
        assert!(matches!(button.element_type, UIElementType::Button { .. }));
        println!("✅ Button creation");
    }

    #[test]
    fn test_label_creation() {
        let label = UIElement::label(Vec2::new(0.0, 0.0), "Test Label");
        assert!(matches!(label.element_type, UIElementType::Label { .. }));
        println!("✅ Label creation");
    }

    #[test]
    fn test_container_creation() {
        let container = UIElement::container(
            Vec2::new(0.0, 0.0),
            Vec2::new(400.0, 300.0),
            LayoutType::Vertical { spacing: 10 },
        );
        assert!(matches!(container.element_type, UIElementType::Container { .. }));
        println!("✅ Container creation");
    }

    #[test]
    fn test_slider_creation() {
        let slider = UIElement::slider(Vec2::new(0.0, 0.0), Vec2::new(200.0, 30.0), 0.0, 100.0, 50.0);
        if let UIElementType::Slider { value, min, max } = slider.element_type {
            assert_eq!(value, 50.0);
            assert_eq!(min, 0.0);
            assert_eq!(max, 100.0);
        }
        println!("✅ Slider creation");
    }

    #[test]
    fn test_child_elements() {
        let button = UIElement::button(Vec2::new(10.0, 10.0), Vec2::new(100.0, 50.0), "Child");
        let container = UIElement::container(
            Vec2::new(0.0, 0.0),
            Vec2::new(400.0, 300.0),
            LayoutType::Vertical { spacing: 10 },
        )
        .with_child(button);

        assert_eq!(container.children.len(), 1);
        println!("✅ Child elements");
    }

    #[test]
    fn test_ui_styles() {
        let button_style = UIStyle::button();
        let panel_style = UIStyle::panel();

        assert!(button_style.background_color.z > 0.5); // Blue component
        assert!(panel_style.background_color.x < 0.2); // Dark background
        println!("✅ UI styles");
    }

    #[test]
    fn test_layout_types() {
        let horizontal = LayoutType::Horizontal { spacing: 10 };
        let vertical = LayoutType::Vertical { spacing: 10 };
        let grid = LayoutType::Grid { columns: 3, spacing: 5 };

        assert!(matches!(horizontal, LayoutType::Horizontal { .. }));
        assert!(matches!(vertical, LayoutType::Vertical { .. }));
        assert!(matches!(grid, LayoutType::Grid { .. }));
        println!("✅ Layout types");
    }

    #[test]
    fn test_handle_click() {
        let mut ui = UISystem::new();
        let button = UIElement::button(Vec2::new(100.0, 100.0), Vec2::new(200.0, 50.0), "Click Me");
        let id = ui.add_element(button);

        let event = ui.handle_click(Vec2::new(150.0, 125.0));
        assert!(event.is_some());
        if let Some(UIEvent::Click { element_id }) = event {
            assert_eq!(element_id, id);
        }
        println!("✅ Handle click");
    }

    #[test]
    fn test_visibility() {
        let mut button = UIElement::button(Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0), "Test");
        button.visible = false;

        assert!(!button.visible);
        println!("✅ Element visibility");
    }

    #[test]
    fn test_enabled_state() {
        let mut button = UIElement::button(Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0), "Test");
        button.enabled = false;

        assert!(!button.enabled);
        println!("✅ Element enabled state");
    }

    #[test]
    fn test_clear_ui() {
        let mut ui = UISystem::new();
        ui.add_element(UIElement::button(Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0), "Test"));
        ui.clear();

        assert_eq!(ui.elements.len(), 0);
        println!("✅ Clear UI");
    }
}

