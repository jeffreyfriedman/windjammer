//! Immediate Mode UI System for Games
//!
//! Simple, game-focused UI inspired by Dear ImGui and egui.
//!
//! **Philosophy**: Zero crate leakage - clean, simple API for Windjammer games.

use crate::math::{Vec2, Vec4};
use crate::renderer::Color;
use std::collections::HashMap;

/// UI context (main entry point)
pub struct UI {
    /// Current frame's draw commands
    draw_list: Vec<DrawCommand>,
    
    /// Layout stack
    layout_stack: Vec<LayoutState>,
    
    /// Current layout state
    current_layout: LayoutState,
    
    /// Input state
    mouse_pos: Vec2,
    mouse_down: bool,
    mouse_clicked: bool,
    
    /// ID generation
    next_id: u64,
    
    /// Hot/active widget tracking
    hot_widget: Option<u64>,
    active_widget: Option<u64>,
    
    /// Style
    style: UIStyle,
}

/// Draw command
#[derive(Clone, Debug)]
pub enum DrawCommand {
    Rect {
        pos: Vec2,
        size: Vec2,
        color: Color,
        rounding: f32,
    },
    Text {
        pos: Vec2,
        text: String,
        color: Color,
        size: f32,
    },
    Line {
        start: Vec2,
        end: Vec2,
        color: Color,
        thickness: f32,
    },
}

/// Layout state
#[derive(Clone, Debug)]
struct LayoutState {
    /// Current cursor position
    cursor: Vec2,
    
    /// Layout direction
    direction: LayoutDirection,
    
    /// Available size
    size: Vec2,
    
    /// Padding
    padding: f32,
    
    /// Spacing between widgets
    spacing: f32,
}

/// Layout direction
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

/// UI style
#[derive(Clone, Debug)]
pub struct UIStyle {
    /// Default text size
    pub text_size: f32,
    
    /// Default text color
    pub text_color: Color,
    
    /// Button color
    pub button_color: Color,
    
    /// Button hover color
    pub button_hover_color: Color,
    
    /// Button active color
    pub button_active_color: Color,
    
    /// Window background color
    pub window_bg_color: Color,
    
    /// Padding
    pub padding: f32,
    
    /// Spacing
    pub spacing: f32,
    
    /// Rounding
    pub rounding: f32,
}

impl Default for UIStyle {
    fn default() -> Self {
        Self {
            text_size: 16.0,
            text_color: Color::rgb(1.0, 1.0, 1.0),
            button_color: Color::rgb(0.2, 0.4, 0.8),
            button_hover_color: Color::rgb(0.3, 0.5, 0.9),
            button_active_color: Color::rgb(0.1, 0.3, 0.7),
            window_bg_color: Color::new(0.1, 0.1, 0.1, 0.9),
            padding: 8.0,
            spacing: 4.0,
            rounding: 4.0,
        }
    }
}

impl UI {
    /// Create a new UI context
    pub fn new() -> Self {
        let initial_layout = LayoutState {
            cursor: Vec2::new(0.0, 0.0),
            direction: LayoutDirection::Vertical,
            size: Vec2::new(f32::INFINITY, f32::INFINITY),
            padding: 8.0,
            spacing: 4.0,
        };
        
        Self {
            draw_list: Vec::new(),
            layout_stack: Vec::new(),
            current_layout: initial_layout,
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_down: false,
            mouse_clicked: false,
            next_id: 0,
            hot_widget: None,
            active_widget: None,
            style: UIStyle::default(),
        }
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self, mouse_pos: Vec2, mouse_down: bool) {
        self.draw_list.clear();
        self.mouse_pos = mouse_pos;
        
        // Detect click (transition from up to down)
        self.mouse_clicked = mouse_down && !self.mouse_down;
        self.mouse_down = mouse_down;
        
        // Reset hot widget (will be set by widgets this frame)
        self.hot_widget = None;
    }
    
    /// End the current frame
    pub fn end_frame(&mut self) {
        // If mouse was released, clear active widget
        if !self.mouse_down {
            self.active_widget = None;
        }
    }
    
    /// Get draw commands for rendering
    pub fn draw_commands(&self) -> &[DrawCommand] {
        &self.draw_list
    }
    
    /// Generate a unique ID
    fn generate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    /// Check if point is in rect
    fn is_point_in_rect(&self, point: Vec2, pos: Vec2, size: Vec2) -> bool {
        point.x >= pos.x && point.x <= pos.x + size.x &&
        point.y >= pos.y && point.y <= pos.y + size.y
    }
    
    /// Advance cursor
    fn advance_cursor(&mut self, size: Vec2) {
        match self.current_layout.direction {
            LayoutDirection::Horizontal => {
                self.current_layout.cursor.x += size.x + self.current_layout.spacing;
            }
            LayoutDirection::Vertical => {
                self.current_layout.cursor.y += size.y + self.current_layout.spacing;
            }
        }
    }
    
    /// Push layout
    fn push_layout(&mut self, layout: LayoutState) {
        self.layout_stack.push(self.current_layout.clone());
        self.current_layout = layout;
    }
    
    /// Pop layout
    fn pop_layout(&mut self) {
        if let Some(layout) = self.layout_stack.pop() {
            self.current_layout = layout;
        }
    }
    
    // === WIDGETS ===
    
    /// Draw a label
    pub fn label(&mut self, text: &str) {
        let pos = self.current_layout.cursor;
        let size = Vec2::new(200.0, self.style.text_size + self.style.padding);
        
        self.draw_list.push(DrawCommand::Text {
            pos: Vec2::new(pos.x, pos.y + self.style.padding / 2.0),
            text: text.to_string(),
            color: self.style.text_color,
            size: self.style.text_size,
        });
        
        self.advance_cursor(size);
    }
    
    /// Draw a button (returns true if clicked)
    pub fn button(&mut self, label: &str) -> bool {
        let id = self.generate_id();
        let pos = self.current_layout.cursor;
        let size = Vec2::new(150.0, self.style.text_size + self.style.padding * 2.0);
        
        // Check interaction
        let is_hot = self.is_point_in_rect(self.mouse_pos, pos, size);
        let is_active = self.active_widget == Some(id);
        
        if is_hot {
            self.hot_widget = Some(id);
            if self.mouse_clicked {
                self.active_widget = Some(id);
            }
        }
        
        // Choose color
        let color = if is_active {
            self.style.button_active_color
        } else if is_hot {
            self.style.button_hover_color
        } else {
            self.style.button_color
        };
        
        // Draw button background
        self.draw_list.push(DrawCommand::Rect {
            pos,
            size,
            color,
            rounding: self.style.rounding,
        });
        
        // Draw button text
        self.draw_list.push(DrawCommand::Text {
            pos: Vec2::new(pos.x + self.style.padding, pos.y + self.style.padding),
            text: label.to_string(),
            color: self.style.text_color,
            size: self.style.text_size,
        });
        
        self.advance_cursor(size);
        
        // Return true if clicked
        is_hot && self.mouse_clicked
    }
    
    /// Draw a progress bar
    pub fn progress_bar(&mut self, fraction: f32, color: Color) {
        let pos = self.current_layout.cursor;
        let size = Vec2::new(200.0, 20.0);
        
        // Background
        self.draw_list.push(DrawCommand::Rect {
            pos,
            size,
            color: Color::new(0.2, 0.2, 0.2, 0.8),
            rounding: self.style.rounding,
        });
        
        // Filled portion
        let fill_width = size.x * fraction.max(0.0).min(1.0);
        if fill_width > 0.0 {
            self.draw_list.push(DrawCommand::Rect {
                pos,
                size: Vec2::new(fill_width, size.y),
                color,
                rounding: self.style.rounding,
            });
        }
        
        self.advance_cursor(size);
    }
    
    /// Draw a slider (returns true if value changed)
    pub fn slider(&mut self, value: &mut f32, min: f32, max: f32) -> bool {
        let id = self.generate_id();
        let pos = self.current_layout.cursor;
        let size = Vec2::new(200.0, 20.0);
        
        // Check interaction
        let is_hot = self.is_point_in_rect(self.mouse_pos, pos, size);
        let is_active = self.active_widget == Some(id);
        
        if is_hot {
            self.hot_widget = Some(id);
            if self.mouse_clicked {
                self.active_widget = Some(id);
            }
        }
        
        let mut changed = false;
        
        // Update value if active
        if is_active && self.mouse_down {
            let t = ((self.mouse_pos.x - pos.x) / size.x).max(0.0).min(1.0);
            let new_value = min + (max - min) * t;
            if (*value - new_value).abs() > 0.001 {
                *value = new_value;
                changed = true;
            }
        }
        
        // Draw track
        self.draw_list.push(DrawCommand::Rect {
            pos,
            size,
            color: Color::new(0.3, 0.3, 0.3, 0.8),
            rounding: self.style.rounding,
        });
        
        // Draw thumb
        let t = ((*value - min) / (max - min)).max(0.0).min(1.0);
        let thumb_x = pos.x + t * size.x;
        let thumb_size = 10.0;
        
        let thumb_color = if is_active {
            self.style.button_active_color
        } else if is_hot {
            self.style.button_hover_color
        } else {
            self.style.button_color
        };
        
        self.draw_list.push(DrawCommand::Rect {
            pos: Vec2::new(thumb_x - thumb_size / 2.0, pos.y - 2.0),
            size: Vec2::new(thumb_size, size.y + 4.0),
            color: thumb_color,
            rounding: self.style.rounding,
        });
        
        self.advance_cursor(size);
        
        changed
    }
    
    /// Draw a checkbox (returns true if value changed)
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        let id = self.generate_id();
        let pos = self.current_layout.cursor;
        let box_size = 20.0;
        let size = Vec2::new(200.0, box_size);
        
        // Check interaction
        let is_hot = self.is_point_in_rect(self.mouse_pos, pos, Vec2::new(box_size, box_size));
        
        let mut changed = false;
        if is_hot && self.mouse_clicked {
            *value = !*value;
            changed = true;
        }
        
        // Draw box
        let box_color = if is_hot {
            self.style.button_hover_color
        } else {
            Color::new(0.3, 0.3, 0.3, 0.8)
        };
        
        self.draw_list.push(DrawCommand::Rect {
            pos,
            size: Vec2::new(box_size, box_size),
            color: box_color,
            rounding: self.style.rounding,
        });
        
        // Draw checkmark if checked
        if *value {
            self.draw_list.push(DrawCommand::Rect {
                pos: Vec2::new(pos.x + 4.0, pos.y + 4.0),
                size: Vec2::new(box_size - 8.0, box_size - 8.0),
                color: self.style.button_color,
                rounding: self.style.rounding / 2.0,
            });
        }
        
        // Draw label
        self.draw_list.push(DrawCommand::Text {
            pos: Vec2::new(pos.x + box_size + self.style.padding, pos.y + 2.0),
            text: label.to_string(),
            color: self.style.text_color,
            size: self.style.text_size,
        });
        
        self.advance_cursor(size);
        
        changed
    }
    
    /// Begin a window
    pub fn window(&mut self, title: &str, pos: Vec2, size: Vec2) {
        // Draw window background
        self.draw_list.push(DrawCommand::Rect {
            pos,
            size,
            color: self.style.window_bg_color,
            rounding: self.style.rounding,
        });
        
        // Draw title bar
        let title_height = self.style.text_size + self.style.padding * 2.0;
        self.draw_list.push(DrawCommand::Rect {
            pos,
            size: Vec2::new(size.x, title_height),
            color: Color::new(0.2, 0.2, 0.2, 1.0),
            rounding: self.style.rounding,
        });
        
        // Draw title text
        self.draw_list.push(DrawCommand::Text {
            pos: Vec2::new(pos.x + self.style.padding, pos.y + self.style.padding),
            text: title.to_string(),
            color: self.style.text_color,
            size: self.style.text_size,
        });
        
        // Set up layout for window contents
        let content_layout = LayoutState {
            cursor: Vec2::new(pos.x + self.style.padding, pos.y + title_height + self.style.padding),
            direction: LayoutDirection::Vertical,
            size: Vec2::new(size.x - self.style.padding * 2.0, size.y - title_height - self.style.padding * 2.0),
            padding: self.style.padding,
            spacing: self.style.spacing,
        };
        
        self.push_layout(content_layout);
    }
    
    /// End the current window
    pub fn end_window(&mut self) {
        self.pop_layout();
    }
    
    /// Draw a separator line
    pub fn separator(&mut self) {
        let pos = self.current_layout.cursor;
        let size = Vec2::new(self.current_layout.size.x, 2.0);
        
        self.draw_list.push(DrawCommand::Line {
            start: pos,
            end: Vec2::new(pos.x + size.x, pos.y),
            color: Color::new(0.5, 0.5, 0.5, 0.5),
            thickness: 1.0,
        });
        
        self.advance_cursor(size);
    }
    
    /// Begin horizontal layout
    pub fn begin_horizontal(&mut self) {
        let mut layout = self.current_layout.clone();
        layout.direction = LayoutDirection::Horizontal;
        self.push_layout(layout);
    }
    
    /// End horizontal layout
    pub fn end_horizontal(&mut self) {
        self.pop_layout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ui_creation() {
        let ui = UI::new();
        assert_eq!(ui.draw_list.len(), 0);
    }
    
    #[test]
    fn test_ui_frame() {
        let mut ui = UI::new();
        ui.begin_frame(Vec2::new(0.0, 0.0), false);
        ui.label("Hello");
        ui.end_frame();
        
        assert!(ui.draw_list.len() > 0);
    }
    
    #[test]
    fn test_button() {
        let mut ui = UI::new();
        ui.begin_frame(Vec2::new(0.0, 0.0), false);
        let clicked = ui.button("Click Me");
        ui.end_frame();
        
        assert!(!clicked); // Not clicked (mouse not over button)
    }
    
    #[test]
    fn test_progress_bar() {
        let mut ui = UI::new();
        ui.begin_frame(Vec2::new(0.0, 0.0), false);
        ui.progress_bar(0.5, Color::red());
        ui.end_frame();
        
        assert!(ui.draw_list.len() >= 2); // Background + fill
    }
}

