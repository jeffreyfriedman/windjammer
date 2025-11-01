// Game rendering system using wgpu

use super::{Color, Sprite, Transform};

/// Simple 2D renderer for games
pub struct Renderer {
    width: u32,
    height: u32,
    title: String,
    // TODO: Add wgpu state when implementing full rendering
}

impl Renderer {
    pub fn new(width: u32, height: u32, title: &str) -> Self {
        Renderer {
            width,
            height,
            title: title.to_string(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        // TODO: Implement with wgpu
        println!("Clear screen with color: {:?}", color);
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite, transform: &Transform) {
        // TODO: Implement with wgpu
        println!(
            "Draw sprite '{}' at ({}, {}) size: {}x{}",
            sprite.texture_path,
            transform.position.x,
            transform.position.y,
            sprite.width,
            sprite.height
        );
    }

    pub fn present(&mut self) {
        // TODO: Implement with wgpu
        // For now, just indicate frame complete
    }
}


