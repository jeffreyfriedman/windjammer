//! 2D sprite rendering

use super::backend::Vertex2D;
use crate::math::Vec2;
use crate::transform::Transform2D;

/// 2D Sprite
#[derive(Clone)]
pub struct Sprite {
    pub position: Vec2,
    pub size: Vec2,
    pub color: [f32; 4],
    pub texture_id: Option<u32>,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Sprite {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            position,
            size,
            color: [1.0, 1.0, 1.0, 1.0], // White
            texture_id: None,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_rotation_degrees(mut self, degrees: f32) -> Self {
        self.rotation = degrees.to_radians();
        self
    }

    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec2::new(scale, scale);
        self
    }

    /// Create a sprite from a Transform2D
    pub fn from_transform(transform: Transform2D, size: Vec2) -> Self {
        Self {
            position: transform.position,
            size,
            color: [1.0, 1.0, 1.0, 1.0],
            texture_id: None,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }

    /// Generate vertices for this sprite (with rotation and scale applied)
    pub fn vertices(&self) -> [Vertex2D; 4] {
        let w = self.size.x * self.scale.x;
        let h = self.size.y * self.scale.y;

        // Local space corners (centered at origin)
        let half_w = w / 2.0;
        let half_h = h / 2.0;
        let corners = [
            Vec2::new(-half_w, -half_h), // Top-left
            Vec2::new(half_w, -half_h),  // Top-right
            Vec2::new(half_w, half_h),   // Bottom-right
            Vec2::new(-half_w, half_h),  // Bottom-left
        ];

        // Apply rotation and translation
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();

        let transform_point = |p: Vec2| -> [f32; 2] {
            let rotated_x = p.x * cos - p.y * sin;
            let rotated_y = p.x * sin + p.y * cos;
            [rotated_x + self.position.x, rotated_y + self.position.y]
        };

        [
            Vertex2D {
                position: transform_point(corners[0]),
                tex_coords: [0.0, 0.0],
                color: self.color,
            },
            Vertex2D {
                position: transform_point(corners[1]),
                tex_coords: [1.0, 0.0],
                color: self.color,
            },
            Vertex2D {
                position: transform_point(corners[2]),
                tex_coords: [1.0, 1.0],
                color: self.color,
            },
            Vertex2D {
                position: transform_point(corners[3]),
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
        assert_eq!(sprite.rotation, 0.0);
        assert_eq!(sprite.scale, Vec2::ONE);
    }

    #[test]
    fn test_sprite_builder() {
        let sprite = Sprite::new(Vec2::ZERO, Vec2::new(10.0, 10.0))
            .with_color([1.0, 0.0, 0.0, 1.0])
            .with_rotation(std::f32::consts::PI / 4.0)
            .with_scale(Vec2::new(2.0, 2.0));

        assert_eq!(sprite.color, [1.0, 0.0, 0.0, 1.0]);
        assert!((sprite.rotation - std::f32::consts::PI / 4.0).abs() < 0.001);
        assert_eq!(sprite.scale, Vec2::new(2.0, 2.0));
    }

    #[test]
    fn test_sprite_from_transform() {
        let transform = Transform2D::new()
            .with_position(Vec2::new(5.0, 5.0))
            .with_rotation(std::f32::consts::PI / 2.0)
            .with_scale(Vec2::new(2.0, 2.0));

        let sprite = Sprite::from_transform(transform, Vec2::new(10.0, 10.0));
        assert_eq!(sprite.position, Vec2::new(5.0, 5.0));
        assert_eq!(sprite.scale, Vec2::new(2.0, 2.0));
    }

    #[test]
    fn test_sprite_vertices() {
        let sprite = Sprite::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let vertices = sprite.vertices();
        assert_eq!(vertices.len(), 4);

        // With no rotation, vertices should be centered around origin
        // Top-left should be at (-5, -5) + position
        assert!((vertices[0].position[0] + 5.0).abs() < 0.001);
        assert!((vertices[0].position[1] + 5.0).abs() < 0.001);
    }

    #[test]
    fn test_sprite_vertices_with_rotation() {
        let sprite = Sprite::new(Vec2::ZERO, Vec2::new(10.0, 10.0))
            .with_rotation(std::f32::consts::PI / 2.0); // 90 degrees

        let vertices = sprite.vertices();
        assert_eq!(vertices.len(), 4);

        // After 90 degree rotation, top-left becomes top-right in screen space
        // Verify rotation was applied (positions should be different from unrotated)
        let unrotated = Sprite::new(Vec2::ZERO, Vec2::new(10.0, 10.0)).vertices();
        assert!((vertices[0].position[0] - unrotated[0].position[0]).abs() > 0.1);
    }

    #[test]
    fn test_sprite_vertices_with_scale() {
        let sprite = Sprite::new(Vec2::ZERO, Vec2::new(10.0, 10.0)).with_scale(Vec2::new(2.0, 2.0));

        let vertices = sprite.vertices();

        // With 2x scale, corners should be further from center
        // Top-left should be at (-10, -10) instead of (-5, -5)
        assert!((vertices[0].position[0] + 10.0).abs() < 0.001);
        assert!((vertices[0].position[1] + 10.0).abs() < 0.001);
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

    #[test]
    fn test_sprite_batch_clear() {
        let mut batch = SpriteBatch::new();
        batch.add(Sprite::new(Vec2::ZERO, Vec2::ONE));
        batch.add(Sprite::new(Vec2::ONE, Vec2::ONE));

        assert_eq!(batch.sprites().len(), 2);

        batch.clear();
        assert_eq!(batch.sprites().len(), 0);
    }
}
