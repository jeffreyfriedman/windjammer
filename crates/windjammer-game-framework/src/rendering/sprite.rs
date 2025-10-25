//! 2D sprite rendering

use super::backend::Vertex2D;
use crate::math::Vec2;

/// 2D Sprite
#[derive(Clone)]
pub struct Sprite {
    pub position: Vec2,
    pub size: Vec2,
    pub color: [f32; 4],
    pub texture_id: Option<u32>,
}

impl Sprite {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            position,
            size,
            color: [1.0, 1.0, 1.0, 1.0], // White
            texture_id: None,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Generate vertices for this sprite
    pub fn vertices(&self) -> [Vertex2D; 4] {
        let x = self.position.x;
        let y = self.position.y;
        let w = self.size.x;
        let h = self.size.y;

        [
            // Top-left
            Vertex2D {
                position: [x, y],
                tex_coords: [0.0, 0.0],
                color: self.color,
            },
            // Top-right
            Vertex2D {
                position: [x + w, y],
                tex_coords: [1.0, 0.0],
                color: self.color,
            },
            // Bottom-right
            Vertex2D {
                position: [x + w, y + h],
                tex_coords: [1.0, 1.0],
                color: self.color,
            },
            // Bottom-left
            Vertex2D {
                position: [x, y + h],
                tex_coords: [0.0, 1.0],
                color: self.color,
            },
        ]
    }

    /// Generate indices for this sprite (two triangles)
    pub fn indices() -> [u16; 6] {
        [0, 1, 2, 0, 2, 3]
    }
}

/// Batch of sprites for efficient rendering
pub struct SpriteBatch {
    sprites: Vec<Sprite>,
}

impl SpriteBatch {
    pub fn new() -> Self {
        Self {
            sprites: Vec::new(),
        }
    }

    pub fn add(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    pub fn clear(&mut self) {
        self.sprites.clear();
    }

    pub fn sprites(&self) -> &[Sprite] {
        &self.sprites
    }

    /// Generate all vertices for the batch
    pub fn vertices(&self) -> Vec<Vertex2D> {
        self.sprites
            .iter()
            .flat_map(|sprite| sprite.vertices())
            .collect()
    }

    /// Generate all indices for the batch
    pub fn indices(&self) -> Vec<u16> {
        self.sprites
            .iter()
            .enumerate()
            .flat_map(|(i, _)| {
                let offset = (i * 4) as u16;
                Sprite::indices().map(|idx| idx + offset)
            })
            .collect()
    }
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_creation() {
        let sprite = Sprite::new(Vec2::new(10.0, 20.0), Vec2::new(32.0, 32.0));
        assert_eq!(sprite.position.x, 10.0);
        assert_eq!(sprite.size.x, 32.0);
    }

    #[test]
    fn test_sprite_vertices() {
        let sprite = Sprite::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let vertices = sprite.vertices();
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0].position, [0.0, 0.0]);
        assert_eq!(vertices[2].position, [10.0, 10.0]);
    }

    #[test]
    fn test_sprite_batch() {
        let mut batch = SpriteBatch::new();
        batch.add(Sprite::new(Vec2::ZERO, Vec2::ONE));
        batch.add(Sprite::new(Vec2::new(10.0, 10.0), Vec2::ONE));

        assert_eq!(batch.sprites().len(), 2);
        assert_eq!(batch.vertices().len(), 8); // 4 vertices per sprite
        assert_eq!(batch.indices().len(), 12); // 6 indices per sprite
    }
}
