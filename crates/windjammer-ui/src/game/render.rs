//! Rendering context for games

use super::math::{Vec2, Vec3};

/// RGBA color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

/// Sprite for 2D rendering
#[derive(Debug, Clone)]
pub struct Sprite {
    pub texture_id: String,
    pub width: f32,
    pub height: f32,
}

impl Sprite {
    pub fn new(texture_id: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            texture_id: texture_id.into(),
            width,
            height,
        }
    }
}

/// Rendering context
pub struct RenderContext {
    // Platform-specific rendering state would go here
    _reserved: (),
}

impl RenderContext {
    pub fn new() -> Self {
        Self { _reserved: () }
    }

    /// Clear the screen with a color
    pub fn clear(&self, _color: Color) {
        // Would actually clear the screen
        // In a real implementation, this would send commands to the GPU
    }

    /// Draw a sprite at a position
    pub fn draw_sprite(&self, _sprite: &Sprite, _position: Vec2) {
        // Would render sprite to screen
    }

    /// Draw text at a position
    pub fn draw_text(&self, _text: &str, _position: Vec2) {
        // Would render text to screen
    }

    /// Draw a rectangle
    pub fn draw_rect(&self, _position: Vec2, _size: Vec2, _color: Color) {
        // Would draw a filled rectangle
    }

    /// Draw a circle
    pub fn draw_circle(&self, _center: Vec2, _radius: f32, _color: Color) {
        // Would draw a filled circle
    }

    /// Draw a line
    pub fn draw_line(&self, _start: Vec2, _end: Vec2, _color: Color, _thickness: f32) {
        // Would draw a line
    }

    /// Draw a 3D mesh (for 3D games)
    pub fn draw_mesh(&self, _position: Vec3, _rotation: Vec3, _scale: Vec3) {
        // Would render 3D mesh
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::BLACK.r, 0.0);
        assert_eq!(Color::RED.r, 1.0);
        assert_eq!(Color::RED.g, 0.0);
    }

    #[test]
    fn test_color_creation() {
        let color = Color::rgb(0.5, 0.5, 0.5);
        assert_eq!(color.r, 0.5);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_sprite_creation() {
        let sprite = Sprite::new("player.png", 32.0, 32.0);
        assert_eq!(sprite.texture_id, "player.png");
        assert_eq!(sprite.width, 32.0);
    }

    #[test]
    fn test_render_context() {
        let ctx = RenderContext::new();
        ctx.clear(Color::BLUE);
        // In a real implementation, we would verify the clear command was sent
    }
}
